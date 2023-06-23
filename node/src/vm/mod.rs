// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use async_trait::async_trait;
use dusk_bls12_381_sign::PublicKey as BlsPublicKey;
use dusk_consensus::{
    contract_state::CallParams, user::provisioners::Provisioners,
};
use node_data::ledger::{Block, Transaction};

#[derive(Default)]
pub struct Config {}

pub trait VMExecution: Send + Sync + 'static {
    fn execute_state_transition(
        &self,
        params: &CallParams,
    ) -> anyhow::Result<(Vec<Transaction>, Vec<Transaction>, [u8; 32])>;

    fn verify_state_transition(
        &self,
        params: &CallParams,
    ) -> anyhow::Result<[u8; 32]>;

    fn accept(
        &self,
        blk: &Block,
    ) -> anyhow::Result<(Vec<Transaction>, [u8; 32])>;
    
    fn finalize(
        &self,
        blk: &Block,
    ) -> anyhow::Result<(Vec<Transaction>, [u8; 32])>;

    fn preverify(&self, tx: &Transaction) -> anyhow::Result<()>;

    fn get_provisioners(&self) -> anyhow::Result<Provisioners>;

    fn get_state_root(&self) -> anyhow::Result<[u8; 32]>;
}
