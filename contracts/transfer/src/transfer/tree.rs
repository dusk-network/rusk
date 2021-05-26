// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Error;

use canonical_derive::Canon;
use dusk_bls12_381::BlsScalar;
use dusk_poseidon::tree::{
    PoseidonAnnotation, PoseidonBranch, PoseidonLeaf, PoseidonTree,
};
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

impl From<Leaf> for Note {
    fn from(leaf: Leaf) -> Note {
        leaf.note
    }
}

impl PoseidonLeaf for Leaf {
    #[cfg(not(target_arch = "wasm32"))]
    fn poseidon_hash(&self) -> BlsScalar {
        self.note.hash()
    }

    #[cfg(target_arch = "wasm32")]
    fn poseidon_hash(&self) -> BlsScalar {
        rusk_abi::poseidon_hash(self.note.hash_inputs().into())
    }

    fn pos(&self) -> &u64 {
        self.note.pos()
    }

    fn set_pos(&mut self, pos: u64) {
        self.note.set_pos(pos);
    }
}

#[derive(Debug, Default, Clone, Canon)]
pub struct Tree {
    tree: PoseidonTree<Leaf, PoseidonAnnotation, TRANSFER_TREE_DEPTH>,
}

impl Tree {
    pub fn inner(
        &self,
    ) -> &PoseidonTree<Leaf, PoseidonAnnotation, TRANSFER_TREE_DEPTH> {
        &self.tree
    }

    pub fn inner_mut(
        &mut self,
    ) -> &mut PoseidonTree<Leaf, PoseidonAnnotation, TRANSFER_TREE_DEPTH> {
        &mut self.tree
    }

    pub fn get(&self, pos: u64) -> Result<Option<Leaf>, Error> {
        Ok(self.tree.get(pos)?)
    }

    pub fn push(&mut self, leaf: Leaf) -> Result<u64, Error> {
        Ok(self.tree.push(leaf).map(|pos| pos)?)
    }

    pub fn root(&mut self) -> Result<BlsScalar, Error> {
        // FIXME Use proper root
        // https://github.com/dusk-network/rusk/issues/224
        // self.tree.root()
        Ok(BlsScalar::one())
    }

    pub fn opening(
        &self,
        pos: u64,
    ) -> Result<Option<PoseidonBranch<TRANSFER_TREE_DEPTH>>, Error> {
        Ok(self.tree.branch(pos)?)
    }
}
