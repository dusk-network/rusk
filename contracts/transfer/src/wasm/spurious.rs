// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{Transfer, TRANSFER_TREE_DEPTH};

use alloc::vec::Vec;
use canonical::Store;
use dusk_bls12_381::BlsScalar;
use dusk_poseidon::tree::PoseidonBranch;
use phoenix_core::Note;

impl<S: Store> Transfer<S> {
    pub fn balance(&self, address: BlsScalar) -> u64 {
        self.balance
            .get(&address)
            .unwrap_or_default()
            .map(|v| *v)
            .unwrap_or_default()
    }

    pub fn root(&self) -> BlsScalar {
        self.notes.as_ref().root().unwrap_or_default()
    }

    pub fn notes_from_height(&self, block_height: u64) -> Vec<Note> {
        self.notes_mapping
            .get(&block_height)
            .unwrap_or_default()
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    pub fn opening(
        &self,
        pos: u64,
    ) -> Option<PoseidonBranch<TRANSFER_TREE_DEPTH>> {
        self.notes.opening(pos).unwrap_or_default()
    }
}
