// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical::{Canon, Store};
use canonical_derive::Canon;
use dusk_bls12_381::BlsScalar;
use dusk_poseidon::tree::PoseidonLeaf;
use phoenix_core::Note;

#[derive(Debug, Clone, Copy, Canon)]
pub struct Leaf {
    block_height: u64,
    note: Note,
}

impl Leaf {
    pub fn new(block_height: u64, note: Note) -> Self {
        Self { block_height, note }
    }
}

impl AsRef<Note> for Leaf {
    fn as_ref(&self) -> &Note {
        &self.note
    }
}

impl<S> PoseidonLeaf<S> for Leaf
where
    S: Store,
{
    #[cfg(not(target_arch = "wasm32"))]
    fn poseidon_hash(&self) -> BlsScalar {
        self.note.hash()
    }

    #[cfg(target_arch = "wasm32")]
    fn poseidon_hash(&self) -> BlsScalar {
        dusk_abi::poseidon_hash(self.note.hash_inputs().into())
    }

    fn pos(&self) -> u64 {
        self.note.pos()
    }

    fn set_pos(&mut self, pos: u64) {
        self.note.set_pos(pos);
    }
}
