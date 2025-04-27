// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # JSON-RPC Key Models
//!
//! This module contains models for key-related types, including BLS public keys
//! and their serialization/deserialization as Base58 strings.
//!
//! ## Key Types
//!
//! - [`AccountPublicKey`]: A wrapper around
//!   `dusk_core::signatures::bls::PublicKey` that provides custom `serde`
//!   serialization and deserialization as a Base58 string.
//!
//! ## Serialization/Deserialization Details
//!
//! The `AccountPublicKey` type implements `serde::Serialize` and
//! `serde::Deserialize` for Base58 encoding/decoding.
//!
//! - `serialize` converts the `BlsPublicKey` to a byte array and encodes it
//!   using Base58.
//! - `deserialize` decodes a Base58 string back into a byte array and converts
//!   it to a `BlsPublicKey`.
//!
//! ## Error Handling
//!
//! - If the input string is not valid Base58, `deserialize` returns a
//!   `serde::de::Error`.
//! - If the decoded bytes do not have the correct length for a BLS public key
//!   (`BlsPublicKey::SIZE`), `deserialize` returns an error.

use dusk_bytes::Serializable;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use serde::{de, Deserializer, Serializer};

/// A wrapper around `dusk_core::signatures::bls::PublicKey` that provides
/// custom `serde` serialization and deserialization as a Base58 string.
///
/// This is used throughout the JSON-RPC models where a BLS public key needs to
/// be represented in the standard Base58 format.
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
