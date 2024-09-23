// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::aggregator::{Aggregator, StepVote};
use crate::commons::RoundUpdate;
use crate::config::is_emergency_iter;
use crate::errors::ConsensusError;
use crate::msg_handler::{HandleMsgOutput, MsgHandler};
use crate::step_votes_reg::SafeAttestationInfoRegistry;
use async_trait::async_trait;
use node_data::bls::PublicKeyBytes;
use node_data::ledger::{Block, StepVotes};
use node_data::StepName;
use tracing::{info, warn};

use crate::user::committee::Committee;

use crate::iteration_ctx::RoundCommittees;
use node_data::message::payload::{QuorumType, Validation, Vote};
use node_data::message::{
    payload, Message, Payload, SignedStepMessage, StepMessage,
};

fn final_result(
    sv: StepVotes,
    vote: Vote,
    quorum: QuorumType,
) -> HandleMsgOutput {
    let p = payload::ValidationResult::new(sv, vote, quorum);
    let msg = Message::from(p);

    HandleMsgOutput::Ready(msg)
}

pub struct ValidationHandler {
    pub(crate) aggr: Aggregator<Validation>,
    pub(crate) candidate: Option<Block>,
    sv_registry: SafeAttestationInfoRegistry,
    curr_iteration: u8,
}

// Implement the required trait to use Aggregator
impl StepVote for Validation {
    fn vote(&self) -> &Vote {
        &self.vote
    }
}

impl ValidationHandler {
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
}

impl ValidationHandler {
    pub(crate) fn new(sv_registry: SafeAttestationInfoRegistry) -> Self {
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
            return Ok(final_result(sv, vote, quorum_type));
        }

        Ok(HandleMsgOutput::Pending)
    }

    /// Collects the validation message from former iteration.
    async fn collect_from_past(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        committee: &Committee,
        generator: Option<PublicKeyBytes>,
    ) -> Result<HandleMsgOutput, ConsensusError> {
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
                    return Ok(final_result(sv, p.vote, QuorumType::Valid));
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
        Ok(HandleMsgOutput::Pending)
    }

    /// Handles of an event of step execution timeout
    fn handle_timeout(
        &self,
        _ru: &RoundUpdate,
        curr_iteration: u8,
    ) -> Option<Message> {
        if is_emergency_iter(curr_iteration) {
            // While we are in Emergency mode but still the candidate is missing
            // then we request it

            // TODO: Request ValidationResult by prev_block_hash, iteration
            // lockup key TODO: Should we also request the candidate
            // block, if it's still missing?
        }

        None
    }
}
