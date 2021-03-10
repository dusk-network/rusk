// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical::{Canon, InvalidEncoding, Store};
use canonical_derive::Canon;
use dusk_bls12_381::BlsScalar;
use dusk_poseidon::tree::PoseidonBranch;
use dusk_poseidon::tree::{PoseidonAnnotation, PoseidonLeaf, PoseidonTree};
use dusk_poseidon::Error as PoseidonError;
use phoenix_core::Note;

pub const TRANSFER_TREE_DEPTH: usize = 17;

#[derive(Debug, Clone, Copy, Canon)]
pub struct Leaf {
    pub block_height: u64,
    pub note: Note,
}

impl From<(u64, Note)> for Leaf {
    fn from(args: (u64, Note)) -> Self {
        let (block_height, note) = args;

        Self { block_height, note }
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

impl<S: Store> Tree<S> {
    pub fn inner(
        &self,
    ) -> &PoseidonTree<Leaf, PoseidonAnnotation, S, TRANSFER_TREE_DEPTH> {
        &self.tree
    }

    pub fn inner_mut(
        &mut self,
    ) -> &mut PoseidonTree<Leaf, PoseidonAnnotation, S, TRANSFER_TREE_DEPTH>
    {
        &mut self.tree
    }

    pub fn get(
        &self,
        pos: u64,
    ) -> Result<Option<Leaf>, PoseidonError<S::Error>> {
        // FIXME invalid casting
        // https://github.com/dusk-network/Poseidon252/issues/116
        self.tree.get(pos as usize)
    }

    pub fn push(&mut self, leaf: Leaf) -> Result<u64, PoseidonError<S::Error>> {
        // FIXME invalid casting
        // https://github.com/dusk-network/Poseidon252/issues/116
        self.tree.push(leaf).map(|pos| pos as u64)
    }

    pub fn root(&mut self) -> Result<BlsScalar, PoseidonError<S::Error>> {
        // FIXME Use proper root
        // https://github.com/dusk-network/rusk/issues/224
        // self.tree.root()
        Ok(BlsScalar::one())
    }

    pub fn opening(
        &self,
        pos: u64,
    ) -> Result<Option<PoseidonBranch<TRANSFER_TREE_DEPTH>>, S::Error> {
        // FIXME invalid casting
        // https://github.com/dusk-network/Poseidon252/issues/116
        self.tree
            .branch(pos as usize)
            .map_err(|_| InvalidEncoding.into())
    }
}
