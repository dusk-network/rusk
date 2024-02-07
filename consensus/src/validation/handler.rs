// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::aggregator::Aggregator;
use crate::commons::{ConsensusError, RoundUpdate};
use crate::msg_handler::{HandleMsgOutput, MsgHandler};
use crate::step_votes_reg::SafeCertificateInfoRegistry;
use async_trait::async_trait;
use node_data::ledger::{Block, StepVotes};
use node_data::StepName;
use tracing::{info, warn};

use crate::user::committee::Committee;

use crate::iteration_ctx::RoundCommittees;
use node_data::message::payload::{QuorumType, Validation, Vote};
use node_data::message::{payload, Message, Payload, StepMessage};

fn final_result(
    sv: StepVotes,
    vote: Vote,
    quorum: QuorumType,
) -> HandleMsgOutput {
    let msg = Message::from_validation_result(payload::ValidationResult {
        sv,
        vote,
        quorum,
    });

    HandleMsgOutput::Ready(msg)
}

pub struct ValidationHandler {
    pub(crate) aggr: Aggregator,
    pub(crate) candidate: Option<Block>,
    sv_registry: SafeCertificateInfoRegistry,
    curr_iteration: u8,
}

impl ValidationHandler {
    pub(crate) fn new(sv_registry: SafeCertificateInfoRegistry) -> Self {
        Self {
            sv_registry,
            aggr: Aggregator::default(),
            candidate: None,
            curr_iteration: 0,
        }
    }

    pub(crate) fn reset(&mut self, curr_iteration: u8) {
        self.candidate = None;
        self.curr_iteration = curr_iteration;
    }

    fn unwrap_msg(msg: Message) -> Result<Validation, ConsensusError> {
        match msg.payload {
            Payload::Validation(r) => Ok(r),
            _ => Err(ConsensusError::InvalidMsgType),
        }
    }
}

#[async_trait]
impl MsgHandler for ValidationHandler {
    /// Verifies if a msg is a valid validation message.
    fn verify(
        &self,
        msg: &Message,
        _ru: &RoundUpdate,
        _iteration: u8,
        _committee: &Committee,
        _round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError> {
        match &msg.payload {
            Payload::Validation(p) => p.verify_signature()?,
            Payload::Empty => (),
            _ => Err(ConsensusError::InvalidMsgType)?,
        };

        Ok(())
    }

    /// Collects the validation message.
    async fn collect(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        committee: &Committee,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        let p = Self::unwrap_msg(msg)?;

        // NoQuorum cannot be cast from validation committee
        if p.vote == Vote::NoQuorum {
            return Err(ConsensusError::InvalidVote(p.vote));
        }

        let iteration = p.header().iteration;
        if iteration != self.curr_iteration {
            // Message that belongs to step from the past must be handled with
            // collect_from_past fn
            warn!(
                event = "drop message",
                reason = "invalid iteration number",
                msg_iteration = iteration,
            );
            return Ok(HandleMsgOutput::Pending);
        }

        let collect_vote = self.aggr.collect_vote(
            committee,
            p.sign_info(),
            &p.vote,
            p.get_step(),
        );
        match collect_vote {
            Ok((sv, quorum_reached)) => {
                // Record result in global round registry
                _ = self.sv_registry.lock().await.add_step_votes(
                    iteration,
                    &p.vote,
                    sv,
                    StepName::Validation,
                    quorum_reached,
                    committee.excluded().expect("Generator to be excluded"),
                );

                if quorum_reached {
                    let vote = p.vote;

                    let quorum_type = match vote {
                        Vote::NoCandidate => QuorumType::NoCandidate,
                        Vote::Invalid(_) => QuorumType::Invalid,
                        Vote::Valid(_) => QuorumType::Valid,
                        Vote::NoQuorum => {
                            return Err(ConsensusError::InvalidVote(vote));
                        }
                    };
                    info!(event = "quorum reached", %vote);
                    return Ok(final_result(sv, vote, quorum_type));
                }
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
        }

        Ok(HandleMsgOutput::Pending)
    }

    /// Collects the validation message from former iteration.
    async fn collect_from_past(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        committee: &Committee,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        let p = Self::unwrap_msg(msg)?;

        // NoQuorum cannot be cast from validation committee
        if p.vote == Vote::NoQuorum {
            return Err(ConsensusError::InvalidVote(p.vote));
        }

        // Collect vote, if msg payload is validation type
        let collect_vote = self.aggr.collect_vote(
            committee,
            p.sign_info(),
            &p.vote,
            p.get_step(),
        );

        match collect_vote {
            Ok((sv, quorum_reached)) => {
                if let Some(quorum_msg) =
                    self.sv_registry.lock().await.add_step_votes(
                        p.header().iteration,
                        &p.vote,
                        sv,
                        StepName::Validation,
                        quorum_reached,
                        committee.excluded().expect("Generator to be excluded"),
                    )
                {
                    return Ok(HandleMsgOutput::Ready(quorum_msg));
                }
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
        }
        Ok(HandleMsgOutput::Pending)
    }

    /// Handles of an event of step execution timeout
    fn handle_timeout(&self) -> Result<HandleMsgOutput, ConsensusError> {
        Ok(final_result(
            StepVotes::default(),
            Vote::NoCandidate,
            QuorumType::NoQuorum,
        ))
    }
}
