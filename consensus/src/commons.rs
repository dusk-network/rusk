// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

// RoundUpdate carries the data about the new Round, such as the active
// Provisioners, the BidList, the Seed and the Hash.

use node_data::ledger::*;
use node_data::message::payload::{QuorumType, Vote};
use std::collections::HashMap;

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

use dusk_bls12_381_sign::SecretKey;
use node_data::bls::PublicKey;
use node_data::message::{AsyncQueue, Message, Payload};
use node_data::StepName;
use tracing::error;

pub type TimeoutSet = HashMap<StepName, Duration>;

#[derive(Clone, Default, Debug)]
pub struct RoundUpdate {
    // Current round number of the ongoing consensus
    pub round: u64,

    // This provisioner consensus keys
    pub pubkey_bls: PublicKey,
    pub secret_key: SecretKey,

    seed: Seed,
    hash: [u8; 32],
    cert: Certificate,

    pub base_timeouts: TimeoutSet,
}

impl RoundUpdate {
    pub fn new(
        pubkey_bls: PublicKey,
        secret_key: SecretKey,
        mrb_header: &Header,
        base_timeouts: TimeoutSet,
    ) -> Self {
        let round = mrb_header.height + 1;
        RoundUpdate {
            round,
            pubkey_bls,
            secret_key,
            cert: mrb_header.cert,
            hash: mrb_header.hash,
            seed: mrb_header.seed,
            base_timeouts,
        }
    }

    pub fn seed(&self) -> Seed {
        self.seed
    }

    pub fn hash(&self) -> [u8; 32] {
        self.hash
    }

    pub fn cert(&self) -> &Certificate {
        &self.cert
    }
}

#[derive(Debug, Clone, Copy, Error)]
pub enum StepSigError {
    #[error("Failed to reach a quorum")]
    VoteSetTooSmall,
    #[error("Verification error {0}")]
    VerificationFailed(dusk_bls12_381_sign::Error),
    #[error("Empty Apk instance")]
    EmptyApk,
    #[error("Invalid Type")]
    InvalidType,
}

impl From<dusk_bls12_381_sign::Error> for StepSigError {
    fn from(inner: dusk_bls12_381_sign::Error) -> Self {
        Self::VerificationFailed(inner)
    }
}

#[derive(Debug, Clone)]
pub enum ConsensusError {
    InvalidBlock,
    InvalidBlockHash,
    InvalidSignature(dusk_bls12_381_sign::Error),
    InvalidMsgType,
    InvalidValidationStepVotes(StepSigError),
    InvalidValidation(QuorumType),
    InvalidPrevBlockHash(Hash),
    InvalidQuorumType,
    InvalidVote(Vote),
    InvalidMsgIteration(u8),
    FutureEvent,
    PastEvent,
    NotCommitteeMember,
    NotImplemented,
    NotReady,
    MaxIterationReached,
    ChildTaskTerminated,
    Canceled,
}

impl From<StepSigError> for ConsensusError {
    fn from(e: StepSigError) -> Self {
        Self::InvalidValidationStepVotes(e)
    }
}
impl From<dusk_bls12_381_sign::Error> for ConsensusError {
    fn from(e: dusk_bls12_381_sign::Error) -> Self {
        Self::InvalidSignature(e)
    }
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

#[derive(Clone)]
pub(crate) struct QuorumMsgSender {
    queue: AsyncQueue<Message>,
}

impl QuorumMsgSender {
    pub(crate) fn new(queue: AsyncQueue<Message>) -> Self {
        Self { queue }
    }

    /// Sends an quorum (internally) to the quorum loop.
    pub(crate) async fn send_quorum(&self, msg: Message) {
        match &msg.payload {
            Payload::Quorum(q) if !q.cert.ratification.is_empty() => {
                tracing::debug!(
                    event = "send quorum_msg",
                    vote = ?q.vote(),
                    round = msg.header.round,
                    iteration = msg.header.iteration,
                    validation = ?q.cert.validation,
                    ratification = ?q.cert.ratification,
                );
            }
            _ => return,
        }

        if let Err(e) = self.queue.send(msg).await {
            error!("send quorum_msg failed with {e:?}")
        }
    }
}

pub fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|n| n.as_secs())
        .expect("This is heavy.")
}
