// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, RoundUpdate};
use crate::msg_handler::{HandleMsgOutput, MsgHandler};
use crate::step_votes_reg::SafeCertificateInfoRegistry;
use async_trait::async_trait;
use node_data::{ledger, StepName};
use tracing::{error, warn};

use crate::aggregator::Aggregator;

use crate::iteration_ctx::RoundCommittees;
use crate::quorum::verifiers::verify_votes;
use node_data::message::payload::{
    QuorumType, Ratification, ValidationResult, Vote,
};
use node_data::message::{
    payload, ConsensusHeader, Message, Payload, StepMessage,
};

use crate::user::committee::Committee;

pub struct RatificationHandler {
    pub(crate) sv_registry: SafeCertificateInfoRegistry,

    pub(crate) aggregator: Aggregator,
    validation_result: ValidationResult,
    pub(crate) curr_iteration: u8,
}

#[async_trait]
impl MsgHandler for RatificationHandler {
    fn verify(
        &self,
        msg: &Message,
        iteration: u8,
        round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError> {
        if let Payload::Ratification(p) = &msg.payload {
            p.verify_signature()?;
            Self::verify_validation_result(
                &msg.header,
                iteration,
                round_committees,
                &p.validation_result,
            )?;

            return Ok(());
        }

        Err(ConsensusError::InvalidMsgType)
    }

    /// Collect the ratification message.
    async fn collect(
        &mut self,
        msg: Message,
        ru: &RoundUpdate,
        committee: &Committee,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        let p = Self::unwrap_msg(msg)?;
        let iteration = p.header().iteration;

        if iteration != self.curr_iteration {
            // Message that belongs to step from the past must be handled with
            // collect_from_past fn
            return Err(ConsensusError::InvalidMsgIteration(iteration));
        }

        // Collect vote, if msg payload is of ratification type
        let (sv, quorum_reached) = self
            .aggregator
            .collect_vote(committee, p.sign_info(), &p.vote, p.get_step())
            .map_err(|error| {
                let vote = p.vote.clone();
                warn!(
                    event = "Cannot collect vote",
                    ?error,
                    from = p.sign_info().signer.to_bs58(),
                    %vote,
                    msg_step = p.get_step(),
                    msg_round = p.header().round,
                );
                ConsensusError::InvalidVote(vote)
            })?;

        // Record any signature in global registry
        _ = self.sv_registry.lock().await.add_step_votes(
            iteration,
            &p.vote,
            sv,
            StepName::Ratification,
            quorum_reached,
            committee.excluded().expect("Generator to be excluded"),
        );

        if quorum_reached {
            return Ok(HandleMsgOutput::Ready(self.build_quorum_msg(
                ru,
                iteration,
                p.vote,
                *p.validation_result.sv(),
                sv,
            )));
        }

        Ok(HandleMsgOutput::Pending)
    }

    /// Collects the ratification message from former iteration.
    async fn collect_from_past(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        committee: &Committee,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        let p = Self::unwrap_msg(msg)?;

        // Collect vote, if msg payload is ratification type
        let collect_vote = self.aggregator.collect_vote(
            committee,
            p.sign_info(),
            &p.vote,
            p.get_step(),
        );

        match collect_vote {
            Ok((sv, quorum_reached)) => {
                // Record any signature in global registry
                if let Some(quorum_msg) =
                    self.sv_registry.lock().await.add_step_votes(
                        p.header().iteration,
                        &p.vote,
                        sv,
                        StepName::Ratification,
                        quorum_reached,
                        committee.excluded().expect("Generator to be excluded"),
                    )
                {
                    return Ok(HandleMsgOutput::Ready(quorum_msg));
                };
            }
            Err(error) => {
                warn!(
                    event = "Cannot collect vote",
                    ?error,
                    from = p.sign_info().signer.to_bs58(),
                    vote = %p.vote,
                    msg_step = p.get_step(),
                    msg_round = p.header().round,
                );
            }
        };

        Ok(HandleMsgOutput::Pending)
    }

    /// Handle of an event of step execution timeout
    fn handle_timeout(&self) -> Result<HandleMsgOutput, ConsensusError> {
        Ok(HandleMsgOutput::Ready(Message::empty()))
    }
}

impl RatificationHandler {
    pub(crate) fn new(sv_registry: SafeCertificateInfoRegistry) -> Self {
        Self {
            sv_registry,
            aggregator: Default::default(),
            validation_result: Default::default(),
            curr_iteration: 0,
        }
    }

    fn build_quorum_msg(
        &self,
        ru: &RoundUpdate,
        iteration: u8,
        vote: Vote,
        validation: ledger::StepVotes,
        ratification: ledger::StepVotes,
    ) -> Message {
        let header = node_data::message::ConsensusHeader {
            prev_block_hash: ru.hash(),
            round: ru.round,
            iteration,
        };

        let quorum = payload::Quorum {
            header,
            result: vote.into(),
            validation,
            ratification,
        };

        Message::new_quorum(quorum)
    }

    pub(crate) fn reset(&mut self, iter: u8, validation: ValidationResult) {
        self.validation_result = validation;
        self.curr_iteration = iter;
    }

    pub(crate) fn validation_result(&self) -> &ValidationResult {
        &self.validation_result
    }

    fn unwrap_msg(msg: Message) -> Result<Ratification, ConsensusError> {
        match msg.payload {
            Payload::Ratification(r) => Ok(r),
            _ => Err(ConsensusError::InvalidMsgType),
        }
    }

    /// Verifies either valid or nil quorum of validation output
    fn verify_validation_result(
        header: &ConsensusHeader,
        iter: u8,
        round_committees: &RoundCommittees,
        result: &ValidationResult,
    ) -> Result<(), ConsensusError> {
        // TODO: Check all quorums
        match result.quorum() {
            QuorumType::Valid | QuorumType::NoCandidate => {
                if let Some(validation_committee) =
                    round_committees.get_validation_committee(iter)
                {
                    verify_votes(
                        header,
                        StepName::Validation,
                        result.vote(),
                        result.sv(),
                        validation_committee,
                    )?;

                    return Ok(());
                } else {
                    error!("could not get validation committee");
                }
            }
            _ => {}
        }
        Err(ConsensusError::InvalidValidation(result.quorum()))
    }
}
