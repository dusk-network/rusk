// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Implementation of the Bid Genesis Contract.
//! This smart contract that acts as a decentralized interface to manage blind
//! bids. It is complementary to the Proof Of Blind Bid algorithm used within
//! the Dusk Network consensus.
//! For more documentation regarding the contract or the blindbid protocol,
//! please check the following [docs](https://app.gitbook.com/@dusk-network/s/specs/specifications/smart-contracts/genenesis-contracts/bid-contract)

#![cfg_attr(target_arch = "wasm32", no_std)]
#![cfg_attr(
    target_arch = "wasm32",
    feature(core_intrinsics, lang_items, alloc_error_handler)
)]
#![deny(missing_docs)]

extern crate alloc;
pub(crate) mod leaf;
pub(crate) mod map;

#[cfg(target_arch = "wasm32")]
pub(crate) mod hosted;

use canonical_derive::Canon;
use dusk_poseidon::tree::PoseidonTree;
pub use leaf::{BidLeaf, Expiration, ExpirationAnnotation, ExpirationFilter};
use map::KeyToIdxMap;

/// `VerifierKey` used by the `BidCorrectnessCircuit` to verify a
/// Bid correctness `Proof` using the PLONK proving systyem.
pub const BID_CORRECTNESS_VD: &[u8] = core::include_bytes!(
    "../../../.rusk/keys/9213f1d9a165da07ceb3eafebefbd1216bb80ab151e7e1ae888ad4670c2bef70.vd"
);

/// OPCODEs for each contract method
pub mod ops {
    // Transactions
    /// Bid fn OPCODE.
    pub const BID: u8 = 0x01;
    /// WITHDRAW Bid OPCODE
    pub const WITHDRAW: u8 = 0x02;
    /// EXTEND Bid OPCODE
    pub const EXTEND_BID: u8 = 0x03;
}

/// Constants related to the Bid Contract logic.
pub mod contract_constants {
    // TODO: Still waiting for values from the research side.
    // See: https://github.com/dusk-network/rusk/issues/160

    /// t_m in the specs
    /// Represents the time it takes for a Bid from the moment when it's
    /// appended until the moment when it becomes elegible.
    pub const MATURITY_PERIOD: u64 = 0;
    /// t_b in the specs
    /// Represents the ammount of time that takes for a Bid to become
    /// expired from the time it was elegible.
    pub const EXPIRATION_PERIOD: u64 = 10;
    /// t_c in the specs
    /// Represents the time it takes for a Bid to be withrawable from the
    /// time it became expired.
    pub const COOLDOWN_PERIOD: u64 = 0;
    /// Height of the `BidTree` used inside of the BidContract in order to
    /// store the `Bid`s and provide merkle openings to them.
    pub const BID_TREE_DEPTH: usize = 17;
}

/// Alias for `PoseidonTree<BidLeaf, ExpirationAnnotation, BID_TREE_DEPTH>`.
pub type BidTree = PoseidonTree<
    BidLeaf,
    ExpirationAnnotation,
    { contract_constants::BID_TREE_DEPTH },
>;

/// Bid Contract structure. This structure represents the contents of the
/// Bid Contract as well as all of the functions that can be directly called
/// for it.
///
/// This Smart Contract that acts as a decentralized interface to manage blind
/// bids. It is complementary to the Proof Of Blind Bid algorithm used within
/// the Dusk Network consensus.
#[derive(Default, Debug, Clone, Canon)]
pub struct Contract {
    tree: BidTree,
    key_idx_map: KeyToIdxMap,
}

impl Contract {
    /// Generate a new `BidContract` instance.
    pub fn new() -> Self {
        Self {
            tree: BidTree::new(),
            key_idx_map: KeyToIdxMap::new(),
        }
    }

    /// Return a reference to the internal tree.
    pub fn tree(&self) -> &BidTree {
        &self.tree
    }

    /// Return a mutable reference to the internal tree.
    pub fn tree_mut(&mut self) -> &mut BidTree {
        &mut self.tree
    }

    /// Returns a reference to the internal map of the contract.
    pub fn key_idx_map(&self) -> &KeyToIdxMap {
        &self.key_idx_map
    }

    /// Returns a mutable reference to the internal map of the contract.
    pub fn key_idx_map_mut(&mut self) -> &mut KeyToIdxMap {
        &mut self.key_idx_map
    }
}
