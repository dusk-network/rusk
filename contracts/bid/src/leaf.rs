// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical::{Canon, Store};
use canonical_derive::Canon;
use cfg_if::cfg_if;
use core::borrow::Borrow;
use dusk_blindbid::bid::Bid;
use dusk_bls12_381::BlsScalar;

#[derive(Debug, Clone, Copy, Canon)]
pub struct BidLeaf {
    pub bid: Bid,
}

impl Borrow<u64> for BidLeaf {
    fn borrow(&self) -> &u64 {
        &self.bid.pos
    }
}

impl From<Bid> for BidLeaf {
    fn from(bid: Bid) -> BidLeaf {
        BidLeaf { bid }
    }
}

impl From<BidLeaf> for Bid {
    fn from(leaf: BidLeaf) -> Bid {
        leaf.bid
    }
}

extern "C" {
    #[cfg(feature = "hosted")]
    fn p_hash(ofs: &u8, len: u32, ret_addr: &mut [u8; 32]);
}

use poseidon252::tree::PoseidonLeaf;
impl<S> PoseidonLeaf<S> for BidLeaf
where
    S: Store,
{
    fn poseidon_hash(&self) -> BlsScalar {
        // Since we use `cfg_if` the compiler can't see that the mut is
        // necessary due to the branching.
        #[allow(unused_mut)]
        let mut result;
        cfg_if! {
            if #[cfg(feature = "host")] {
                result = self.bid.hash();
            }
            else if #[cfg(feature = "hosted")] {
                unsafe {
                    let mut bid_attrs = [0u8;416];
                    // Add Type fields
                    bid_attrs[0..32].copy_from_slice(&b"53313116000000000000000000000000"[..]);
                    // Add cipher as scalars
                    bid_attrs[32..64].copy_from_slice(&self.bid.encrypted_data.cipher()[0].to_bytes()[..]);
                    bid_attrs[64..96].copy_from_slice(&self.bid.encrypted_data.cipher()[1].to_bytes()[..]);
                    // Add pk_r
                    bid_attrs[128..192].copy_from_slice(&self.bid.encrypted_data.cipher()[0].to_bytes()[..]);
                    //todo!("Finish fields")   ;
                    let mut result_ffi = [0u8; 32];
                    p_hash(&bid_attrs[0], 416, &mut result_ffi);
                    result = BlsScalar::from_bytes(&result_ffi).unwrap()
                }
            }
        }
        result
    }

    fn pos(&self) -> u64 {
        self.bid.pos
    }

    fn set_pos(&mut self, pos: u64) {
        self.bid.pos = pos;
    }
}
