// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use async_trait::async_trait;
use node_data::ledger::{Block, Transaction};

#[derive(Default)]
pub struct Config {}

pub trait VMExecution: Send + Sync + 'static {
    fn execute_state_transition(
        &self,
        txs: &[Transaction],
    ) -> anyhow::Result<()>;

    fn verify_state_transition(
        &self,
        txs: &[Transaction],
    ) -> anyhow::Result<()>;

    fn accept(&self, blk: &Block) -> anyhow::Result<()>;
    fn finalize(&self, blk: &Block) -> anyhow::Result<()>;
    fn preverify(&self, tx: &Transaction) -> anyhow::Result<()>;
}

/// Empty Placeholder for VMExecution
pub struct VMExecutionImpl {}

impl VMExecutionImpl {
    pub fn new(conf: Config) -> Self {
        Self {}
    }
}

impl VMExecution for VMExecutionImpl {
    fn execute_state_transition(
        &self,
        txs: &[Transaction],
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn verify_state_transition(
        &self,
        txs: &[Transaction],
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn accept(&self, blk: &Block) -> anyhow::Result<()> {
        Ok(())
    }

    fn finalize(&self, blk: &Block) -> anyhow::Result<()> {
        Ok(())
    }

    fn preverify(&self, tx: &Transaction) -> anyhow::Result<()> {
        Ok(())
    }
}
