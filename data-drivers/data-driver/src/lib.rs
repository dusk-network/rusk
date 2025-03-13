// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types used for interacting with Dusk's transfer and stake contracts.

#![no_std]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![deny(unused_crate_dependencies)]
#![deny(unused_extern_crates)]

extern crate alloc;

mod error;

use alloc::vec::Vec;
use alloc::{format, string::String};

use bytecheck::CheckBytes;
use rkyv::validation::validators::DefaultValidator;
use rkyv::{check_archived_root, Archive, Deserialize, Infallible};

pub use error::Error;
pub use serde_json::Value as JsonValue;

/// A trait for converting between JSON and native RKYV formats in a contract.
///
/// The `ConvertibleContract` trait provides methods for encoding and decoding
/// function inputs, outputs, and events, as well as retrieving the contract's
/// JSON schema.
pub trait ConvertibleContract {
    /// Encodes the input of a function from JSON into the native RKYV format.
    ///
    /// # Parameters
    /// - `fn_name`: The name of the function whose input is being encoded.
    /// - `json`: A JSON string representing the function's input.
    ///
    /// # Returns
    /// - `Ok(Vec<u8>)`: A byte vector containing the serialized RKYV data.
    /// - `Err(Error)`: If encoding fails.
    ///
    /// # Errors
    /// - Returns `Error::Rkyv` if the serialization process fails.
    /// - Returns `Error::Serde` if the input JSON cannot be parsed.
    fn encode_input_fn(
        &self,
        fn_name: &str,
        json: &str,
    ) -> Result<Vec<u8>, Error>;

    /// Decodes the input of a function from the native RKYV format into JSON.
    ///
    /// # Parameters
    /// - `fn_name`: The name of the function whose input is being decoded.
    /// - `rkyv`: A byte slice containing the RKYV-encoded function input.
    ///
    /// # Returns
    /// - `Ok(String)`: A JSON string representation of the function input.
    /// - `Err(Error)`: If decoding fails.
    ///
    /// # Errors
    /// - Returns `Error::Rkyv` if the deserialization process fails.
    /// - Returns `Error::Serde` if the resulting object cannot be serialized to
    ///   JSON.
    fn decode_input_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<JsonValue, Error>;

    /// Decodes the output of a function from the native RKYV format into JSON.
    ///
    /// # Parameters
    /// - `fn_name`: The name of the function whose output is being decoded.
    /// - `rkyv`: A byte slice containing the RKYV-encoded function output.
    ///
    /// # Returns
    /// - `Ok(String)`: A JSON string representation of the function output.
    /// - `Err(Error)`: If decoding fails.
    ///
    /// # Errors
    /// - Returns `Error::Rkyv` if the deserialization process fails.
    /// - Returns `Error::Serde` if the resulting object cannot be serialized to
    ///   JSON.
    fn decode_output_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<JsonValue, Error>;

    /// Decodes an event from the native RKYV format into JSON.
    ///
    /// # Parameters
    /// - `event_name`: The name of the event to be decoded.
    /// - `rkyv`: A byte slice containing the RKYV-encoded event data.
    ///
    /// # Returns
    /// - `Ok(String)`: A JSON string representation of the event data.
    /// - `Err(Error)`: If decoding fails.
    ///
    /// # Errors
    /// - Returns `Error::Rkyv` if the deserialization process fails.
    /// - Returns `Error::Serde` if the resulting object cannot be serialized to
    ///   JSON.
    fn decode_event(
        &self,
        event_name: &str,
        rkyv: &[u8],
    ) -> Result<JsonValue, Error>;

    /// Returns the JSON schema describing the contract's data structure.
    ///
    /// # Returns
    /// - `String`: A JSON string containing the contract's schema definition.
    ///
    /// # Errors
    /// - This function does not return an error.
    fn get_schema(&self) -> String;
}

/// Converts a JSON string into a serialized RKYV archive.
///
/// # Parameters
/// - `json`: A JSON string representing the object to be serialized.
///
/// # Returns
/// - `Ok(Vec<u8>)`: A byte vector containing the serialized RKYV data.
/// - `Err(Error)`: If serialization fails.
///
/// # Type Parameters
/// - `I`: The type of the object being serialized. Must implement:
///   - `serde::de::Deserialize<'a>`: Allows deserialization from JSON.
///   - `rkyv::Archive`: Indicates the type is archivable.
///   - `rkyv::Serialize<rkyv::ser::serializers::AllocSerializer<1024>>`:
///     Enables RKYV serialization.
///
/// # Errors
/// - Returns `serde_json::Error` if JSON deserialization fails.
/// - Returns `Error::Rkyv` if RKYV serialization fails.
pub fn json_to_rkyv<'a, I>(json: &'a str) -> Result<Vec<u8>, Error>
where
    I: serde::de::Deserialize<'a>,
    I: Archive,
    I: rkyv::Serialize<rkyv::ser::serializers::AllocSerializer<1024>>,
{
    let object: I = serde_json::from_str(json)?;
    let rkyv = rkyv::to_bytes(&object)
        .map_err(|e| Error::Rkyv(format!("cannot serialize: {e}")))?
        .to_vec();

    Ok(rkyv)
}

/// Converts a serialized RKYV archive into a JSON string.
///
/// # Parameters
/// - `rkyv`: A byte slice containing the serialized RKYV data.
///
/// # Returns
/// - `Ok(String)`: A JSON representation of the deserialized object.
/// - `Err(Error)`: If deserialization fails.
///
/// # Type Parameters
/// - `T`: The type of the object being deserialized. Must implement:
///   - `serde::ser::Serialize`: Required for JSON conversion.
///   - `rkyv::Archive`: Indicates the type is archivable.
///   - `CheckBytes<DefaultValidator<'a>>`: Ensures safety of archived data.
///   - `Deserialize<T, Infallible>`: Allows deserialization into `T`.
///
/// # Errors
/// - Returns `Error::Rkyv` if:
///   - The archive cannot be validated (`check_archived_root` fails).
///   - Deserialization from RKYV to Rust fails.
/// - Returns `serde_json::Error` if JSON serialization fails.
pub fn rkyv_to_json<T>(rkyv: &[u8]) -> Result<serde_json::Value, Error>
where
    T: serde::ser::Serialize,
    T: Archive,
    for<'a> T::Archived:
        CheckBytes<DefaultValidator<'a>> + Deserialize<T, Infallible>,
{
    let root = check_archived_root::<T>(rkyv)
        .map_err(|e| Error::Rkyv(format!("cannot check_archived_root: {e}")))?;
    let object: T = root
        .deserialize(&mut Infallible)
        .map_err(|e| Error::Rkyv(format!("cannot deserialize: {e}")))?;

    let json = serde_json::to_value(&object)?;

    Ok(json)
}
