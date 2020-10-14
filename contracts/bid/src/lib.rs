// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(feature = "hosted", no_std)]
#![feature(lang_items)]

use canonical_derive::Canon;
use dusk_blindbid::{bid::Bid, tree::BidTree};

const PAGE_SIZE: usize = 1024 * 4;

//#[derive(Canon)]
pub struct BidContract {
    bid_tree: BidTree,
}

impl BidContract {
    pub fn new() -> Self {
        BidContract {
            bid_tree: BidTree::new(17usize),
        }
    }

    pub fn inner(&self) -> &BidTree {
        &self.bid_tree
    }

    pub fn inner_mut(&mut self) -> &mut BidTree {
        &mut self.bid_tree
    }
}

#[cfg(feature = "host")]
mod host {
    use super::*;
    use canonical_host::{Module, Query};

    impl Module for BidContract {
        const BYTECODE: &'static [u8] = include_bytes!("../bidcontract.wasm");
    }

    // queries
    type QueryIndex = u8;

    impl BidContract {
        pub fn find_bid(pk: PublicKey) -> Query<(QueryIndex, PublicKey), Bid> {
            Query::new((0, pk))
        }

        pub fn bid(
            bid: Bid,
            correctness_proof: Proof,
            spending_proof: Proof,
        ) -> Query<(QueryIndex, Bid, Proof, Proof), ()> {
            Query::new((1, bid, correctness_proof, spending_proof))
        }
    }
}
