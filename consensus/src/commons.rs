// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

// RoundUpdate carries the data about the new Round, such as the active
// Provisioners, the BidList, the Seed and the Hash.

use node_data::ledger::*;

use node_data::message::Payload;

use dusk_bls12_381_sign::SecretKey;

use node_data::bls::PublicKey;

use crate::config;
use node_data::message::{AsyncQueue, Message};

use tracing::error;

#[derive(Clone, Default, Debug)]
pub struct RoundUpdate {
    // Current round number of the ongoing consensus
    pub round: u64,

    // This provisioner consensus keys
    pub pubkey_bls: PublicKey,
    pub secret_key: SecretKey,

    seed: Seed,
    hash: [u8; 32],
    timestamp: i64,
    cert: Certificate,
}

impl RoundUpdate {
    pub fn new(
        pubkey_bls: PublicKey,
        secret_key: SecretKey,
        mrb_block: &Block,
    ) -> Self {
        let round = mrb_block.header().height + 1;
        RoundUpdate {
            round,
            pubkey_bls,
            secret_key,
            cert: mrb_block.header().cert,
            hash: mrb_block.header().hash,
            seed: mrb_block.header().seed,
            timestamp: mrb_block.header().timestamp,
        }
    }

    pub fn seed(&self) -> Seed {
        self.seed
    }

    pub fn hash(&self) -> [u8; 32] {
        self.hash
    }

    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }

    pub fn cert(&self) -> &Certificate {
        &self.cert
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ConsensusError {
    InvalidBlock,
    InvalidBlockHash,
    InvalidSignature,
    InvalidMsgType,
    InvalidValidation,
    InvalidQuorumType,
    FutureEvent,
    PastEvent,
    NotCommitteeMember,
    NotImplemented,
    NotReady,
    MaxIterationReached,
    ChildTaskTerminated,
    Canceled,
}
#[async_trait::async_trait]
pub trait Database: Send + Sync {
    fn store_candidate_block(&mut self, b: Block);
    async fn get_candidate_block_by_hash(
        &self,
        h: &Hash,
    ) -> anyhow::Result<Block>;
    fn delete_candidate_blocks(&mut self);
}

pub enum StepName {
    Proposal,
    Validation,
    Ratification,
}

pub trait IterCounter {
    /// Count of all steps per a single iteration
    const STEP_NUM: u8 = 3;
    type Step;
    fn next(&mut self) -> Result<Self, ConsensusError>
    where
        Self: Sized;
    fn from_step(step_num: Self::Step) -> Self;
    fn step_from_name(&self, st: StepName) -> Self::Step;
    fn step_from_pos(&self, pos: usize) -> Self::Step;
}

impl IterCounter for u8 {
    type Step = u8;

    fn next(&mut self) -> Result<Self, ConsensusError> {
        let next = *self + 1;
        if next >= config::CONSENSUS_MAX_ITER {
            return Err(ConsensusError::MaxIterationReached);
        }

        *self = next;
        Ok(next)
    }

    fn from_step(step: Self::Step) -> Self {
        step / Self::STEP_NUM
    }

    fn step_from_name(&self, st: StepName) -> Self::Step {
        let proposal_step_num = self * Self::STEP_NUM;
        match st {
            StepName::Proposal => proposal_step_num,
            StepName::Validation => proposal_step_num + 1,
            StepName::Ratification => proposal_step_num + 2,
        }
    }

    fn step_from_pos(&self, pos: usize) -> Self::Step {
        self * Self::STEP_NUM + pos as u8
    }
}

#[derive(Clone)]
pub(crate) struct QuorumMsgSender {
    queue: AsyncQueue<Message>,
}

impl QuorumMsgSender {
    pub(crate) fn new(queue: AsyncQueue<Message>) -> Self {
        Self { queue }
    }

    /// Sends an quorum (internally) to the quorum loop.
    pub(crate) async fn send(&self, msg: Message) -> bool {
        if let Payload::Quorum(q) = &msg.payload {
            if q.signature == [0u8; 48]
                || q.validation.is_empty()
                || q.ratification.is_empty()
                || msg.header.block_hash == [0; 32]
            {
                return false;
            }

            tracing::debug!(
                event = "send quorum_msg",
                hash = to_str(&msg.header.block_hash),
                round = msg.header.round,
                step = msg.header.step,
                validation = format!("{:#?}", q.validation),
                ratification = format!("{:#?}", q.ratification),
                signature = to_str(&q.signature),
            );

            let _ = self
                .queue
                .send(msg.clone())
                .await
                .map_err(|e| error!("send quorum_msg failed with {:?}", e));

            return true;
        }

        false
    }
}
