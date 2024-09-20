// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io;

use execution_core::signatures::bls::Error as BlsSigError;
use node_data::{
    ledger::{Hash, InvalidFault},
    message::payload::{QuorumType, RatificationResult, Vote},
    StepName,
};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Error)]
pub enum StepSigError {
    #[error("Failed to reach a quorum")]
    VoteSetTooSmall,
    #[error("Verification error {0}")]
    VerificationFailed(BlsSigError),
    #[error("Invalid Type")]
    InvalidType,
}

impl From<BlsSigError> for StepSigError {
    fn from(inner: BlsSigError) -> Self {
        Self::VerificationFailed(inner)
    }
}

#[derive(Debug, Clone)]
pub enum ConsensusError {
    InvalidBlock,
    InvalidBlockHash,
    InvalidBlockSize(usize),
    InvalidSignature(BlsSigError),
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
    ChildTaskTerminated,
    Canceled(u64),
    VoteAlreadyCollected,
    TooManyTransactions(usize),
    TooManyFaults(usize),
    UnknownBlockSize,
}

impl From<StepSigError> for ConsensusError {
    fn from(e: StepSigError) -> Self {
        Self::InvalidValidationStepVotes(e)
    }
}
impl From<BlsSigError> for ConsensusError {
    fn from(e: BlsSigError) -> Self {
        Self::InvalidSignature(e)
    }
}

#[derive(Debug, Error)]
pub enum OperationError {
    #[error("failed to call VST {0}")]
    InvalidVST(anyhow::Error),
    #[error("failed to call EST {0}")]
    InvalidEST(anyhow::Error),
    #[error("failed to verify header {0}")]
    InvalidHeader(HeaderError),
    #[error("Unable to update metrics {0}")]
    MetricsUpdate(anyhow::Error),
    #[error("Invalid Iteration Info {0}")]
    InvalidIterationInfo(io::Error),
    #[error("Invalid Faults {0}")]
    InvalidFaults(InvalidFault),
}

#[derive(Debug, Error)]
pub enum HeaderError {
    #[error("unsupported block version")]
    UnsupportedVersion,
    #[error("empty block hash")]
    EmptyHash,
    #[error("invalid block height block_height: {0}, curr_height: {0}")]
    MismatchHeight(u64, u64),
    #[error("block time is less than minimum block time")]
    BlockTimeLess,
    #[error("block timestamp {0} is higher than local time")]
    BlockTimeHigher(u64),
    #[error("invalid previous block hash")]
    PrevBlockHash,
    #[error("block already exists")]
    BlockExists,
    #[error("invalid block signature: {0}")]
    InvalidBlockSignature(String),
    #[error("invalid seed: {0}")]
    InvalidSeed(String),

    #[error("Invalid Attestation: {0}")]
    InvalidAttestation(AttestationError),

    #[error("Generic error in header verification: {0}")]
    Generic(anyhow::Error),
}

impl HeaderError {
    pub fn must_vote(&self) -> bool {
        match self {
            HeaderError::MismatchHeight(_, _) => false,
            HeaderError::BlockTimeHigher(_) => false,
            HeaderError::PrevBlockHash => false,
            HeaderError::BlockExists => false,
            HeaderError::InvalidBlockSignature(_) => false,

            HeaderError::BlockTimeLess => true,
            HeaderError::UnsupportedVersion => true,
            HeaderError::EmptyHash => true,
            HeaderError::InvalidSeed(_) => true,
            HeaderError::InvalidAttestation(_) => true,

            // TODO: This must be removed as soon as we remove all anyhow errors
            HeaderError::Generic(_) => true,
        }
    }
}

#[derive(Debug, Clone, Copy, Error)]
pub enum AttestationError {
    #[error("Invalid votes for {0:?}: {1:?}")]
    InvalidVotes(StepName, StepSigError),
    #[error("Expected block hash: {0:?}, Got: {1:?}")]
    InvalidHash(Hash, Hash),
    #[error("Result: {0:?}, Expected: {1:?}")]
    InvalidResult(RatificationResult, RatificationResult),
}

impl From<AttestationError> for HeaderError {
    fn from(value: AttestationError) -> Self {
        Self::InvalidAttestation(value)
    }
}

impl From<anyhow::Error> for HeaderError {
    fn from(value: anyhow::Error) -> Self {
        Self::Generic(value)
    }
}

impl From<io::Error> for OperationError {
    fn from(value: io::Error) -> Self {
        Self::InvalidIterationInfo(value)
    }
}

impl From<InvalidFault> for OperationError {
    fn from(value: InvalidFault) -> Self {
        Self::InvalidFaults(value)
    }
}
