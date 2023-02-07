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
//! The ABI to develop Dusk Network smart contracts

#![no_std]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![feature(const_fn_floating_point_arithmetic)]

use dusk_bls12_381::BlsScalar;
use dusk_bytes::DeserializableSlice;

pub use piecrust_uplink::ModuleId;
use piecrust_uplink::MODULE_ID_BYTES;

/// Constant depth of the merkle tree that provides the opening proofs.
pub const POSEIDON_TREE_DEPTH: usize = 17;

/// Label used for the ZK transcript initialization. Must be the same for prover
/// and verifier.
pub const TRANSCRIPT_LABEL: &[u8] = b"dusk-network";

/// Module ID of the genesis transfer contract
pub const fn transfer_module() -> ModuleId {
    reserved(0x1)
}

/// Module ID of the genesis stake contract
pub const fn stake_module() -> ModuleId {
    reserved(0x2)
}

#[inline]
const fn reserved(b: u8) -> ModuleId {
    let mut bytes = [0u8; MODULE_ID_BYTES];
    bytes[0] = b;
    ModuleId::from_bytes(bytes)
}

/// Converts a `ModuleId` to a `BlsScalar`
///
/// This cannot fail since the contract id should be generated always using
/// `rusk_abi::gen_module_id` that ensures the bytes are inside the BLS field.
pub fn module_to_scalar(module_id: &ModuleId) -> BlsScalar {
    BlsScalar::from_slice(module_id.as_bytes())
        .expect("Something went REALLY wrong if a contract id is not a scalar")
}

pub mod dusk;
#[doc(hidden)]
pub mod hash;

mod query;
pub use query::*;
