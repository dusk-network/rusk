// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! ![Build Status](https://github.com/dusk-network/rusk/workflows/Continuous%20integration/badge.svg)
//! [![Repository](https://img.shields.io/badge/github-rusk-blueviolet?logo=github)](https://github.com/dusk-network/rusk)
//! [![Documentation](https://img.shields.io/badge/docs-rusk--abi-blue?logo=rust)](https://docs.rs/rusk-abi/)

//! # Rusk ABI
//!
//! The ABI to develop Rusk's specific Contracts

#![warn(missing_docs)]
#![no_std]
#![deny(clippy::all)]

pub use dusk_abi::ContractId;

/// Constant depth of the merkle tree that provides the opening proofs.
pub const POSEIDON_TREE_DEPTH: usize = 17;

/// Label used for the ZK transcript initialization. Must be the same for prover
/// and verifier.
pub const TRANSCRIPT_LABEL: &[u8] = b"dusk-network";

/// Contract ID of the genesis transfer contract
pub const fn transfer_contract() -> ContractId {
    ContractId::reserved(0x1)
}

/// Contract ID of the genesis stake contract
pub const fn stake_contract() -> ContractId {
    ContractId::reserved(0x2)
}

#[doc(hidden)]
pub mod hash;

cfg_if::cfg_if! {
    if #[cfg(feature = "module")] {
        #[doc(hidden)]
        pub mod module;
        pub use module::*;
    }
}
