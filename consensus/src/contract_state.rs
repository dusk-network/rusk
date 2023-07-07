// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::user::provisioners::Provisioners;
use node_data::ledger::{SpentTransaction, Transaction};

pub type StateRoot = [u8; 32];

#[derive(Debug)]
pub enum Error {
    Failed,
}

#[allow(unused)]
#[derive(Default)]
pub struct CallParams {
    pub round: u64,
    pub txs: Vec<Transaction>,
    pub block_gas_limit: u64,
    pub generator_pubkey: node_data::bls::PublicKey,
}

#[allow(unused)]
#[derive(Default)]
pub struct Output {
    pub txs: Vec<SpentTransaction>,
    pub state_root: StateRoot,
    pub provisioners: Provisioners,
    pub discarded_txs: Vec<Transaction>,
}
pub trait Operations: Send + Sync {
    fn verify_state_transition(
        &self,
        params: CallParams,
    ) -> Result<StateRoot, Error>;

    fn get_mempool_txs(
        &self,
        block_gas_limit: u64,
    ) -> Result<Vec<Transaction>, Error>;

    fn execute_state_transition(
        &self,
        params: CallParams,
    ) -> Result<Output, Error>;
    // fn accept(&self, params: CallParams) -> Result<Output, Error>;
    // fn finalize(&self, params: CallParams) -> Result<Output, Error>;

    // fn get_state_root(&self) -> Result<StateRoot, Error>;
}
