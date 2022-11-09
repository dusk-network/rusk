// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::Transaction;
use crate::user::provisioners::Provisioners;

pub type StateRoot = [u8; 32];

#[derive(Debug)]
pub enum Error {
    Failed,
}

#[allow(unused)]
#[derive(Default)]
pub struct CallParams {
    round: u64,
    txs: Vec<Transaction>,
    block_gas_limit: u64,
    generator_pubkey: crate::util::pubkey::ConsensusPublicKey,
}

#[allow(unused)]
#[derive(Default)]
pub struct Output {
    txs: Vec<Transaction>,
    state_root: StateRoot,
    provisioners: Provisioners,
}

pub trait Operations: Send + Sync {
    fn verify_state_transition(
        &self,
        params: CallParams,
    ) -> Result<StateRoot, Error>;

    fn execute_state_transition(
        &self,
        params: CallParams,
    ) -> Result<Output, Error>;
    fn accept(&self, params: CallParams) -> Result<Output, Error>;
    fn finalize(&self, params: CallParams) -> Result<Output, Error>;

    fn get_state_root(&self) -> Result<StateRoot, Error>;
}

// TODO: implement trait Operations
