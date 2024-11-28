// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::Arc;

use async_trait::async_trait;
use node_data::bls::PublicKeyBytes;
use node_data::ledger::{to_str, Block, StepVotes};
use node_data::message::payload::{
    GetResource, Inv, QuorumType, Validation, Vote,
};
use node_data::message::{
    payload, ConsensusHeader, Message, Payload, SignedStepMessage, StepMessage,
};
use node_data::StepName;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::aggregator::{Aggregator, StepVote};
use crate::commons::{Database, RoundUpdate};
use crate::config::is_emergency_iter;
use crate::errors::ConsensusError;
use crate::iteration_ctx::RoundCommittees;
use crate::msg_handler::{MsgHandler, StepOutcome};
use crate::step_votes_reg::SafeAttestationInfoRegistry;
use crate::user::committee::Committee;

pub struct ValidationHandler<D: Database> {
    pub(crate) aggr: Aggregator<Validation>,
    pub(crate) candidate: Option<Block>,
    sv_registry: SafeAttestationInfoRegistry,
    curr_iteration: u8,
    pub(crate) db: Arc<Mutex<D>>,
}

// Implement the required trait to use Aggregator
impl StepVote for Validation {
    fn vote(&self) -> &Vote {
        &self.vote
    }
}

pub fn verify_stateless(
    msg: &Message,
    round_committees: &RoundCommittees,
) -> Result<(), ConsensusError> {
    match &msg.payload {
        Payload::Validation(p) => {
            p.verify_signature()?;

            let signer = &p.sign_info.signer;
            let committee = round_committees
                .get_committee(msg.get_step())
                .expect("committee to be created before run");

            committee
                .votes_for(signer)
                .ok_or(ConsensusError::NotCommitteeMember)?;
        }
        Payload::Empty => (),
        _ => {
            info!("cannot verify in validation handler");
            Err(ConsensusError::InvalidMsgType)?
        }
    }

    Ok(())
}

