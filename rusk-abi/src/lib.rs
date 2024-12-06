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
#![cfg_attr(not(target_family = "wasm"), deny(unused_crate_dependencies))]
#![deny(unused_extern_crates)]

#[cfg(all(feature = "host", feature = "abi"))]
compile_error!("features \"host\" and \"abi\" are mutually exclusive");

extern crate alloc;

#[cfg(feature = "debug")]
pub use piecrust_uplink::debug as piecrust_debug;

#[cfg(feature = "abi")]
mod abi;
#[cfg(feature = "abi")]
pub use abi::{
    block_height, chain_id, hash, owner, owner_raw, poseidon_hash, self_owner,
    self_owner_raw, verify_bls, verify_bls_multisig, verify_groth16_bn254,
    verify_plonk, verify_schnorr,
};

#[cfg(feature = "abi")]
pub use piecrust_uplink::{
    call,
    call_raw,
    call_raw_with_limit,
    call_with_limit,
    caller,
    emit,
    emit_raw,
    feed,
    limit,
    self_id,
    spent,
    wrap_call,
    wrap_call_unchecked, // maybe use for our Transaction in spend_and_execute
};

#[cfg(feature = "host")]
mod host;
#[cfg(feature = "host")]
pub use host::{
    hash, new_ephemeral_vm, new_genesis_session, new_session, new_vm,
    poseidon_hash, verify_bls, verify_bls_multisig, verify_plonk,
    verify_schnorr,
};
#[cfg(feature = "host")]
pub use piecrust::{
    CallReceipt, CallTree, CallTreeElem, CommitRoot, ContractData,
    Error as PiecrustError, PageOpening, Session, VM,
};

#[cfg(any(feature = "host", feature = "abi"))]
enum Metadata {}

#[cfg(any(feature = "host", feature = "abi"))]
impl Metadata {
    pub const CHAIN_ID: &'static str = "chain_id";
    pub const BLOCK_HEIGHT: &'static str = "block_height";
}

#[cfg(any(feature = "host", feature = "abi"))]
enum Query {}

#[cfg(any(feature = "host", feature = "abi"))]
impl Query {
    pub const HASH: &'static str = "hash";
    pub const POSEIDON_HASH: &'static str = "poseidon_hash";
    pub const VERIFY_PLONK: &'static str = "verify_plonk";
    pub const VERIFY_GROTH16_BN254: &'static str = "verify_groth16_bn254";
    pub const VERIFY_SCHNORR: &'static str = "verify_schnorr";
    pub const VERIFY_BLS: &'static str = "verify_bls";
    pub const VERIFY_BLS_MULTISIG: &'static str = "verify_bls_multisig";
}

#[cfg(test)]
mod tests {
    // rust doesn't allow for optional dev-dependencies so we need to add this
    // work-around to satisfy the `unused_crate_dependencies` lint
    use ff as _;
    use once_cell as _;
    use rand as _;
}
