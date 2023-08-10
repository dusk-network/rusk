// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use anyhow::bail;
use dusk_consensus::commons::Database;
use dusk_consensus::contract_state::{
    CallParams, Error, Operations, Output, VerificationOutput,
};
use node_data::ledger::*;

/// Implements Executor trait to mock Contract Storage calls.
pub struct Executor {}

#[async_trait::async_trait]
impl Operations for Executor {
    async fn verify_state_transition(
        &self,
        _params: CallParams,
        _txs: Vec<Transaction>,
    ) -> Result<VerificationOutput, Error> {
        Ok(VerificationOutput {
            state_root: [0; 32],
            event_hash: [1; 32],
        })
    }

    async fn execute_state_transition(
        &self,
        _params: CallParams,
    ) -> Result<Output, Error> {
        Ok(Output::default())
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
