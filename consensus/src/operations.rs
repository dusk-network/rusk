// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt;
use std::time::Duration;

use dusk_bls12_381_sign::PublicKey;
use node_data::ledger::{Block, Header, SpentTransaction, Transaction};
use node_data::StepName;

pub type StateRoot = [u8; 32];
pub type EventHash = [u8; 32];

#[derive(Debug)]
pub enum Error {
    Failed,
    InvalidIterationInfo,
}

#[derive(Default, Clone, Debug)]
pub struct CallParams {
    pub round: u64,
    pub block_gas_limit: u64,
    pub generator_pubkey: node_data::bls::PublicKey,
    pub missed_generators: Vec<PublicKey>,
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
        disable_winning_cert_check: bool,
    ) -> Result<(), Error>;

    async fn verify_state_transition(
        &self,
        blk: &Block,
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
