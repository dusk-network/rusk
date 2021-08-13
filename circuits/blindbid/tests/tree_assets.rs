// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical_derive::Canon;
use core::borrow::Borrow;
use dusk_blindbid::Bid;
use dusk_plonk::bls12_381::BlsScalar;
use dusk_poseidon::tree::{PoseidonLeaf, PoseidonMaxAnnotation, PoseidonTree};
use microkelvin::Keyed;

#[derive(Debug, Clone, Copy, Canon)]
pub struct BidLeaf(pub(crate) Bid);

impl BidLeaf {
    /// Generates a new BidLeaf instance from a `Bid`.
    pub fn new(bid: Bid) -> Self {
        BidLeaf(bid)
    }

    /// Returns the internal bid representation of the `BidLeaf` as with
    /// the `Bid` type.
    pub fn bid(&self) -> Bid {
        self.0
    }

    /// Returns a &mut to the internal bid representation of the `BidLeaf`
    /// as with the `Bid` type.
    pub fn bid_mut(&mut self) -> &mut Bid {
        &mut self.0
    }
}

impl Borrow<u64> for BidLeaf {
    fn borrow(&self) -> &u64 {
        self.0.borrow()
    }
}

impl From<Bid> for BidLeaf {
    fn from(bid: Bid) -> BidLeaf {
        BidLeaf(bid)
    }
}

impl From<BidLeaf> for Bid {
    fn from(leaf: BidLeaf) -> Bid {
        leaf.0
    }
}

impl PoseidonLeaf for BidLeaf {
    fn poseidon_hash(&self) -> BlsScalar {
        self.0.hash()
    }

    fn pos(&self) -> &u64 {
        self.0.pos()
    }

    fn set_pos(&mut self, pos: u64) {
        self.0.set_pos(pos);
    }
}

impl Keyed<u64> for BidLeaf {
    fn key(&self) -> &u64 {
        self.0.pos()
    }
}

#[allow(dead_code)]
pub type BidTree = PoseidonTree<BidLeaf, PoseidonMaxAnnotation<u64>, 17>;
