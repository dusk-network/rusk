// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical::{Canon, Store};
use canonical_derive::Canon;
use dusk_bls12_381::BlsScalar;
use phoenix_core::Note;
use poseidon252::tree::PoseidonLeaf;

#[cfg(feature = "hosted")]
extern "C" {
    fn p_hash(ofs: &u8, len: u32, ret_addr: &mut [u8; 32]);
}

#[derive(Debug, Clone, Copy, Canon)]
pub struct Leaf {
    note: Note,
}

impl AsRef<Note> for Leaf {
    fn as_ref(&self) -> &Note {
        &self.note
    }
}

impl From<Note> for Leaf {
    fn from(note: Note) -> Self {
        Self { note }
    }
}

impl<S> PoseidonLeaf<S> for Leaf
where
    S: Store,
{
    #[cfg(feature = "host")]
    fn poseidon_hash(&self) -> BlsScalar {
        self.note.hash()
    }

    #[cfg(feature = "hosted")]
    fn poseidon_hash(&self) -> BlsScalar {
        const B_LEN: usize = 12 * 32;
        let mut bytes = [0u8; B_LEN];

        // Grant the expected inputs is 12
        let hash_inputs: [BlsScalar; 12] = self.note.hash_inputs();

        bytes
            .chunks_mut(32)
            .zip(hash_inputs.iter())
            .for_each(|(b, h)| b.copy_from_slice(&h.to_bytes()));

        let mut result_ffi = [0u8; 32];

        unsafe {
            p_hash(&bytes[0], B_LEN as u32, &mut result_ffi);
        }

        BlsScalar::from_bytes(&result_ffi).unwrap_or(BlsScalar::zero())
    }

    fn pos(&self) -> u64 {
        self.note.pos()
    }

    fn set_pos(&mut self, pos: u64) {
        self.note.set_pos(pos);
    }
}
