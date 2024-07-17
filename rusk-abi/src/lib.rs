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

pub use piecrust_uplink::{
    ContractError, ContractId, Event, StandardBufSerializer, ARGBUF_LEN,
    CONTRACT_ID_BYTES,
};

#[cfg(feature = "debug")]
pub use piecrust_uplink::debug as piecrust_debug;

#[cfg(feature = "abi")]
mod abi;
#[cfg(feature = "abi")]
pub use abi::{
    block_height, hash, owner, owner_raw, poseidon_hash, self_owner,
    self_owner_raw, verify_bls, verify_proof, verify_schnorr,
};
#[cfg(feature = "abi")]
pub use piecrust_uplink::{
    call,
    call_raw,
    call_raw_with_limit,
    call_with_limit,
    caller,
    emit,
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
    poseidon_hash, verify_bls, verify_proof, verify_schnorr,
};
#[cfg(feature = "host")]
pub use piecrust::{
    CallReceipt, CallTree, CallTreeElem, ContractData, Error as PiecrustError,
    PageOpening, Session, VM,
};

pub mod dusk;

use dusk_bytes::DeserializableSlice;
use execution_core::BlsScalar;

/// Label used for the ZK transcript initialization. Must be the same for prover
/// and verifier.
pub const TRANSCRIPT_LABEL: &[u8] = b"dusk-network";

/// ID of the genesis transfer contract
pub const TRANSFER_CONTRACT: ContractId = reserved(0x1);
/// ID of the genesis stake contract
pub const STAKE_CONTRACT: ContractId = reserved(0x2);
/// ID of the genesis license contract
pub const LICENSE_CONTRACT: ContractId = reserved(0x3);

enum Metadata {}

#[allow(dead_code)]
impl Metadata {
    pub const BLOCK_HEIGHT: &'static str = "block_height";
}

enum Query {}

#[allow(dead_code)]
impl Query {
    pub const HASH: &'static str = "hash";
    pub const POSEIDON_HASH: &'static str = "poseidon_hash";
    pub const VERIFY_PROOF: &'static str = "verify_proof";
    pub const VERIFY_SCHNORR: &'static str = "verify_schnorr";
    pub const VERIFY_BLS: &'static str = "verify_bls";
}

#[inline]
const fn reserved(b: u8) -> ContractId {
    let mut bytes = [0u8; CONTRACT_ID_BYTES];
    bytes[0] = b;
    ContractId::from_bytes(bytes)
}

/// Generate a [`ContractId`] address from the given slice of bytes, that is
/// also a valid [`BlsScalar`]
pub fn gen_contract_id(bytes: &[u8]) -> ContractId {
    ContractId::from_bytes(BlsScalar::hash_to_scalar(bytes).into())
}

/// Converts a `ContractId` to a `BlsScalar`
///
/// This cannot fail since the contract id should be generated always using
/// `rusk_abi::gen_module_id` that ensures the bytes are inside the BLS field.
pub fn contract_to_scalar(module_id: &ContractId) -> BlsScalar {
    BlsScalar::from_slice(module_id.as_bytes())
        .expect("Something went REALLY wrong if a contract id is not a scalar")
}
