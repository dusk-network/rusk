// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Serde helper functions for JSON-RPC model serialization.

/// Serialize u64 as a string.
pub mod u64_to_string {
    use serde::Serializer;

    pub fn serialize<S>(num: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&num.to_string())
    }

    // Deserialization from String back to u64.
    use serde::{Deserialize, Deserializer};
    use std::str::FromStr;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        u64::from_str(&s).map_err(serde::de::Error::custom)
    }
}

/// Serialize Option<u64> as Option<String>.
pub mod opt_u64_to_string {
    use serde::Serializer;

    pub fn serialize<S>(
        opt_num: &Option<u64>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match opt_num {
            Some(num) => serializer.serialize_some(&num.to_string()),
            None => serializer.serialize_none(),
        }
    }

    // Deserialization from Option<String> back to Option<u64>.
    use serde::{Deserialize, Deserializer};
    use std::str::FromStr;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<String>::deserialize(deserializer)?
            .map(|s| u64::from_str(&s).map_err(serde::de::Error::custom))
            .transpose()
    }
}

/// Serialize/deserialize `Vec<u8>` to/from Base64 string.
pub mod base64_vec_u8 {
    use serde::{Deserialize, Deserializer, Serializer};
    // Import the Engine trait to get access to the encode/decode methods
    use base64::Engine as _;

    /// Serializes `Vec<u8>` to a Base64 string using the standard engine.
    pub fn serialize<S>(
        bytes: &Vec<u8>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Use the engine's encode method via the Engine trait
        serializer.serialize_str(
            &base64::engine::general_purpose::STANDARD.encode(bytes),
        )
    }

    /// Deserializes a Base64 string into `Vec<u8>` using the standard engine.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // Use the engine's decode method via the Engine trait
        base64::engine::general_purpose::STANDARD
            .decode(s.as_bytes())
            .map_err(serde::de::Error::custom)
    }
}
