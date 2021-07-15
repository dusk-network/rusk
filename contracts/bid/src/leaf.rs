// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical_derive::Canon;
use core::borrow::Borrow;
use dusk_blindbid::Bid;
use dusk_bls12_381::BlsScalar;
use dusk_poseidon::tree::{
    PoseidonAnnotation, PoseidonLeaf, PoseidonTreeAnnotation,
};
use microkelvin::{
    Annotation, Cardinality, Child, Combine, Compound, Keyed, MaxKey, Step,
    Walk, Walker,
};

/// Alias for `u64` which relates to the expiration of a `Bid`.
#[derive(
    Copy, Clone, Default, Debug, Canon, Ord, PartialOrd, Eq, PartialEq,
)]
pub struct Expiration(pub(crate) u64);

/// Annotation to filter leafs of a tree in respect of the Expiration of them.
#[derive(Clone, Debug, Default, Canon)]
pub struct ExpirationAnnotation {
    ann: PoseidonAnnotation,
    expiration: MaxKey<Expiration>,
}

impl Borrow<MaxKey<Expiration>> for ExpirationAnnotation {
    fn borrow(&self) -> &MaxKey<Expiration> {
        &self.expiration
    }
}

impl Borrow<Cardinality> for ExpirationAnnotation {
    fn borrow(&self) -> &Cardinality {
        self.ann.borrow()
    }
}

impl Borrow<BlsScalar> for ExpirationAnnotation {
    fn borrow(&self) -> &BlsScalar {
        self.ann.borrow()
    }
}

impl<L> Annotation<L> for ExpirationAnnotation
where
    L: PoseidonLeaf,
    L: Borrow<u64>,
    L: Keyed<Expiration>,
{
    fn from_leaf(leaf: &L) -> Self {
        let ann = PoseidonAnnotation::from_leaf(leaf);
        let expiration = MaxKey::from_leaf(leaf);

        Self { ann, expiration }
    }
}

impl<C, A> Combine<C, A> for ExpirationAnnotation
where
    C: Compound<A>,
    C::Leaf: PoseidonLeaf + Keyed<Expiration> + Borrow<u64>,
    A: Annotation<C::Leaf>
        + PoseidonTreeAnnotation<C::Leaf>
        + Borrow<Cardinality>
        + Borrow<MaxKey<Expiration>>,
{
    fn combine(node: &C) -> Self {
        ExpirationAnnotation {
            ann: PoseidonAnnotation::combine(node),
            expiration: MaxKey::combine(node),
        }
    }
}

impl PoseidonTreeAnnotation<BidLeaf> for ExpirationAnnotation {}

/// Walker method to find the elements that are avobe a certain a expiration.
pub struct ExpirationFilter(u64);

impl<C, A> Walker<C, A> for ExpirationFilter
where
    C: Compound<A>,
    C::Leaf: Keyed<Expiration>,
    A: Combine<C, A> + Borrow<MaxKey<Expiration>>,
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
                        match n.annotation().borrow() {
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
impl PoseidonLeaf for BidLeaf {
    #[cfg(not(target_arch = "wasm32"))]
    fn poseidon_hash(&self) -> BlsScalar {
        self.0.hash()
    }

    #[cfg(target_arch = "wasm32")]
    fn poseidon_hash(&self) -> BlsScalar {
        rusk_abi::hosted::poseidon_hash(self.0.as_hash_inputs().into())
    }

    fn pos(&self) -> &u64 {
        self.0.pos()
    }

    fn set_pos(&mut self, pos: u64) {
        self.0.set_pos(pos);
    }
}

impl Keyed<Expiration> for BidLeaf {
    fn key(&self) -> &Expiration {
        unsafe {
            core::mem::transmute::<&u64, &Expiration>(self.0.expiration())
        }
    }
}
