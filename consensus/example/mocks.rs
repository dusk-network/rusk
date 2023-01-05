// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_consensus::{
    commons::{Block, Database, Hash},
    contract_state::{CallParams, Error, Operations, Output, StateRoot},
};

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
}

#[derive(Debug, Default)]
/// Implements Database trait to store candidates blocks in heap memory.
pub struct SimpleDB {
    candidates: std::collections::BTreeMap<Hash, Block>,
}

impl Database for SimpleDB {
    fn store_candidate_block(&mut self, b: Block) {
        if b.header.hash == Hash::default() {
            tracing::error!("candidate block with empty hash");
            return;
        }

        self.candidates.entry(b.header.hash).or_insert(b);
    }

    fn get_candidate_block_by_hash(&self, h: &Hash) -> Option<(Hash, Block)> {
        if let Some(v) = self.candidates.get_key_value(h) {
            return Some((*v.0, v.1.clone()));
        }
        None
    }

    fn delete_candidate_blocks(&mut self) {
        self.candidates.clear();
    }
}
