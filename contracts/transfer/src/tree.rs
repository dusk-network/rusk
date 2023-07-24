// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;

use dusk_bls12_381::BlsScalar;
use phoenix_core::transaction::*;
use phoenix_core::Note;

use poseidon_merkle::{
    Item as PoseidonItem, Opening as PoseidonOpening, Tree as PoseidonTree,
};

use crate::state::A;

pub struct Tree {
    tree: PoseidonTree<(), TRANSFER_TREE_DEPTH, A>,
    // Since `dusk-merkle` does not include data blocks with the tree, we do it
    // here.
    leaves: Vec<TreeLeaf>,
}

impl Tree {
    pub const fn new() -> Self {
        Self {
            tree: PoseidonTree::new(),
            leaves: Vec::new(),
        }
    }

    pub fn get(&self, pos: u64) -> Option<TreeLeaf> {
        self.leaves.get(pos as usize).cloned()
    }

    pub fn push(&mut self, mut leaf: TreeLeaf) -> u64 {
        // update the position before computing the hash
        let pos = self.leaves.len() as u64;
        leaf.note.set_pos(pos);

        // compute the item that goes in the leaf of the tree
        let hash = rusk_abi::poseidon_hash(leaf.note.hash_inputs().to_vec());
        let item = PoseidonItem { hash, data: () };

        self.tree.insert(pos, item);
        self.leaves.push(leaf);

        pos
    }

    pub fn extend_notes<I: IntoIterator<Item = Note>>(
        &mut self,
        block_height: u64,
        notes: I,
    ) {
        for note in notes {
            let leaf = TreeLeaf { block_height, note };
            self.push(leaf);
        }
    }

    pub fn root(&self) -> BlsScalar {
        self.tree.root().hash
    }

    /// Return an iterator through the leaves in the tree, starting from a given
    /// `height`.
    pub fn leaves(&self, height: u64) -> impl Iterator<Item = &TreeLeaf> {
        // We can do this since we know the leaves are strictly increasing in
        // block height. If this ever changes - such as in the case of a
        // sparsely populated tree - we should annotate the tree and use
        // `Tree::walk` instead.
        self.leaves
            .iter()
            .skip_while(move |leaf| leaf.block_height < height)
    }

    pub fn opening(
        &self,
        pos: u64,
    ) -> Option<PoseidonOpening<(), TRANSFER_TREE_DEPTH, A>> {
        self.tree.opening(pos)
    }
}
