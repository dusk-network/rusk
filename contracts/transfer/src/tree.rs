// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use core::borrow::Borrow;
use core::ops::Range;

use dusk_bls12_381::BlsScalar;
use dusk_poseidon::tree::PoseidonTree;
use microkelvin::{Child, Compound, Step, Walk, Walker};
use nstack::annotation::{Keyed, MaxKey};
use ranno::Annotation;
use transfer_contract_types::TreeLeaf;

pub const TRANSFER_TREE_DEPTH: usize = 17;

#[derive(Debug, Clone)]
pub struct Tree {
    tree: PoseidonTree<TreeLeaf, u64, TRANSFER_TREE_DEPTH>,
}

impl Tree {
    pub const fn new() -> Self {
        Self {
            tree: PoseidonTree::new(),
        }
    }

    pub fn get(&self, pos: u64) -> Option<TreeLeaf> {
        self.tree.get(pos)
    }

    pub fn push(&mut self, leaf: TreeLeaf) -> u64 {
        self.tree.push(leaf)
    }

    pub fn root(&self) -> BlsScalar {
        self.tree.root()
    }

    pub fn leaves(
        &self,
        range: Range<u64>,
    ) -> Option<impl Iterator<Item = &TreeLeaf>> {
        self.tree
            .annotated_iter_walk(HeightRangeWalker(range))
            .map(|v| v.into_iter())
    }
}

/// Walker to find the leaves that are between two block heights.
pub struct HeightRangeWalker(Range<u64>);

impl<C, A> Walker<C, A> for HeightRangeWalker
where
    C: Compound<A>,
    C::Leaf: Keyed<u64>,
    A: Annotation<C> + Borrow<MaxKey<u64>>,
{
    fn walk(&mut self, walk: Walk<C, A>) -> Step {
        for i in 0.. {
            match walk.child(i) {
                Child::Leaf(l) => {
                    if self.0.contains(l.key()) {
                        return Step::Found(i);
                    }
                }
                Child::Node(n) => {
                    let max_node_block_height = match *(*n.anno()).borrow() {
                        MaxKey::NegativeInfinity => return Step::Abort,
                        MaxKey::Maximum(max) => max,
                    };

                    if max_node_block_height >= self.0.start {
                        return Step::Into(i);
                    }
                }
                Child::Empty => {}
                Child::EndOfNode => return Step::Advance,
            }
        }
        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::HeightRangeWalker;

    use dusk_bls12_381::BlsScalar;
    use dusk_poseidon::tree::{PoseidonLeaf, PoseidonTree};
    use nstack::annotation::Keyed;

    const TRANSFER_TREE_DEPTH: usize = 17;

    #[derive(Debug, Clone, Copy)]
    pub struct Leaf {
        pub block_height: u64,
        pub pos: u64,
    }

    impl Keyed<u64> for Leaf {
        fn key(&self) -> &u64 {
            &self.block_height
        }
    }

    impl PoseidonLeaf for Leaf {
        fn poseidon_hash(&self) -> BlsScalar {
            BlsScalar::zero()
        }

        fn pos(&self) -> &u64 {
            &self.pos
        }

        fn set_pos(&mut self, pos: u64) {
            self.pos = pos;
        }
    }

    type Tree = PoseidonTree<Leaf, u64, TRANSFER_TREE_DEPTH>;

    #[test]
    fn walk() {
        let mut tree = Tree::new();

        for i in 0..64 {
            tree.push(Leaf {
                block_height: i / 4,
                pos: 0,
            });
        }

        let (from_height, until_height) = (2, 5);
        let range = from_height..until_height;

        let walk = tree
            .annotated_iter_walk(HeightRangeWalker(range.clone()))
            .expect("There should be a walker");

        for leaf in walk
            .into_iter()
            .take_while(move |item| item.block_height < until_height)
        {
            assert!(range.contains(&leaf.block_height));
        }
    }
}
