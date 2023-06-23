// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_consensus::user::provisioners::{Provisioners, DUSK};
use node::vm::{Config, VMExecution};
use node_data::ledger::{Block, Transaction};

/// Empty Placeholder for VMExecution
pub struct VMExecutionImpl {}

impl VMExecutionImpl {
    pub fn new(_conf: Config) -> Self {
        Self {}
    }
    fn get_mocked_provisioners() -> Provisioners {
        // Load provisioners keys from external consensus keys files.
        let keys = node_data::bls::load_provisioners_keys(4);
        let mut provisioners = Provisioners::new();

        for (_, (_, pk)) in keys.iter().enumerate() {
            tracing::info!("Adding provisioner: {:#?}", pk);
            provisioners.add_member_with_value(pk.clone(), 1000 * DUSK * 10);
        }

        provisioners
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
    fn get_provisioners(&self) -> Result<Provisioners, anyhow::Error> {
        Ok(VMExecutionImpl::get_mocked_provisioners())
    }
    fn get_state_root(&self) -> anyhow::Result<[u8; 32]> {
        Ok([0u8;32])
    }
}
