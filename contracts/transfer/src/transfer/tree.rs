// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Error;
use core::borrow::Borrow;

use canonical::Canon;
use canonical_derive::Canon;
use dusk_bls12_381::BlsScalar;
use dusk_poseidon::tree::{
    PoseidonAnnotation, PoseidonBranch, PoseidonLeaf, PoseidonTree,
    PoseidonTreeAnnotation,
};
use microkelvin::{
    AnnoIter, Annotation, Cardinality, Child, Combine, Compound, Keyed, MaxKey,
    Step, Walk, Walker,
};
use phoenix_core::Note;

pub const TRANSFER_TREE_DEPTH: usize = 17;

/// Annotation to filter leafs of a tree in respect of the Expiration of them.
#[derive(Clone, Debug, Default, Canon)]
pub struct NotesAnnotation {
    poseidon: PoseidonAnnotation,
    block_height: MaxKey<BlockHeight>,
}

impl Borrow<MaxKey<BlockHeight>> for NotesAnnotation {
    fn borrow(&self) -> &MaxKey<BlockHeight> {
        &self.block_height
    }
}

impl Borrow<Cardinality> for NotesAnnotation {
    fn borrow(&self) -> &Cardinality {
        self.poseidon.borrow()
    }
}

impl Borrow<BlsScalar> for NotesAnnotation {
    fn borrow(&self) -> &BlsScalar {
        self.poseidon.borrow()
    }
}

impl Borrow<PoseidonAnnotation> for NotesAnnotation {
    fn borrow(&self) -> &PoseidonAnnotation {
        &self.poseidon
    }
}

impl<L> Annotation<L> for NotesAnnotation
where
    L: PoseidonLeaf,
    L: Borrow<u64>,
    L: Keyed<BlockHeight>,
{
    fn from_leaf(leaf: &L) -> Self {
        let poseidon = PoseidonAnnotation::from_leaf(leaf);
        let block_height = MaxKey::from_leaf(leaf);

        Self {
            poseidon,
            block_height,
        }
    }
}

impl<A> Combine<A> for NotesAnnotation
where
    A: Borrow<Cardinality>
        + Borrow<MaxKey<BlockHeight>>
        + Borrow<PoseidonAnnotation>
        + Borrow<BlsScalar>,
{
    fn combine<C: Compound<A>>(iter: AnnoIter<C, A>) -> Self
    where
        C: Compound<A>,
        A: Annotation<C::Leaf>,
    {
        NotesAnnotation {
            poseidon: PoseidonAnnotation::combine(iter.clone()),
            block_height: MaxKey::combine(iter),
        }
    }
}

impl PoseidonTreeAnnotation<Leaf> for NotesAnnotation where Leaf:  {}

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Canon,
)]
pub struct BlockHeight(u64);

impl From<u64> for BlockHeight {
    fn from(b: u64) -> Self {
        Self(b)
    }
}

impl From<BlockHeight> for u64 {
    fn from(b: BlockHeight) -> u64 {
        b.0
    }
}

#[derive(Debug, Clone, Copy, Canon)]
pub struct Leaf {
    pub block_height: BlockHeight,
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

impl Keyed<BlockHeight> for Leaf {
    fn key(&self) -> &BlockHeight {
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

#[derive(Debug, Default, Clone, Canon)]
pub struct Tree {
    tree: PoseidonTree<Leaf, NotesAnnotation, TRANSFER_TREE_DEPTH>,
}

impl Tree {
    pub fn inner(
        &self,
    ) -> &PoseidonTree<Leaf, NotesAnnotation, TRANSFER_TREE_DEPTH> {
        &self.tree
    }

    pub fn inner_mut(
        &mut self,
    ) -> &mut PoseidonTree<Leaf, NotesAnnotation, TRANSFER_TREE_DEPTH> {
        &mut self.tree
    }

    pub fn get(&self, pos: u64) -> Result<Option<Leaf>, Error> {
        Ok(self.tree.get(pos)?)
    }

    pub fn push(&mut self, leaf: Leaf) -> Result<u64, Error> {
        Ok(self.tree.push(leaf)?)
    }

    pub fn root(&mut self) -> Result<BlsScalar, Error> {
        Ok(self.tree.root()?)
    }

    pub fn opening(
        &self,
        pos: u64,
    ) -> Result<Option<PoseidonBranch<TRANSFER_TREE_DEPTH>>, Error> {
        Ok(self.tree.branch(pos)?)
    }

    pub fn notes(
        &self,
        block_height: u64,
    ) -> Result<impl Iterator<Item = Result<&Note, Error>>, Error> {
        Ok(self
            .tree
            .annotated_iter_walk(BlockHeightFilter(block_height))?
            .into_iter()
            .map(|result| {
                result.map_err(|e| e.into()).map(|leaf| leaf.as_ref())
            }))
    }
}

// Walker method to find the elements that are avobe a certain a block height.
pub struct BlockHeightFilter(u64);

impl<C, A> Walker<C, A> for BlockHeightFilter
where
    C: Compound<A>,
    C::Leaf: Keyed<BlockHeight>,
    A: Combine<A> + Annotation<C::Leaf> + Borrow<MaxKey<BlockHeight>> + Canon,
{
    fn walk(&mut self, walk: Walk<C, A>) -> Step {
        for i in 0.. {
            match walk.child(i) {
                Child::Leaf(l) => {
                    if l.key().0 >= self.0 {
                        return Step::Found(i);
                    } else {
                        self.0 -= 1
                    }
                }
                Child::Node(n) => {
                    let max_node_block_height: u64 =
                        match *(*n.annotation()).borrow() {
                            MaxKey::NegativeInfinity => return Step::Abort,
                            MaxKey::Maximum(value) => value.0,
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
