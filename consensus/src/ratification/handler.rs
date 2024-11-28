// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use async_trait::async_trait;
use node_data::bls::PublicKeyBytes;
use node_data::ledger::Attestation;
use node_data::message::payload::{Ratification, ValidationResult, Vote};
use node_data::message::{
    payload, ConsensusHeader, Message, Payload, SignedStepMessage, StepMessage,
};
use node_data::{ledger, StepName};
use tracing::{debug, error, info, warn};

use crate::aggregator::{Aggregator, StepVote};
use crate::commons::RoundUpdate;
use crate::config::is_emergency_iter;
use crate::errors::ConsensusError;
use crate::iteration_ctx::RoundCommittees;
use crate::msg_handler::{MsgHandler, StepOutcome};
use crate::quorum::verifiers::verify_votes;
use crate::step_votes_reg::SafeAttestationInfoRegistry;
use crate::user::committee::Committee;

pub struct RatificationHandler {
    pub(crate) sv_registry: SafeAttestationInfoRegistry,

    pub(crate) aggregator: Aggregator<Ratification>,
    validation_result: ValidationResult,
    pub(crate) curr_iteration: u8,
}

// Implement the required trait to use Aggregator
impl StepVote for Ratification {
    fn vote(&self) -> &Vote {
        &self.vote
    }
}

impl RatificationHandler {
    pub fn verify_stateless(
        msg: &Message,
        round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError> {
        match &msg.payload {
            Payload::Ratification(p) => {
                p.verify_signature()?;
                let signer = &p.sign_info.signer;
                let committee = round_committees
                    .get_committee(msg.get_step())
                    .expect("committee to be created before run");

                committee
                    .votes_for(signer)
                    .ok_or(ConsensusError::NotCommitteeMember)?;
            }

            Payload::ValidationQuorum(q) => Self::verify_validation_result(
                &q.header,
                &q.result,
                round_committees,
            )?,
            Payload::Empty => (),
            _ => {
                info!("cannot verify in validation handler");
                Err(ConsensusError::InvalidMsgType)?
            }
        }

        Ok(())
    }
}

#[async_trait]
impl MsgHandler for RatificationHandler {
    fn verify(
        &self,
        msg: &Message,
        _round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError> {
        if let Payload::Ratification(p) = &msg.payload {
            if self.aggregator.is_vote_collected(p) {
                return Err(ConsensusError::VoteAlreadyCollected);
            }

            p.verify_signature()?;

            return Ok(());
        }

        Err(ConsensusError::InvalidMsgType)
    }

    /// Collect the Ratification message.
    async fn collect(
        &mut self,
        msg: Message,
        ru: &RoundUpdate,
        committee: &Committee,
        generator: Option<PublicKeyBytes>,
        round_committees: &RoundCommittees,
    ) -> Result<StepOutcome, ConsensusError> {
        let p = Self::unwrap_msg(msg)?;
        let vote = p.vote;
        let iteration = p.header().iteration;

        if iteration != self.curr_iteration {
            // Message that belongs to step from the past must be handled with
            // collect_from_past fn
            return Err(ConsensusError::InvalidMsgIteration(iteration));
        }

        // Ensure the vote matches the msg ValidationResult
        let vr_vote = *p.validation_result.vote();
        if vr_vote != vote {
            warn!(
                event = "Vote discarded",
                step = "Ratification",
                reason = "mismatch with msg ValidationResult",
                round = ru.round,
                iter = iteration
            );

            return Err(ConsensusError::VoteMismatch(vr_vote, vote));
        }

        // If the vote is a Quorum, check it against our result
        // (NoQuorum votes need no verification)
        if vote != Vote::NoQuorum {
            // If our result is NoQuorum, verify votes and update our result
            let local_vote = *self.validation_result().vote();
            match local_vote {
                Vote::NoQuorum => {
                    // If our result is NoQuorum, verify votes and
                    // then update our result
                    Self::verify_validation_result(
                        &p.header,
                        &p.validation_result,
                        round_committees,
                    )?;

                    self.update_validation_result(p.validation_result.clone())
                }

                _ => {
                    // If our result is also a Quorum, check they match and skip
                    // verification.
                    if vote != local_vote {
                        if !is_emergency_iter(iteration) {
                            warn!(
                                event = "Vote discarded",
                                step = "Ratification",
                                reason = "mismatch with local ValidationResult",
                                round = ru.round,
                                iter = iteration
                            );

                            return Err(ConsensusError::VoteMismatch(
                                local_vote, vote,
                            ));
                        } else {
                            // In Emergency Mode, we do not discard votes
                            // because multiple votes are allowed
                            Self::verify_validation_result(
                                &p.header,
                                &p.validation_result,
                                round_committees,
                            )?;

                            // We update our result to be sure to build a
                            // coherent Quorum message
                            self.update_validation_result(
                                p.validation_result.clone(),
                            )
                        }
                    }
                }
            }
        }

        // Collect vote
        let (ratification_sv, quorum_reached) = self
            .aggregator
            .collect_vote(committee, &p)
            .map_err(|error| {
                warn!(
                    event = "Cannot collect vote",
                    ?error,
                    from = p.sign_info().signer.to_bs58(),
                    ?vote,
                    msg_step = p.get_step(),
                    msg_iter = p.header().iteration,
                    msg_height = p.header().round,
                );
                ConsensusError::InvalidVote(vote)
            })?;

        // Record any signature in global registry
        let _ = self.sv_registry.lock().await.set_step_votes(
            iteration,
            &vote,
            ratification_sv,
            StepName::Ratification,
            quorum_reached,
            &generator.expect("There must be a valid generator"),
        );

        if quorum_reached {
            // INFO: we set Validation SV to our local result to always include
            // a quorum-reaching (if any) SV even if the Ratification result is
            // NoQuorum
            let validation_sv = *self.validation_result.sv();

            return Ok(StepOutcome::Ready(self.build_quorum_msg(
                ru,
                iteration,
                vote,
                validation_sv,
                ratification_sv,
            )));
        }

        Ok(StepOutcome::Pending)
    }

    /// Collects the ratification message from former iteration.
    async fn collect_from_past(
        &mut self,
        msg: Message,
        committee: &Committee,
        generator: Option<PublicKeyBytes>,
    ) -> Result<StepOutcome, ConsensusError> {
        let p = Self::unwrap_msg(msg)?;

        // Collect vote, if msg payload is ratification type
        let collect_vote = self.aggregator.collect_vote(committee, &p);

        match collect_vote {
            Ok((sv, quorum_reached)) => {
                // Record any signature in global registry
                if let Some(quorum_msg) =
                    self.sv_registry.lock().await.set_step_votes(
                        p.header().iteration,
                        &p.vote,
                        sv,
                        StepName::Ratification,
                        quorum_reached,
                        &generator.expect("There must be a valid generator"),
                    )
                {
                    return Ok(StepOutcome::Ready(quorum_msg));
                }
            }
            Err(error) => {
                warn!(
                    event = "Cannot collect vote",
                    ?error,
                    from = p.sign_info().signer.to_bs58(),
                    vote = ?p.vote,
                    msg_step = p.get_step(),
                    msg_iter = p.header().iteration,
                    msg_height = p.header().round,
                );
            }
        };

        Ok(StepOutcome::Pending)
    }

    /// Handle of an event of step execution timeout
    fn handle_timeout(
        &self,
        _ru: &RoundUpdate,
        _curr_iteration: u8,
    ) -> Option<Message> {
        None
    }
}

impl RatificationHandler {
    pub(crate) fn new(sv_registry: SafeAttestationInfoRegistry) -> Self {
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
            att: Attestation {
                result: vote.into(),
                validation,
                ratification,
            },
        };

