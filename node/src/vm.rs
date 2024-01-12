// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_consensus::{
    operations::CallParams, operations::VerificationOutput,
    user::provisioners::Provisioners,
};
use node_data::ledger::{Block, SpentTransaction, Transaction};

#[derive(Default)]
pub struct Config {}

pub trait VMExecution: Send + Sync + 'static {
    fn execute_state_transition<I: Iterator<Item = Transaction>>(
        &self,
        params: &CallParams,
        txs: I,
    ) -> anyhow::Result<(
        Vec<SpentTransaction>,
        Vec<Transaction>,
        VerificationOutput,
    )>;

    fn verify_state_transition(
        &self,
        params: &CallParams,
        txs: Vec<Transaction>,
    ) -> anyhow::Result<VerificationOutput>;

    fn accept(
        &self,
        blk: &Block,
    ) -> anyhow::Result<(Vec<SpentTransaction>, VerificationOutput)>;

    fn finalize(
        &self,
        blk: &Block,
    ) -> anyhow::Result<(Vec<SpentTransaction>, VerificationOutput)>;

    fn preverify(&self, tx: &Transaction) -> anyhow::Result<()>;

    fn get_provisioners(
        &self,
        base_commit: [u8; 32],
    ) -> anyhow::Result<Provisioners>;

    fn get_state_root(&self) -> anyhow::Result<[u8; 32]>;

    fn revert(&self) -> anyhow::Result<[u8; 32]>;
}
