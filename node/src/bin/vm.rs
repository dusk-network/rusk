// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node::vm::{Config, VMExecution};
use node_data::ledger::{Block, Transaction};

/// Empty Placeholder for VMExecution
pub struct VMExecutionImpl {}

impl VMExecutionImpl {
    pub fn new(_conf: Config) -> Self {
        Self {}
    }
}

impl VMExecution for VMExecutionImpl {
    fn execute_state_transition(
        &self,
        _txs: &[Transaction],
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn verify_state_transition(
        &self,
        _txs: &[Transaction],
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn accept(&self, _blk: &Block) -> anyhow::Result<()> {
        Ok(())
    }

    fn finalize(&self, _blk: &Block) -> anyhow::Result<()> {
        Ok(())
    }

    fn preverify(&self, _tx: &Transaction) -> anyhow::Result<()> {
        Ok(())
    }
}
