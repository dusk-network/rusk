// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt;

use node_data::ledger::{SpentTransaction, Transaction};

use crate::user::provisioners::Provisioners;

pub type StateRoot = [u8; 32];
pub type EventHash = [u8; 32];

#[derive(Debug)]
pub enum Error {
    Failed,
}

#[derive(Default, Clone, Debug)]
pub struct CallParams {
    pub round: u64,
    pub block_gas_limit: u64,
    pub generator_pubkey: node_data::bls::PublicKey,
}

#[derive(Default)]
pub struct Output {
    pub txs: Vec<SpentTransaction>,
    pub verification_output: VerificationOutput,
    pub provisioners: Provisioners,
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
    async fn verify_state_transition(
        &self,
        params: CallParams,
        txs: Vec<Transaction>,
    ) -> Result<VerificationOutput, Error>;

    async fn execute_state_transition(
        &self,
        params: CallParams,
    ) -> Result<Output, Error>;
}
