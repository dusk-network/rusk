// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical::{Canon, InvalidEncoding, Store};
use canonical_derive::Canon;
use dusk_bls12_381::BlsScalar;
use dusk_poseidon::tree::{PoseidonAnnotation, PoseidonLeaf, PoseidonTree};
use phoenix_core::Note;

pub const TRANSFER_TREE_DEPTH: usize = 17;

#[derive(Debug, Clone, Copy, Canon)]
pub struct Leaf {
    block_height: u64,
    note: Note,
}

impl From<(u64, Note)> for Leaf {
    fn from(args: (u64, Note)) -> Self {
        let (block_height, note) = args;

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
        rusk_abi::poseidon_hash(self.note.hash_inputs().into())
    }

    fn pos(&self) -> u64 {
        self.note.pos()
    }

    fn set_pos(&mut self, pos: u64) {
        self.note.set_pos(pos);
    }
}

#[derive(Debug, Default, Clone, Canon)]
pub struct Tree<S>
where
    S: Store,
{
    tree: PoseidonTree<Leaf, PoseidonAnnotation, S, TRANSFER_TREE_DEPTH>,
}

impl<S: Store>
    AsRef<PoseidonTree<Leaf, PoseidonAnnotation, S, TRANSFER_TREE_DEPTH>>
    for Tree<S>
{
    fn as_ref(
        &self,
    ) -> &PoseidonTree<Leaf, PoseidonAnnotation, S, TRANSFER_TREE_DEPTH> {
        &self.tree
    }
}

impl<S: Store>
    AsMut<PoseidonTree<Leaf, PoseidonAnnotation, S, TRANSFER_TREE_DEPTH>>
    for Tree<S>
{
    fn as_mut(
        &mut self,
    ) -> &mut PoseidonTree<Leaf, PoseidonAnnotation, S, TRANSFER_TREE_DEPTH>
    {
        &mut self.tree
    }
}

impl<S: Store> Tree<S> {
    pub fn root(&mut self) -> Result<BlsScalar, S::Error> {
        self.tree.root().map_err(|_| InvalidEncoding.into())
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    use dusk_poseidon::tree::PoseidonBranch;
    use dusk_poseidon::Error as PoseidonError;

    impl<S> Tree<S>
    where
        S: Store,
    {
        pub fn push(
            &mut self,
            leaf: Leaf,
        ) -> Result<usize, PoseidonError<S::Error>> {
            self.tree.push(leaf)
        }

        pub fn opening(
            &self,
            pos: u64,
        ) -> Result<Option<PoseidonBranch<TRANSFER_TREE_DEPTH>>, S::Error>
        {
            // FIXME invalid casting
            // https://github.com/dusk-network/Poseidon252/issues/116
            self.tree
                .branch(pos as usize)
                .map_err(|_| InvalidEncoding.into())
        }
    }
}
