// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(feature = "hosted", no_std)]
#![feature(min_const_generics)]

pub(crate) mod leaf;
pub(crate) mod map;
pub(crate) mod tree;

pub use leaf::BidLeaf;

use canonical::{Canon, Sink, Source, Store};
use map::KeyToIdxMap;
use tree::BidTree;

pub mod ops {
    // QUERIES
    pub const FIND_BID: u16 = 0x00;
    pub const WITHDRAW: u16 = 0x01;

    // Transactions
    pub const BID: u16 = 0x02;
    pub const EXTEND_BID: u16 = 0x03;
}

#[derive(Debug, Clone)]
pub struct Contract<S: Store> {
    tree: BidTree<S>,
    map: KeyToIdxMap<S>,
}

impl<S> Canon<S> for Contract<S>
where
    S: Store,
{
    fn read(source: &mut impl Source<S>) -> Result<Self, S::Error> {
        Ok(Contract {
            tree: Canon::<S>::read(source)?,
            map: Canon::<S>::read(source)?,
        })
    }

    fn write(&self, sink: &mut impl Sink<S>) -> Result<(), S::Error> {
        self.tree.write(sink)?;
        self.map.write(sink)
    }

    fn encoded_len(&self) -> usize {
        Canon::<S>::encoded_len(&self.tree) + Canon::<S>::encoded_len(&self.map)
    }
}

impl<S: Store> Contract<S> {
    pub fn new() -> Self {
        Self {
            tree: BidTree::new(),
            map: KeyToIdxMap::new(),
        }
    }

    pub fn tree(&self) -> &BidTree<S> {
        &self.tree
    }

    pub fn tree_mut(&mut self) -> &mut BidTree<S> {
        &mut self.tree
    }

    pub fn map(&self) -> &KeyToIdxMap<S> {
        &self.map
    }

    pub fn map_mut(&mut self) -> &mut KeyToIdxMap<S> {
        &mut self.map
    }
}

#[cfg(feature = "host")]
pub mod host;

#[cfg(feature = "hosted")]
pub mod hosted;
