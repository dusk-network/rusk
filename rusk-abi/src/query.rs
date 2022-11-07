// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

mod public_input;
pub use public_input::*;

/// The type of the circuit to request a proof verification on.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
#[non_exhaustive]
pub enum CircuitType {
    /// Execute circuit with the given inputs and outputs
    Execute(usize, usize),
    /// Withdraw from contract transparent
    WFCT,
    /// Send to contract transparent
    STCT,
    /// Withdraw from contract obfuscated
    WFCO,
    /// Send to contract obfuscated
    STCO,
}

/// Host query types offered by `rusk`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(not(feature = "host"), doc(hidden))]
pub enum QueryType {
    /// Perform a blake2b hash
    Hash,
    /// Perform a poseidon hash
    PoseidonHash,
    /// Verify a plonk proof
    VerifyProof,
    /// Verify a schnorr signature
    VerifySchnorr,
    /// Verify a BLS signature
    VerifyBls,
}

impl QueryType {
    /// Returns the string representation of the query type
    pub const fn as_str(&self) -> &'static str {
        match self {
            QueryType::Hash => "hash",
            QueryType::PoseidonHash => "poseidon_hash",
            QueryType::VerifyProof => "verify_proof",
            QueryType::VerifySchnorr => "verify_schnorr",
            QueryType::VerifyBls => "verify_bls",
        }
    }
}

/// Metadata types offered by `rusk`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(not(feature = "host"), doc(hidden))]
pub enum MetadataType {
    /// Query for the current block height
    BlockHeight,
}

impl MetadataType {
    /// Returns the string representation of the query type
    pub const fn as_str(&self) -> &'static str {
        match self {
            MetadataType::BlockHeight => "block_height",
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "host")] {
        mod host;
        pub use host::*;
    } else {
        mod hosted;
        pub use hosted::*;
    }
}
