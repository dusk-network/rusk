// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use core::borrow::Borrow;

use dusk_bls12_381::BlsScalar;
use dusk_poseidon::tree::{PoseidonBranch, PoseidonLeaf, PoseidonTree};
use microkelvin::{Child, Compound, Step, Walk, Walker};
use nstack::annotation::{Keyed, MaxKey};
use phoenix_core::Note;
use ranno::Annotation;

pub const TRANSFER_TREE_DEPTH: usize = 17;

#[derive(Debug, Clone, Copy)]
pub struct Leaf {
    pub block_height: u64,
    pub note: Note,
}

impl AsRef<Note> for Leaf {
    fn as_ref(&self) -> &Note {
        &self.note
    }
}

impl Borrow<u64> for Leaf {
    fn borrow(&self) -> &u64 {
        self.note.pos()
    }
}

impl Keyed<u64> for Leaf {
    fn key(&self) -> &u64 {
        &self.block_height
    }
}

impl From<(u64, Note)> for Leaf {
    fn from(args: (u64, Note)) -> Self {
        let (block_height, note) = args;
        let block_height = block_height.into();

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

#[derive(Debug, Clone)]
pub struct Tree {
    tree: PoseidonTree<Leaf, u64, TRANSFER_TREE_DEPTH>,
}

impl Tree {
    pub const fn new() -> Self {
        Self {
            tree: PoseidonTree::new(),
        }
    }

    pub fn inner(&self) -> &PoseidonTree<Leaf, u64, TRANSFER_TREE_DEPTH> {
        &self.tree
    }

    pub fn inner_mut(
        &mut self,
    ) -> &mut PoseidonTree<Leaf, u64, TRANSFER_TREE_DEPTH> {
        &mut self.tree
    }

    pub fn get(&self, pos: u64) -> Option<Leaf> {
        self.tree.get(pos)
    }

    pub fn push(&mut self, leaf: Leaf) -> u64 {
        self.tree.push(leaf)
    }

    pub fn root(&mut self) -> BlsScalar {
        self.tree.root()
    }

    pub fn opening(
        &self,
        pos: u64,
    ) -> Option<PoseidonBranch<TRANSFER_TREE_DEPTH>> {
        self.tree.branch(pos)
    }

    pub fn leaves(
        &self,
        block_height: u64,
    ) -> Option<impl Iterator<Item = &Leaf>> {
        self.tree
            .annotated_iter_walk(BlockHeightFilter(block_height))
            .map(|v| v.into_iter())
    }
}

// Walker method to find the elements that are above a certain a block height.
pub struct BlockHeightFilter(u64);

impl<C, A> Walker<C, A> for BlockHeightFilter
where
    C: Compound<A>,
    C::Leaf: Keyed<u64>,
    A: Annotation<C> + Borrow<MaxKey<u64>>,
{
    fn walk(&mut self, walk: Walk<C, A>) -> Step {
        for i in 0.. {
            match walk.child(i) {
                Child::Leaf(l) => {
                    if l.key() >= &self.0 {
                        return Step::Found(i);
                    } else {
                        self.0 -= 1
                    }
                }
                Child::Node(n) => {
                    let max_node_block_height: u64 = match *(*n.anno()).borrow()
                    {
                        MaxKey::NegativeInfinity => return Step::Abort,
                        MaxKey::Maximum(value) => value,
                    };
                    if max_node_block_height >= self.0 {
                        return Step::Into(i);
                    } else {
                        self.0 -= 1
                    }
                }
                Child::Empty => (),
                Child::EndOfNode => return Step::Advance,
            }
        }
        unreachable!()
    }
}
