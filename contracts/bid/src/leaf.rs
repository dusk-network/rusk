// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical::{Canon, Store};
use canonical_derive::Canon;
use core::borrow::Borrow;
use dusk_blindbid::Bid;
use dusk_bls12_381::BlsScalar;
use dusk_poseidon::tree::PoseidonLeaf;

/// Wrapper struct over `dusk-blindbid::Bid` which is needed
/// to be able to implement `PoseidonLeaf` trait logic so that the
/// hashing of the Bids is done in the host envoirnoment instead
/// of WASM.
///
/// Aside from this difference, BidLeaf does not vary on anything
/// from the original `Bid` struct at all.
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
        &self.0.borrow()
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

// Since the `sponge_hash` fn of `Poseidon` is quite expensive, the variant
// when executed in the `hosted` envoiroment would indeed call a host_function
// to do the computations in Rust instead of WASM.
impl<S> PoseidonLeaf<S> for BidLeaf
where
    S: Store,
{
    #[cfg(not(target_arch = "wasm32"))]
    fn poseidon_hash(&self) -> BlsScalar {
        self.0.hash()
    }

    #[cfg(target_arch = "wasm32")]
    fn poseidon_hash(&self) -> BlsScalar {
        rusk_abi::hosted::poseidon_hash(self.0.as_hash_inputs().into())
    }

    fn pos(&self) -> u64 {
        self.0.pos()
    }

    fn set_pos(&mut self, pos: u64) {
        self.0.set_pos(pos);
    }
}
