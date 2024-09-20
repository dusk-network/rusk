// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt;
use std::io;
use std::time::Duration;

use node_data::bls::PublicKey;
use node_data::bls::PublicKeyBytes;
use node_data::ledger::Hash;
use node_data::ledger::{
    Block, Fault, Header, InvalidFault, Slash, SpentTransaction, Transaction,
};
use node_data::message::payload::RatificationResult;
use node_data::StepName;
use thiserror::Error;

use crate::commons::StepSigError;

pub type StateRoot = [u8; 32];
pub type EventHash = [u8; 32];
pub type Voter = (PublicKey, usize);

#[derive(Debug, Error)]
pub enum Error {
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

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::InvalidIterationInfo(value)
    }
}

impl From<InvalidFault> for Error {
    fn from(value: InvalidFault) -> Self {
        Self::InvalidFaults(value)
    }
}

#[derive(Default, Clone, Debug)]
pub struct CallParams {
    pub round: u64,
    pub generator_pubkey: node_data::bls::PublicKey,
    pub to_slash: Vec<Slash>,
    pub voters_pubkey: Option<Vec<Voter>>,
    pub max_txs_bytes: usize,
}

#[derive(Default)]
pub struct Output {
    pub txs: Vec<SpentTransaction>,
    pub verification_output: VerificationOutput,
    pub discarded_txs: Vec<Transaction>,
}

#[derive(Debug, Default, PartialEq)]
pub struct VerificationOutput {
    pub state_root: StateRoot,
    pub event_hash: EventHash,
}

impl fmt::Display for VerificationOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VerificationOutput {{ state_root: {}, event_hash: {} }}",
            hex::encode(self.state_root),
            hex::encode(self.event_hash)
        )
    }
}

#[async_trait::async_trait]
pub trait Operations: Send + Sync {
    async fn verify_candidate_header(
        &self,
        candidate_header: &Header,
        expected_generator: &PublicKeyBytes,
    ) -> Result<(u8, Vec<Voter>, Vec<Voter>), HeaderError>;

    async fn verify_faults(
        &self,
        block_height: u64,
        faults: &[Fault],
    ) -> Result<(), Error>;

    async fn verify_state_transition(
        &self,
        blk: &Block,
        voters: &[Voter],
    ) -> Result<VerificationOutput, Error>;

    async fn execute_state_transition(
        &self,
        params: CallParams,
    ) -> Result<Output, Error>;

    async fn add_step_elapsed_time(
        &self,
        round: u64,
        step_name: StepName,
        elapsed: Duration,
    ) -> Result<(), Error>;

    async fn get_block_gas_limit(&self) -> u64;
}
