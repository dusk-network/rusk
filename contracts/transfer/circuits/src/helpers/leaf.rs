// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical::{Canon, Store};
use canonical_derive::Canon;
use dusk_plonk::bls12_381::BlsScalar;
use phoenix_core::Note;
use poseidon252::tree::PoseidonLeaf;

#[derive(Debug, Clone, Canon)]
pub struct NoteLeaf(Note);

impl From<Note> for NoteLeaf {
    fn from(note: Note) -> NoteLeaf {
        NoteLeaf(note)
    }
}

impl From<NoteLeaf> for Note {
    fn from(leaf: NoteLeaf) -> Note {
        leaf.0
    }
}

impl<S> PoseidonLeaf<S> for NoteLeaf
where
    S: Store,
{
    fn poseidon_hash(&self) -> BlsScalar {
        self.0.hash()
    }

    fn pos(&self) -> u64 {
        self.0.pos()
    }

    fn set_pos(&mut self, pos: u64) {
        self.0.set_pos(pos)
    }
}
