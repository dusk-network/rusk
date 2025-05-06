// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Serde helper functions for JSON-RPC model serialization.

/// Serialize u64 as a string.
///
/// This module provides serialization and deserialization utilities for `u64`
/// values where the `u64` is represented as a string in JSON. This is useful
/// for ensuring compatibility with systems that require numeric values to be
/// encoded as strings in JSON.
///
/// # Purpose
/// - Serialize `u64` into a JSON string.
/// - Deserialize a JSON string back into a `u64`.
///
/// # Usage
/// To use this module, annotate your `u64` field with:
/// ```ignore
/// #[serde(with = "rusk::jsonrpc::model::serde_helper::u64_to_string")]
/// ```
///
/// # Example
/// ```rust
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Example {
///     #[serde(with = "rusk::jsonrpc::model::serde_helper::u64_to_string")]
///     pub value: u64,
/// }
///
/// let example = Example { value: 42 };
/// let json = serde_json::to_string(&example).unwrap();
/// assert_eq!(json, r#"{"value":"42"}"#);
///
/// let deserialized: Example = serde_json::from_str(&json).unwrap();
/// assert_eq!(deserialized.value, 42);
/// ```
pub mod u64_to_string {
    use serde::Serializer;

    /// Serializes a `u64` into a JSON string.
    ///
    /// # Arguments
    /// - `num`: A reference to the `u64` value to serialize.
    /// - `serializer`: The serializer to use for the operation.
    ///
    /// # Returns
    /// A serialized JSON string representing the `u64` value.
    ///
    /// # Errors
    /// Returns an error if the serialization process fails.
    pub fn serialize<S>(num: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&num.to_string())
    }

    // Deserialization from String back to u64.
    use serde::{Deserialize, Deserializer};
    use std::str::FromStr;

    /// Deserializes a JSON string into a `u64`.
    ///
    /// # Arguments
    /// - `deserializer`: The deserializer to use for the operation.
    ///
    /// # Returns
    /// A `u64` reconstructed from the JSON string.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The JSON string cannot be parsed as a `u64`.
    /// - The deserialization process encounters any other issues.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        u64::from_str(&s).map_err(serde::de::Error::custom)
    }
}

