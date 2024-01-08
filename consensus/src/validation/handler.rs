// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::aggregator::Aggregator;
use crate::commons::{ConsensusError, RoundUpdate};
use crate::msg_handler::{HandleMsgOutput, MsgHandler};
use crate::step_votes_reg::{SafeCertificateInfoRegistry, SvType};
use async_trait::async_trait;
use node_data::ledger::{Block, StepVotes};
use tracing::warn;

use crate::user::committee::Committee;

use crate::iteration_ctx::RoundCommittees;
use node_data::message::payload::QuorumType;
use node_data::message::{payload, Message, Payload};

const EMPTY_SIGNATURE: [u8; 48] = [0u8; 48];

fn final_result(
    sv: StepVotes,
    hash: [u8; 32],
    quorum: QuorumType,
) -> HandleMsgOutput {
    let msg = Message::from_validation_result(payload::ValidationResult {
        sv,
        hash,
        quorum,
    });

    HandleMsgOutput::Ready(msg)
}

pub struct ValidationHandler {
    pub(crate) aggr: Aggregator,
    pub(crate) candidate: Block,
    sv_registry: SafeCertificateInfoRegistry,
    curr_iteration: u8,
}

impl ValidationHandler {
    pub(crate) fn new(sv_registry: SafeCertificateInfoRegistry) -> Self {
        Self {
            sv_registry,
            aggr: Aggregator::default(),
            candidate: Block::default(),
            curr_iteration: 0,
        }
    }

    pub(crate) fn reset(&mut self, curr_iteration: u8) {
        self.candidate = Block::default();
        self.curr_iteration = curr_iteration;
    }
}

#[async_trait]
impl MsgHandler<Message> for ValidationHandler {
    /// Verifies if a msg is a valid reduction message.
    fn verify(
        &self,
        msg: &Message,
        _ru: &RoundUpdate,
        _iteration: u8,
        _committee: &Committee,
        _round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError> {
        let signed_hash = match &msg.payload {
            Payload::Validation(p) => Ok(p.signature),
            Payload::Empty => Ok(EMPTY_SIGNATURE),
            _ => Err(ConsensusError::InvalidMsgType),
        }?;

        if msg.header.verify_signature(&signed_hash).is_err() {
            return Err(ConsensusError::InvalidSignature);
        }

        Ok(())
    }

    /// Collects the reduction message.
    async fn collect(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        committee: &Committee,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        let iteration = msg.header.iteration;
        if iteration != self.curr_iteration {
            // Message that belongs to step from the past must be handled with
            // collect_from_past fn
            warn!(
                event = "drop message",
                reason = "invalid iteration number",
                msg_iteration = iteration,
            );
            return Ok(HandleMsgOutput::Pending(msg));
        }

        let signature = match &msg.payload {
            Payload::Validation(p) => Ok(p.signature),
            _ => Err(ConsensusError::InvalidMsgType),
        }?;

        // Collect vote, if msg payload is reduction type
        if let Some((hash, sv, quorum_reached)) =
            self.aggr.collect_vote(committee, &msg.header, &signature)
        {
            // Record result in global round registry
            _ = self.sv_registry.lock().await.add_step_votes(
                iteration,
                hash,
                sv,
                SvType::Validation,
                quorum_reached,
            );

            if quorum_reached {
                // if the votes converged for an empty hash we invoke halt
                if hash == [0u8; 32] {
                    tracing::warn!(
                        "votes converged for an empty hash (timeout)"
                    );
                    return Ok(final_result(sv, hash, QuorumType::NilQuorum));
                }

                return Ok(final_result(sv, hash, QuorumType::ValidQuorum));
            }
        }

        Ok(HandleMsgOutput::Pending(msg))
    }

    /// Collects the reduction message from former iteration.
    async fn collect_from_past(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        iteration: u8,
        committee: &Committee,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        let signature = match &msg.payload {
            Payload::Validation(p) => Ok(p.signature),
            _ => Err(ConsensusError::InvalidMsgType),
        }?;

        // Collect vote, if msg payload is reduction type
        if let Some((hash, sv, quorum_reached)) =
            self.aggr.collect_vote(committee, &msg.header, &signature)
        {
            // Record result in global round registry
            if let Some(quorum_msg) =
                self.sv_registry.lock().await.add_step_votes(
                    iteration,
                    hash,
                    sv,
                    SvType::Validation,
                    quorum_reached,
                )
            {
                return Ok(HandleMsgOutput::Ready(quorum_msg));
            }
        }

        Ok(HandleMsgOutput::Pending(msg))
    }

    /// Handles of an event of step execution timeout
    fn handle_timeout(
        &mut self,
        _ru: &RoundUpdate,
        _iteration: u8,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        Ok(final_result(
            StepVotes::default(),
            [0u8; 32],
            QuorumType::NoQuorum,
        ))
    }
}
