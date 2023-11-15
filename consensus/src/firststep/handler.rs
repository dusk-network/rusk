// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::Arc;

use crate::aggregator::Aggregator;
use crate::commons::{ConsensusError, Database, RoundUpdate};
use crate::msg_handler::{HandleMsgOutput, MsgHandler};
use crate::step_votes_reg::{SafeStepVotesRegistry, SvType};
use async_trait::async_trait;
use node_data::ledger;
use node_data::ledger::{Block, StepVotes};
use tokio::sync::Mutex;

use crate::user::committee::Committee;
use node_data::message::{payload, Message, Payload};

const EMPTY_SIGNATURE: [u8; 48] = [0u8; 48];

macro_rules! empty_result {
    (  ) => {
        HandleMsgOutput::Ready(Message::from_stepvotes(
            payload::StepVotesWithCandidate {
                sv: StepVotes::default(),
                candidate: Block::default(),
            },
        ))
    };
}

fn final_result(sv: StepVotes, candidate: ledger::Block) -> HandleMsgOutput {
    HandleMsgOutput::Ready(Message::from_stepvotes(
        payload::StepVotesWithCandidate { sv, candidate },
    ))
}

fn final_result_with_timeout(
    sv: StepVotes,
    candidate: ledger::Block,
) -> HandleMsgOutput {
    HandleMsgOutput::ReadyWithTimeoutIncrease(Message::from_stepvotes(
        payload::StepVotesWithCandidate { sv, candidate },
    ))
}

pub struct Reduction<DB: Database> {
    sv_registry: SafeStepVotesRegistry,

    pub(crate) db: Arc<Mutex<DB>>,
    pub(crate) aggr: Aggregator,
    pub(crate) candidate: Block,
    curr_step: u8,
}

impl<DB: Database> Reduction<DB> {
    pub(crate) fn new(
        db: Arc<Mutex<DB>>,
        sv_registry: SafeStepVotesRegistry,
    ) -> Self {
        Self {
            sv_registry,
            db,
            aggr: Aggregator::default(),
            candidate: Block::default(),
            curr_step: 0,
        }
    }

    pub(crate) fn reset(&mut self, curr_step: u8) {
        self.candidate = Block::default();
        self.curr_step = curr_step;
    }
}

#[async_trait]
impl<D: Database> MsgHandler<Message> for Reduction<D> {
    /// Verifies if a msg is a valid reduction message.
    fn verify(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        _step: u8,
        _committee: &Committee,
    ) -> Result<Message, ConsensusError> {
        let signed_hash = match &msg.payload {
            Payload::Reduction(p) => Ok(p.signature),
            Payload::Empty => Ok(EMPTY_SIGNATURE),
            _ => Err(ConsensusError::InvalidMsgType),
        }?;

        if msg.header.verify_signature(&signed_hash).is_err() {
            return Err(ConsensusError::InvalidSignature);
        }

        Ok(msg)
    }

    /// Collects the reduction message.
    async fn collect(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        step: u8,
        committee: &Committee,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        assert_eq!(step, self.curr_step, "msg does not belong to curr step");

        let signature = match &msg.payload {
            Payload::Reduction(p) => Ok(p.signature),
            Payload::Empty => Ok(EMPTY_SIGNATURE),
            _ => Err(ConsensusError::InvalidMsgType),
        }?;

        // Collect vote, if msg payload is reduction type
        if let Some((hash, sv)) =
            self.aggr.collect_vote(committee, &msg.header, &signature)
        {
            // Record result in global round registry
            _ = self.sv_registry.lock().await.add_step_votes(
                step,
                hash,
                sv,
                SvType::FirstReduction,
            );

            // if the votes converged for an empty hash we invoke halt
            if hash == [0u8; 32] {
                tracing::warn!("votes converged for an empty hash");

                return Ok(final_result_with_timeout(
                    StepVotes::default(),
                    ledger::Block::default(),
                ));
            }

            if hash != self.candidate.header().hash {
                // If the block generator is behind this node, we'll miss the
                // candidate block.
                if let Ok(block) = self
                    .db
                    .lock()
                    .await
                    .get_candidate_block_by_hash(&hash)
                    .await
                {
                    return Ok(final_result(sv, block));
                }

                tracing::error!("Failed to retrieve candidate block.");
                return Ok(empty_result!());
            }

            return Ok(final_result(sv, self.candidate.clone()));
        }

        Ok(HandleMsgOutput::Pending(msg))
    }

    /// Collects the reduction message from former iteration.
    async fn collect_from_past(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        step: u8,
        committee: &Committee,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        let signature = match &msg.payload {
            Payload::Reduction(p) => Ok(p.signature),
            Payload::Empty => Ok(EMPTY_SIGNATURE),
            _ => Err(ConsensusError::InvalidMsgType),
        }?;

        // Collect vote, if msg payload is reduction type
        if let Some((hash, sv)) =
            self.aggr.collect_vote(committee, &msg.header, &signature)
        {
            // Record result in global round registry
            if let Some(m) = self.sv_registry.lock().await.add_step_votes(
                step,
                hash,
                sv,
                SvType::FirstReduction,
            ) {
                // TODO: Send an agreement
            }
        }

        Ok(HandleMsgOutput::Pending(msg))
    }

    /// Handles of an event of step execution timeout
    fn handle_timeout(
        &mut self,
        _ru: &RoundUpdate,
        _step: u8,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        Ok(final_result(StepVotes::default(), self.candidate.clone()))
    }
}