/// Serialize `Option<u64>` as `Option<String>`.
///
/// This module provides serialization and deserialization utilities for
/// `Option<u64>` values where the `u64` is represented as a string in JSON.
/// This is useful for ensuring compatibility with systems that require numeric
/// values to be encoded as strings in JSON, while also handling optional
/// values.
///
/// # Purpose
/// - Serialize `Option<u64>` into a JSON string or `null` for `None`.
/// - Deserialize a JSON string or `null` back into an `Option<u64>`.
///
/// # Usage
/// To use this module, annotate your `Option<u64>` field with:
/// ```ignore
/// #[serde(with = "rusk::jsonrpc::model::serde_helper::opt_u64_to_string")]
/// ```
///
/// # Example
/// ```rust
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Example {
///     #[serde(with = "rusk::jsonrpc::model::serde_helper::opt_u64_to_string")]
///     pub value: Option<u64>,
/// }
///
/// let example = Example { value: Some(42) };
/// let json = serde_json::to_string(&example).unwrap();
/// assert_eq!(json, r#"{"value":"42"}"#);
///
/// let deserialized: Example = serde_json::from_str(&json).unwrap();
/// assert_eq!(deserialized.value, Some(42));
///
/// let none_example = Example { value: None };
/// let none_json = serde_json::to_string(&none_example).unwrap();
/// assert_eq!(none_json, r#"{"value":null}"#);
///
/// let deserialized_none: Example = serde_json::from_str(&none_json).unwrap();
/// assert_eq!(deserialized_none.value, None);
/// ```
pub mod opt_u64_to_string {
    use serde::Serializer;

    /// Serializes an `Option<u64>` into a JSON string or `null` for `None`.
    ///
    /// # Arguments
    /// - `opt_num`: A reference to the `Option<u64>` value to serialize.
    /// - `serializer`: The serializer to use for the operation.
    ///
    /// # Returns
    /// A serialized JSON string representing the `u64` value or `null` for
    /// `None`.
    ///
    /// # Errors
    /// Returns an error if the serialization process fails.
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

    /// Deserializes a JSON string or `null` into an `Option<u64>`.
    ///
    /// # Arguments
    /// - `deserializer`: The deserializer to use for the operation.
    ///
    /// # Returns
    /// An `Option<u64>` reconstructed from the JSON string or `null`.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The JSON string cannot be parsed as a `u64`.
    /// - The deserialization process encounters any other issues.
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
///
/// This module provides serialization and deserialization utilities for
/// `Vec<u8>` values where the binary data is represented as a Base64-encoded
/// string in JSON. This is useful for ensuring compatibility with systems that
/// require binary data to be encoded as strings in JSON.
///
/// # Purpose
/// - Serialize `Vec<u8>` into a Base64-encoded JSON string.
/// - Deserialize a Base64-encoded JSON string back into a `Vec<u8>`.
///
/// # Usage
/// To use this module, annotate your `Vec<u8>` field with:
/// ```ignore
/// #[serde(with = "rusk::jsonrpc::model::serde_helper::base64_vec_u8")]
/// ```
///
/// # Example
/// ```rust
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Example {
///     #[serde(with = "rusk::jsonrpc::model::serde_helper::base64_vec_u8")]
///     pub data: Vec<u8>,
/// }
///
/// let example = Example { data: vec![1, 2, 3, 4] };
/// let json = serde_json::to_string(&example).unwrap();
/// assert_eq!(json, r#"{"data":"AQIDBA=="}"#);
///
/// let deserialized: Example = serde_json::from_str(&json).unwrap();
/// assert_eq!(deserialized.data, vec![1, 2, 3, 4]);
/// ```
pub mod base64_vec_u8 {
    use serde::{Deserialize, Deserializer, Serializer};
    // Import the Engine trait to get access to the encode/decode methods
    use base64::Engine as _;

    /// Serializes a `Vec<u8>` into a Base64-encoded JSON string.
    ///
    /// # Arguments
    /// - `bytes`: A reference to the `Vec<u8>` value to serialize.
    /// - `serializer`: The serializer to use for the operation.
    ///
    /// # Returns
    /// A serialized Base64-encoded JSON string representing the `Vec<u8>`.
    ///
    /// # Errors
    /// Returns an error if the serialization process fails.
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

    /// Deserializes a Base64-encoded JSON string into a `Vec<u8>`.
    ///
    /// # Arguments
    /// - `deserializer`: The deserializer to use for the operation.
    ///
    /// # Returns
    /// A `Vec<u8>` reconstructed from the Base64-encoded JSON string.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The JSON string cannot be decoded as Base64.
    /// - The deserialization process encounters any other issues.
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

/// Serialize/deserialize `HashMap<String, u64>` to/from JSON object where `u64`
/// values are represented as strings.
///
/// This module provides serialization and deserialization utilities for
/// `HashMap<String, u64>` where the `u64` values are represented as strings
/// in JSON. This is useful for values that can exceed JavaScript's
/// `Number.MAX_SAFE_INTEGER` (2^53 - 1).
///
/// # Purpose
/// - Serialize `HashMap<String, u64>` into a JSON object where `u64` values are
///   represented as strings.
/// - Deserialize such JSON objects back into `HashMap<String, u64>`.
///
/// # Usage
/// To use this module, annotate your `HashMap<String, u64>` field with:
/// ```ignore
/// #[serde(with = "rusk::jsonrpc::model::serde_helper::string_u64_map_as_strings")]
/// ```
///
/// # Example
/// ```rust
/// use serde::{Serialize, Deserialize};
/// use std::collections::HashMap;
///
/// #[derive(Serialize, Deserialize)]
/// struct Example {
///     #[serde(with = "rusk::jsonrpc::model::serde_helper::string_u64_map_as_strings")]
///     pub data: HashMap<String, u64>,
/// }
///
/// let mut map = HashMap::new();
/// map.insert("key1".to_string(), 42);
/// map.insert("key2".to_string(), 100);
///
/// let example = Example { data: map };
/// let json = serde_json::to_string(&example).unwrap();
///
/// // HashMap iteration order is not guaranteed
/// let expected_jsons = [
///     r#"{"data":{"key1":"42","key2":"100"}}"#,
///     r#"{"data":{"key2":"100","key1":"42"}}"#
/// ];
/// assert!(expected_jsons.contains(&json.as_str()), "Unexpected JSON output: {}", json);
///
/// let deserialized: Example = serde_json::from_str(&json).unwrap();
/// assert_eq!(deserialized.data["key1"], 42);
/// assert_eq!(deserialized.data["key2"], 100);
/// ```
pub mod string_u64_map_as_strings {
    use serde::de;
    use serde::ser::SerializeMap;
    use serde::{Deserializer, Serializer};
    use std::collections::HashMap;
    use std::fmt;

    /// Serializes a `HashMap<String, u64>` into a JSON object where the `u64`
    /// values are represented as strings.
    ///
    /// # Arguments
    /// - `map`: A reference to the `HashMap<String, u64>` to serialize.
    /// - `serializer`: The serializer to use for the operation.
    ///
    /// # Returns
    /// A serialized JSON object with stringified `u64` values.
    ///
    /// # Errors
    /// Returns an error if the serialization process fails.
    pub fn serialize<S>(
        map: &HashMap<String, u64>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut json_object = serializer.serialize_map(Some(map.len()))?;
        for (key, value) in map {
            json_object.serialize_entry(key, &value.to_string())?;
        }
        json_object.end()
    }

    /// Deserializes a JSON object into a `HashMap<String, u64>` where the
    /// values are parsed from strings.
    ///
    /// # Arguments
    /// - `deserializer`: The deserializer to use for the operation.
    ///
    /// # Returns
    /// A `HashMap<String, u64>` reconstructed from the JSON object.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The JSON object contains values that cannot be parsed as `u64`.
    /// - The deserialization process encounters any other issues.
    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<String, u64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct JsonObjectVisitor;

        impl<'de> de::Visitor<'de> for JsonObjectVisitor {
            type Value = HashMap<String, u64>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map of string keys to string values")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: de::MapAccess<'de>,
            {
                let mut record: HashMap<String, u64> = HashMap::new();
                while let Some((key, value)) =
                    map.next_entry::<String, String>()?
                {
                    let value = value.parse::<u64>().map_err(|e| {
                        de::Error::custom(format!(
                            "Invalid u64 value '{}' for key '{}': {}",
                            value, key, e
                        ))
                    })?;
                    record.insert(key, value);
                }
                Ok(record)
            }
        }

        deserializer.deserialize_map(JsonObjectVisitor)
    }
}