        quorum.into()
    }

    pub(crate) fn reset(&mut self, iter: u8, validation: ValidationResult) {
        self.validation_result = validation;
        self.curr_iteration = iter;
    }

    pub(crate) fn validation_result(&self) -> &ValidationResult {
        &self.validation_result
    }

    pub(crate) fn update_validation_result(&mut self, vr: ValidationResult) {
        let cur_vote = *self.validation_result().vote();
        let new_vote = vr.vote();
        debug!(
            "Update local ValidationResult ({:?}) with {:?}",
            cur_vote, new_vote
        );

        self.validation_result = vr;
    }

    fn unwrap_msg(msg: Message) -> Result<Ratification, ConsensusError> {
        match msg.payload {
            Payload::Ratification(r) => Ok(r),
            _ => Err(ConsensusError::InvalidMsgType),
        }
    }

    /// Verifies either valid or nil quorum of validation output
    pub(crate) fn verify_validation_result(
        header: &ConsensusHeader,
        result: &ValidationResult,
        round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError> {
        let iter = header.iteration;
        let validation_committee = round_committees
            .get_validation_committee(iter)
            .ok_or_else(|| {
                error!("could not get validation committee");
                ConsensusError::CommitteeNotGenerated
            })?;
        verify_votes(
            header,
            StepName::Validation,
            result.vote(),
            result.sv(),
            validation_committee,
        )?;
        Ok(())
    }
}
