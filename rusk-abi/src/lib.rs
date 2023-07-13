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

#![cfg_attr(not(feature = "host"), no_std)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![feature(const_fn_floating_point_arithmetic)]

#[cfg(all(feature = "host", feature = "abi"))]
compile_error!("features \"host\" and \"abi\" are mutually exclusive");

extern crate alloc;

mod types;
pub use types::*;

mod abi;
pub use abi::*;

#[cfg(feature = "host")]
mod host;
#[cfg(feature = "host")]
pub use host::*;

pub mod dusk;
#[doc(hidden)]
pub mod hash;

use hash::Hasher;

// re-export `piecrust-uplink` such that `rusk-abi` is the only crate
pub use piecrust_uplink::*;

use dusk_bls12_381::BlsScalar;
use dusk_bytes::DeserializableSlice;

/// Constant depth of the merkle tree that provides the opening proofs.
pub const POSEIDON_TREE_DEPTH: usize = 17;

/// Label used for the ZK transcript initialization. Must be the same for prover
/// and verifier.
pub const TRANSCRIPT_LABEL: &[u8] = b"dusk-network";

/// ID of the genesis transfer contract
pub const TRANSFER_CONTRACT: ContractId = reserved(0x1);
/// ID of the genesis stake contract
pub const STAKE_CONTRACT: ContractId = reserved(0x2);
/// ID of the genesis license contract
pub const LICENSE_CONTRACT: ContractId = reserved(0x3);

#[inline]
const fn reserved(b: u8) -> ContractId {
    let mut bytes = [0u8; CONTRACT_ID_BYTES];
    bytes[0] = b;
    ContractId::from_bytes(bytes)
}

/// Generate a [`ContractId`] address from the given slice of bytes, that is
/// also a valid [`BlsScalar`]
pub fn gen_contract_id(bytes: &[u8]) -> ContractId {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    ContractId::from_bytes(hasher.output())
}

/// Converts a `ContractId` to a `BlsScalar`
///
/// This cannot fail since the contract id should be generated always using
/// `rusk_abi::gen_module_id` that ensures the bytes are inside the BLS field.
pub fn contract_to_scalar(module_id: &ContractId) -> BlsScalar {
    BlsScalar::from_slice(module_id.as_bytes())
        .expect("Something went REALLY wrong if a contract id is not a scalar")
}
