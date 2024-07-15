// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt;
use std::io;
use std::time::Duration;

use execution_core::StakePublicKey;
use node_data::ledger::Fault;
use node_data::ledger::InvalidFault;
use node_data::ledger::{Block, Header, Slash, SpentTransaction, Transaction};
use node_data::StepName;
use thiserror::Error;

pub type StateRoot = [u8; 32];
pub type EventHash = [u8; 32];
pub type VoterWithCredits = (StakePublicKey, usize);

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to call VST {0}")]
    InvalidVST(anyhow::Error),
    #[error("failed to call EST {0}")]
    InvalidEST(anyhow::Error),
    #[error("failed to verify header {0}")]
    InvalidHeader(anyhow::Error),
    #[error("Unable to update metrics {0}")]
    MetricsUpdate(anyhow::Error),
    #[error("Invalid Iteration Info {0}")]
    InvalidIterationInfo(io::Error),
    #[error("Invalid Faults {0}")]
    InvalidFaults(InvalidFault),
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
    pub block_gas_limit: u64,
    pub generator_pubkey: node_data::bls::PublicKey,
    pub to_slash: Vec<Slash>,
    pub voters_pubkey: Option<Vec<VoterWithCredits>>,
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
    async fn verify_block_header(
        &self,
        candidate_header: &Header,
        disable_winning_att_check: bool,
    ) -> Result<(u8, Vec<VoterWithCredits>, Vec<VoterWithCredits>), Error>;

    async fn verify_faults(
        &self,
        block_height: u64,
        faults: &[Fault],
    ) -> Result<(), Error>;

    async fn verify_state_transition(
        &self,
        blk: &Block,
        voters: &[VoterWithCredits],
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
}
