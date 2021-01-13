// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::leaf::BidLeaf;
use crate::contract_constants::BID_TREE_DEPTH;
use canonical::{Canon, Store};
use canonical_derive::Canon;
use microkelvin::{BranchMut, Nth};
use nstack::NStack;
use poseidon252::tree::{
    PoseidonBranch, PoseidonMaxAnnotation, PoseidonTree, PoseidonTreeIterator,
};

/// Append-only tree structure which wraps over `PoseidonTree<BidLeaf>`
/// responsible of the storage and the branch generation capabilities
/// of the Bid contract.
#[derive(Debug, Clone, Canon)]
pub struct BidTree<S>
where
    S: Store,
{
    tree: PoseidonTree<BidLeaf, PoseidonMaxAnnotation, S, BID_TREE_DEPTH>,
}

impl<S> BidTree<S>
where
    S: Store,
{
    /// Constructor for the Tree structure.
    pub fn new() -> Self {
        Self {
            tree: PoseidonTree::new(),
        }
    }

    /// Reference to the internal poseidon tree
    ///
    /// We don't have a mutable reference available because all its mutation
    /// should be protected by encapsulation
    fn inner_mut(
        &mut self,
    ) -> &mut PoseidonTree<BidLeaf, PoseidonMaxAnnotation, S, BID_TREE_DEPTH>
    {
        &mut self.tree
    }

    /// Get a bid from a provided index
    pub fn get(&self, idx: u64) -> Option<BidLeaf> {
        self.tree
            .get(idx as usize)
            .expect("Couldn't traverse the tree.")
    }

    /// Returns a mutable refecence to the internal `Bid` that corresponds
    /// to the given `idx`.
    pub fn get_mut<'a>(
        &'a mut self,
        idx: u64,
    ) -> Option<
        BranchMut<
            'a,
            NStack<BidLeaf, PoseidonMaxAnnotation, S>,
            S,
            BID_TREE_DEPTH,
        >,
    > {
        self.inner_mut()
            .as_mut()
            .nth_mut::<BID_TREE_DEPTH>(idx)
            .expect("Couldn't traverse the tree")
    }

    /// Append a bid to the tree and return its index
    ///
    /// The index will be the last available position
    pub fn push(&mut self, bid: BidLeaf) -> usize {
        self.tree.push(bid).unwrap()
    }

    /// Returns a poseidon branch pointing at the specific index
    pub fn poseidon_branch(
        &self,
        idx: usize,
    ) -> Option<PoseidonBranch<BID_TREE_DEPTH>> {
        self.tree.branch(idx).unwrap()
    }

    /// Returns an iterator over the leaves of the Tree starting from a certain
    /// block-height.
    pub fn iter_block_height(
        &self,
        block_height: u64,
    ) -> PoseidonTreeIterator<
        BidLeaf,
        PoseidonMaxAnnotation,
        S,
        u64,
        BID_TREE_DEPTH,
    > {
        self.tree.iter_walk(block_height).unwrap()
    }
}

// TODO: Fix block-height annotated search on the tree.
/*
#[cfg(test)]
mod tests {
    use crate::bid::Bid;
    use crate::tree::BidTree;
    use crate::{V_RAW_MAX, V_RAW_MIN};
    use canonical_host::MemStore;
    use dusk_pki::SecretSpendKey;
    use dusk_plonk::jubjub::{AffinePoint, GENERATOR_EXTENDED};
    use dusk_plonk::prelude::*;
    use rand::rngs::StdRng;
    use rand::{CryptoRng, Rng, SeedableRng};
    use std::cmp;

    #[derive(Debug, Clone, Copy)]
    pub struct BidContainer {
        pub bid: Bid,
        pub k: BlsScalar,
        pub sk: SecretSpendKey,
        pub encrypt_secret: AffinePoint,
        pub idx: u64,
    }

    impl BidContainer {
        /// Constructor
        pub fn new(
            bid: Bid,
            k: BlsScalar,
            sk: SecretSpendKey,
            encrypt_secret: AffinePoint,
            idx: u64,
        ) -> Self {
            Self {
                bid,
                k,
                sk,
                encrypt_secret,
                idx,
            }
        }

        /// Set the tree index
        pub fn set_idx(&mut self, idx: u64) {
            self.idx = idx;
        }

        /// Create a random bid with all the underlying data except for the tree
        /// index
        pub fn random<R>(rng: &mut R) -> BidContainer
        where
            R: Rng + CryptoRng,
        {
            let k = BlsScalar::random(rng);

            let sk = SecretSpendKey::random(rng);
            let pk = sk.public_key();

            let pk_r = pk.gen_stealth_address(&JubJubScalar::random(rng));

            let encrypt_secret = JubJubScalar::random(rng);
            let encrypt_secret: AffinePoint =
                (GENERATOR_EXTENDED * encrypt_secret).into();

            let value: u64 =
                (&mut rand::thread_rng()).gen_range(V_RAW_MIN, V_RAW_MAX);
            let value = JubJubScalar::from(value);

            let a = rng.gen_range(0u64, u64::MAX);
            let b = rng.gen_range(0u64, u64::MAX);

            let elegibility = cmp::min(a, b);
            let expiration = cmp::max(a, b);

            let bid = Bid::new(
                rng,
                &pk_r,
                &value,
                &encrypt_secret,
                k,
                elegibility,
                expiration,
            )
            .expect("Error generating bid!");

            BidContainer::new(bid, k, sk, encrypt_secret, 0)
        }
    }

    #[test]
    fn block_height_search() {
        let mut tree = BidTree::<MemStore>::new();
        let mut rng_seed = StdRng::seed_from_u64(437894u64);
        let rng = &mut rng_seed;

        // Create 250 random bids and append them to the tree
        let bids: Vec<BidContainer> = (0..2)
            .map(|_| {
                let mut b = BidContainer::random(rng);

                tree.push(b.bid)
                    .map(|idx| b.set_idx(idx as u64))
                    .expect("Failed to append bid to the tree!");

                b
            })
            .collect();
        //panic!("{:?}", bids);
        // Perform the search on every bid
        bids.iter().for_each(|b| {
            let block_height = b.bid.eligibility;
            let view_key = b.sk.view_key();

            let results: Vec<Bid> = tree.iter_block_height(block_height).unwrap().filter_map(|b| {
                let b = b.unwrap();

                if b.expiration < block_height {
                    panic!("An expired bid was returned");
                }

                if view_key.owns(&b) {
                    Some(b)
                } else {
                    None
                }
            }).collect();

            // The target bid should be returned
            if !results.iter().any(|r| r.hash() == b.bid.hash()) {
                panic!(
                    "Search failed for bid {} with expiration {:?} and filter {:?} with criteria {:?}\nMAX= {}",
                    b.idx,
                    b.bid.expiration,
                    block_height,
                    block_height <= b.bid.expiration,
                    u64::MAX
                );
            }
        });
    }
}
*/
