// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

// RoundUpdate carries the data about the new Round, such as the active
// Provisioners, the BidList, the Seed and the Hash.

use node_data::ledger::*;
use node_data::message::payload::{QuorumType, Vote};
use std::fmt;
use std::fmt::Display;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use node_data::message::Payload;

use dusk_bls12_381_sign::SecretKey;

use node_data::bls::PublicKey;

use node_data::message::{AsyncQueue, Message};

use tracing::error;

#[derive(Clone, Default, Debug)]
pub struct RoundUpdate {
    // Current round number of the ongoing consensus
    pub round: u64,
    pub round_base_timeout: Duration,

    // This provisioner consensus keys
    pub pubkey_bls: PublicKey,
    pub secret_key: SecretKey,

    seed: Seed,
    hash: [u8; 32],
    cert: Certificate,
}

impl RoundUpdate {
    pub fn new(
        pubkey_bls: PublicKey,
        secret_key: SecretKey,
        mrb_header: &Header,
        round_base_timeout: Duration,
    ) -> Self {
        let round = mrb_header.height + 1;
        RoundUpdate {
            round,
            pubkey_bls,
            secret_key,
            cert: mrb_header.cert,
            hash: mrb_header.hash,
            seed: mrb_header.seed,
            round_base_timeout,
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

#[derive(Debug, Clone, Copy)]
pub enum Error {
    VoteSetTooSmall(u16),
    VerificationFailed(dusk_bls12_381_sign::Error),
    EmptyApk,
    InvalidType,
    InvalidStepNum,
}

impl From<dusk_bls12_381_sign::Error> for Error {
    fn from(inner: dusk_bls12_381_sign::Error) -> Self {
        Self::VerificationFailed(inner)
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::VoteSetTooSmall(step) => {
                write!(f, "Failed to reach a quorum at step {step}")
            }
            Error::VerificationFailed(_) => write!(f, "Verification error"),
            Error::EmptyApk => write!(f, "Empty Apk instance"),
            Error::InvalidType => write!(f, "Invalid Type"),
            Error::InvalidStepNum => write!(f, "Invalid step number"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ConsensusError {
    InvalidBlock,
    InvalidBlockHash,
    InvalidSignature(dusk_bls12_381_sign::Error),
    InvalidMsgType,
    InvalidValidationStepVotes(Error),
    InvalidValidation(QuorumType),
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

impl From<Error> for ConsensusError {
    fn from(e: Error) -> Self {
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
    pub(crate) async fn send(&self, msg: Message) -> bool {
        if let Payload::Quorum(q) = &msg.payload {
            if q.header.signature.is_zeroed()
                || q.validation.is_empty()
                || q.ratification.is_empty()
                || q.vote == Vote::NoCandidate
            // TODO: Change me accoringly to https://github.com/dusk-network/rusk/issues/1268
            {
                return false;
            }

            tracing::debug!(
                event = "send quorum_msg",
                vote = %q.vote,
                round = msg.header.round,
                iteration = msg.header.iteration,
                validation = ?q.validation,
                ratification = ?q.ratification,
                signature = to_str(q.header.signature.inner()),
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

pub fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|n| n.as_secs())
        .expect("This is heavy.")
}
