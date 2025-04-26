// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # Rusk JSON-RPC API Models
//!
//! This module defines the core data structures used for serialization and
//! deserialization within the Rusk JSON-RPC API.
//!
//! ## Overview
//!
//! The primary purpose of this module and its submodules is to provide Rust
//! representations of the JSON objects described in the Rusk JSON-RPC
//! specification. These models act as the boundary layer between the internal
//! Rusk node data types and the external JSON format consumed by clients.
//!
//! ## Key Features:
//!
//! - **Specification Alignment:** Structures are designed to closely match the
//!   fields and formats specified in the official JSON-RPC documentation.
//! - **Serialization/Deserialization:** All models derive `serde::Serialize`
//!   and `serde::Deserialize` to enable straightforward conversion to and from
//!   JSON.
//! - **Data Conversion:** Many models provide `From` implementations to
//!   facilitate conversion from internal `node_data` types (e.g.,
//!   `node_data::ledger::Block`) into the corresponding RPC model (e.g.,
//!   `model::block::Block`).
//! - **Custom Serialization:** The `serde_helper` submodule provides utilities
//!   for custom serialization logic where needed (e.g., serializing large `u64`
//!   values as JSON strings).
//! - **Modularity:** Models are organized into submodules based on their domain
//!   (e.g., `block`, `transaction`, `consensus`).
//!
//! ## Submodules:
//!
//! - [`block`]: Models related to blocks, headers, status, and faults.
//! - [`chain`]: Models for overall blockchain statistics.
//! - [`consensus`]: Models representing consensus outcomes (e.g., validation
//!   results).
//! - [`contract`]: Placeholder for contract-related models (if any).
//! - [`mempool`]: Models for mempool information.
//! - [`network`]: Models for network peer metrics.
//! - [`prover`]: Placeholder for prover-related models (if any).
//! - [`provisioner`]: Models related to provisioner information (stakes, etc.).
//! - [`serde_helper`]: Utility functions for custom `serde` serialization.
//! - [`subscription`]: Placeholder for WebSocket subscription models (if any).
//! - [`transaction`]: Models for transactions, status, types, events, and
//!   simulation results.

use dusk_bytes::Serializable;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use serde::{de, Deserializer, Serializer};

pub mod block;
pub mod chain;
pub mod consensus;
pub mod contract;
pub mod mempool;
pub mod network;
pub mod prover;
pub mod provisioner;
pub mod serde_helper;
pub mod subscription;
pub mod transaction;

/// A wrapper around `dusk_core::signatures::bls::PublicKey` that provides
/// custom `serde` serialization and deserialization as a Base58 string.
///
/// This is used throughout the JSON-RPC models where a BLS public key needs to
/// be represented in the standard Base58 format.
///
/// # Examples
///
/// ```
/// # use dusk_core::signatures::bls::PublicKey;
/// # use dusk_bytes::Serializable;
/// # use serde::{Serialize, Deserialize};
/// use rusk::jsonrpc::model::AccountPublicKey;
///
/// // Assume `bls_pk` is a valid BlsPublicKey instance
/// let bls_pk_bytes = [0u8; 96]; // Example byte array (needs valid key bytes)
/// # let bls_pk = PublicKey::from_bytes(&bls_pk_bytes).unwrap();
///
/// let account_pk = AccountPublicKey(bls_pk);
///
/// // Serialize to Base58 string
/// let json_string = serde_json::to_string(&account_pk).unwrap();
/// println!("Serialized: {}", json_string);
/// // Example output: ""1111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111"" (for zero bytes)
///
/// // Deserialize from Base58 string
/// let deserialized: AccountPublicKey = serde_json::from_str(&json_string).unwrap();
/// assert_eq!(account_pk, deserialized);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountPublicKey(pub BlsPublicKey);

/// Serializes the `AccountPublicKey` into a Base58 encoded string.
impl serde::Serialize for AccountPublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = self.0.to_bytes();
        let b58_string = bs58::encode(bytes).into_string();
        serializer.serialize_str(&b58_string)
    }
}

/// Deserializes a Base58 encoded string into an `AccountPublicKey`.
///
/// # Errors
///
/// Returns a `serde::de::Error` if:
/// - The input string is not valid Base58.
/// - The decoded bytes do not have the correct length for a BLS public key
///   (`BlsPublicKey::SIZE`).
/// - The decoded bytes do not represent a valid point on the BLS curve.
impl<'de> serde::Deserialize<'de> for AccountPublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let b58_string = String::deserialize(deserializer)?;
        let bytes = bs58::decode(&b58_string)
            .into_vec()
            .map_err(|e| de::Error::custom(format!("Invalid base58: {}", e)))?;

        // Ensure the byte length is correct. BlsPublicKey::SIZE is 96.
        if bytes.len() != BlsPublicKey::SIZE {
            return Err(de::Error::invalid_length(
                bytes.len(),
                &format!("a byte array of size {}", BlsPublicKey::SIZE)
                    .as_str(),
            ));
        }

        // BlsPublicKey doesn't implement DeserializableSlice directly,
        // but `from_bytes` expects a fixed-size array.
        // We can try converting the Vec<u8> into [u8; SIZE].
        let byte_array: [u8; BlsPublicKey::SIZE] =
            bytes.try_into().map_err(|_| {
                de::Error::custom("Failed to convert Vec<u8> to array")
            })?; // Should not fail due to length check

        // Use `from_bytes` which takes the array
        let pk = BlsPublicKey::from_bytes(&byte_array).map_err(|e| {
            de::Error::custom(format!("Invalid public key bytes: {:?}", e))
        })?;

        Ok(AccountPublicKey(pk))
    }
}
