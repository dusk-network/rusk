// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use anyhow::bail;
use dusk_consensus::commons::Database;
use dusk_consensus::contract_state::{
    CallParams, Error, Operations, Output, StateRoot,
};
use node_data::ledger::*;

/// Implements Executor trait to mock Contract Storage calls.
pub struct Executor {}

impl Operations for Executor {
    fn verify_state_transition(
        &self,
        _params: CallParams,
    ) -> Result<StateRoot, Error> {
        Ok([0; 32])
    }

    fn execute_state_transition(
        &self,
        _params: CallParams,
    ) -> Result<Output, Error> {
        Ok(Output::default())
    }

    fn accept(&self, _params: CallParams) -> Result<Output, Error> {
        Ok(Output::default())
    }

    fn finalize(&self, _params: CallParams) -> Result<Output, Error> {
        Ok(Output::default())
    }

    fn get_state_root(&self) -> Result<StateRoot, Error> {
        Ok([0; 32])
    }

    fn get_mempool_txs(
        &self,
        _block_gas_limit: u64,
    ) -> Result<Vec<Transaction>, Error> {
        Ok(vec![])
    }
}

#[derive(Debug, Default)]
/// Implements Database trait to store candidates blocks in heap memory.
pub struct SimpleDB {
    candidates: std::collections::BTreeMap<Hash, Block>,
}

#[async_trait::async_trait]
impl Database for SimpleDB {
    fn store_candidate_block(&mut self, b: Block) {
        if b.header.hash == Hash::default() {
            tracing::error!("candidate block with empty hash");
            return;
        }

        self.candidates.entry(b.header.hash).or_insert(b);
    }

    async fn get_candidate_block_by_hash(
        &self,
        h: &Hash,
    ) -> anyhow::Result<Block> {
        if let Some(v) = self.candidates.get_key_value(h) {
            return Ok(v.1.clone());
        }

        bail!("not found")
    }

    fn delete_candidate_blocks(&mut self) {
        self.candidates.clear();
    }
}
