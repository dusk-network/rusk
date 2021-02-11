// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Leaf;

use canonical::{Canon, Store};
use canonical_derive::Canon;
use dusk_bls12_381::BlsScalar;
use dusk_poseidon::tree::{PoseidonAnnotation, PoseidonBranch, PoseidonTree};
use dusk_poseidon::Error as PoseidonError;

pub const TRANSFER_TREE_DEPTH: usize = 17;

#[derive(Debug, Default, Clone, Canon)]
pub struct Tree<S>
where
    S: Store,
{
    tree: PoseidonTree<Leaf, PoseidonAnnotation, S, TRANSFER_TREE_DEPTH>,
}

impl<S> Tree<S>
where
    S: Store,
{
    pub fn get(
        &self,
        pos: usize,
    ) -> Result<Option<Leaf>, PoseidonError<S::Error>> {
        self.tree.get(pos)
    }

    pub fn push(
        &mut self,
        leaf: Leaf,
    ) -> Result<usize, PoseidonError<S::Error>> {
        self.tree.push(leaf)
    }

    pub fn root(&self) -> Result<BlsScalar, PoseidonError<S::Error>> {
        self.tree.root()
    }

    pub fn path(
        &self,
        pos: u64,
    ) -> Result<
        Option<PoseidonBranch<TRANSFER_TREE_DEPTH>>,
        PoseidonError<S::Error>,
    > {
        // FIXME this cast will truncate positions greater than 2^32 for wasm32
        // environments. The tree is set for 2^34, so that will happen
        self.tree.branch(pos as usize)
    }
}