impl<D: Database> ValidationHandler<D> {
    pub(crate) fn new(
        sv_registry: SafeAttestationInfoRegistry,
        db: Arc<Mutex<D>>,
    ) -> Self {
        Self {
            sv_registry,
            aggr: Aggregator::default(),
            candidate: None,
            curr_iteration: 0,
            db,
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

    async fn build_validation_result(
        &self,
        sv: StepVotes,
        vote: Vote,
        quorum: QuorumType,
        consensus_header: &ConsensusHeader,
    ) -> Message {
        let vr = payload::ValidationResult::new(sv, vote, quorum);

        // In Emergency Mode, we store ValidationResult in case some peer
        // requests it
        if is_emergency_iter(consensus_header.iteration) {
            debug!(
              event = "Store ValidationResult",
              info = ?consensus_header,
              src = "Validation"
            );

            self.db
                .lock()
                .await
                .store_validation_result(consensus_header, &vr)
                .await;
        }

        Message::from(vr)
    }
}

#[async_trait]
impl<D: Database> MsgHandler for ValidationHandler<D> {
    /// Verifies if a msg is a valid validation message.
    fn verify(
        &self,
        msg: &Message,
        _round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError> {
        match &msg.payload {
            Payload::Validation(p) => {
                if self.aggr.is_vote_collected(p) {
                    return Err(ConsensusError::VoteAlreadyCollected);
                }

                p.verify_signature()?
            }
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
        generator: Option<PublicKeyBytes>,
        _round_committees: &RoundCommittees,
    ) -> Result<StepOutcome, ConsensusError> {
        let p = Self::unwrap_msg(msg)?;

        // NoQuorum cannot be cast from validation committee
        if p.vote == Vote::NoQuorum {
            return Err(ConsensusError::InvalidVote(p.vote));
        }

        let iteration = p.header().iteration;
        if iteration != self.curr_iteration {
            // Message that belongs to step from the past must be handled with
            // collect_from_past fn
            return Err(ConsensusError::InvalidMsgIteration(iteration));
        }

        let (sv, quorum_reached) =
            self.aggr.collect_vote(committee, &p).map_err(|error| {
                warn!(
                    event = "Cannot collect vote",
                    ?error,
                    from = p.sign_info().signer.to_bs58(),
                    vote = ?p.vote,
                    msg_step = p.get_step(),
                    msg_iter = p.header().iteration,
                    msg_height = p.header().round,
                );
                ConsensusError::InvalidVote(p.vote)
            })?;
        // Record result in global round registry
        _ = self.sv_registry.lock().await.set_step_votes(
            iteration,
            &p.vote,
            sv,
            StepName::Validation,
            quorum_reached,
            &generator.expect("There must be a valid generator"),
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
            info!(event = "quorum reached", ?vote);

            let vrmsg = self
                .build_validation_result(sv, vote, quorum_type, &p.header())
                .await;

            return Ok(StepOutcome::Ready(vrmsg));
        }

        Ok(StepOutcome::Pending)
    }

    /// Collects the validation message from former iteration.
    async fn collect_from_past(
        &mut self,
        msg: Message,
        committee: &Committee,
        generator: Option<PublicKeyBytes>,
    ) -> Result<StepOutcome, ConsensusError> {
        if is_emergency_iter(msg.header.iteration) {
            if let Payload::ValidationQuorum(vq) = msg.payload {
                if !vq.result.vote().is_valid() {
                    return Err(ConsensusError::InvalidMsgType);
                };

                let vr = vq.result;

                // Store ValidationResult
                debug!(
                  event = "Store ValidationResult",
                  info = ?vq.header,
                  src = "ValidationQuorum"
                );

                self.db
                    .lock()
                    .await
                    .store_validation_result(&vq.header, &vr)
                    .await;

                // Extract the ValidationResult and return it as msg
                let vr_msg = vr.into();
                return Ok(StepOutcome::Ready(vr_msg));
            }
        }

        let p = Self::unwrap_msg(msg)?;

        // NoQuorum cannot be cast from validation committee
        if p.vote == Vote::NoQuorum {
            return Err(ConsensusError::InvalidVote(p.vote));
        }

        // Collect vote, if msg payload is validation type
        let collect_vote = self.aggr.collect_vote(committee, &p);

        match collect_vote {
            Ok((sv, validation_quorum_reached)) => {
                // We ignore the result since it's not possible to have a full
                // quorum in the validation phase
                let _ = self.sv_registry.lock().await.set_step_votes(
                    p.header().iteration,
                    &p.vote,
                    sv,
                    StepName::Validation,
                    validation_quorum_reached,
                    &generator.expect("There must be a valid generator"),
                );

                if p.vote.is_valid() && validation_quorum_reached {
                    // ValidationResult from past iteration is found
                    let vr = self
                        .build_validation_result(
                            sv,
                            p.vote,
                            QuorumType::Valid,
                            &p.header(),
                        )
                        .await;

                    return Ok(StepOutcome::Ready(vr));
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
        }
        Ok(StepOutcome::Pending)
    }

    /// Handles of an event of step execution timeout
    fn handle_timeout(
        &self,
        ru: &RoundUpdate,
        curr_iteration: u8,
    ) -> Option<Message> {
        if is_emergency_iter(curr_iteration) {
            // In Emergency Mode we request the ValidationResult from our peers
            // in case we arrived late and missed the votes

            let prev_block_hash = ru.hash();
            let round = ru.round;

            debug!(
                event = "Request ValidationResult",
                round,
                iteration = curr_iteration,
                prev_block = to_str(&prev_block_hash)
            );

            let mut inv = Inv::new(1);
            inv.add_validation_result(ConsensusHeader {
                prev_block_hash,
                round,
                iteration: curr_iteration,
            });
            let msg = GetResource::new(inv, None, u64::MAX, 0);
            return Some(msg.into());
        }

        None
    }
}
