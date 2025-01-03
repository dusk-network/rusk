// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Core value types for the RUES protocol.
//!
//! This module provides the fundamental value types that represent data
//! exchanged over WebSocket connections in the RUES protocol. These types
//! are transport-agnostic and focus purely on data representation and
//! serialization:
//! - `RuesValue` - Core enum for all possible RUES values
//! - Build and parse RUES protocol messages
//!
//! # Value Types
//!
//! The system supports several value types:
//! - Binary data (raw bytes)
//! - JSON data
//! - Plain text
//! - GraphQL queries/responses
//! - Zero-knowledge proofs
//!
//! # Examples
//!
//! Creating and using values:
//! ```rust
//! use rusk::http::domain::types::value::RuesValue;
//! use serde_json::json;
//! use bytes::Bytes;
//!
//! // Create different value types
//! let binary = RuesValue::Binary(Bytes::from(vec![1, 2, 3]));
//! let json = RuesValue::Json(json!({"key": "value"}));
//! let text = RuesValue::Text("Hello".into());
//! ```

use bytes::{BufMut, Bytes, BytesMut};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;

use crate::http::domain::error::{DomainError, SerDeError};
use crate::http::domain::types::headers::RuesHeaders;

/// Core value types in the RUES system.
///
/// Represents all possible value types that can be transmitted in RUES
/// messages:
/// - `Binary` - Raw binary data
/// - `Json` - JSON-formatted data
/// - `Text` - Plain text data
/// - `GraphQL` - GraphQL queries and responses
/// - `Proof` - Zero-knowledge proofs
///
/// Values maintain ordering and can be used in sorted collections. The ordering
/// is: Binary < Json < Text < GraphQL < Proof
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::types::value::RuesValue;
/// use serde_json::json;
/// use bytes::Bytes;
///
/// // Create binary value
/// let binary = RuesValue::Binary(Bytes::from(vec![1, 2, 3]));
///
/// // Create JSON value
/// let json = RuesValue::Json(json!({
///     "key": "value",
///     "array": [1, 2, 3]
/// }));
///
/// // Create text value
/// let text = RuesValue::Text("Hello, RUES!".into());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuesValue {
    /// Raw binary data
    Binary(Bytes),
    /// JSON data
    Json(JsonValue),
    /// Plain text
    Text(String),
    /// GraphQL query/response
    GraphQL(String),
    /// Zero-knowledge proof
    Proof(Bytes),
}

impl RuesValue {
    /// Serializes this value to bytes using the RUES binary format.
    ///
    /// The binary format consists of:
    /// ```text
    /// [1 byte tag][4 bytes length][payload]
    /// ```
    ///
    /// Tags:
    /// - 0: Binary
    /// - 1: JSON
    /// - 2: Text
    /// - 3: GraphQL
    /// - 4: Proof
    ///
    /// # Returns
    /// * `Ok(Bytes)` - Serialized value
    /// * `Err(DomainError)` - If serialization fails
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::types::value::RuesValue;
    /// use bytes::Bytes;
    ///
    /// let value = RuesValue::Binary(Bytes::from(vec![1, 2, 3]));
    /// let bytes = value.to_bytes().unwrap();
    /// ```
    pub fn to_bytes(&self) -> Result<Bytes, DomainError> {
        let mut buffer = BytesMut::new();

        // Write type tag
        let tag: u8 = match self {
            Self::Binary(_) => 0,
            Self::Json(_) => 1,
            Self::Text(_) => 2,
            Self::GraphQL(_) => 3,
            Self::Proof(_) => 4,
        };
        buffer.put_u8(tag);

        // Write payload with length prefix
        match self {
            Self::Binary(bytes) | Self::Proof(bytes) => {
                buffer.put_u32_le(bytes.len() as u32);
                buffer.extend_from_slice(bytes);
            }
            Self::Json(json) => {
                let json_bytes = serde_json::to_vec(json)
                    .map_err(|e| SerDeError::Json(e))?;
                buffer.put_u32_le(json_bytes.len() as u32);
                buffer.extend_from_slice(&json_bytes);
            }
            Self::Text(text) | Self::GraphQL(text) => {
                let text_bytes = text.as_bytes();
                buffer.put_u32_le(text_bytes.len() as u32);
                buffer.extend_from_slice(text_bytes);
            }
        }

        Ok(buffer.freeze())
    }

    /// Deserializes bytes into a RuesValue.
    ///
    /// Expects data in RUES binary format:
    /// ```text
    /// [1 byte tag][4 bytes length][payload]
    /// ```
    ///
    /// # Arguments
    /// * `bytes` - Data in RUES binary format
    ///
    /// # Returns
    /// * `Ok(RuesValue)` - Deserialized value
    /// * `Err(DomainError)` - If data is invalid or incomplete
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::types::value::RuesValue;
    ///
    /// // Assume we have some valid RUES binary data
    /// let data = vec![0, 3, 0, 0, 0, 1, 2, 3];
    /// let value = RuesValue::from_bytes(&data).unwrap();
    /// ```
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DomainError> {
        if bytes.len() < 5 {
            return Err(SerDeError::MissingField(
                "Buffer too short for header".to_string(),
            )
            .into());
        }

        let tag = bytes[0];
        let payload_len =
            u32::from_le_bytes(bytes[1..5].try_into().map_err(|_| {
                SerDeError::MissingField("Invalid length bytes".to_string())
            })?) as usize;

        if bytes.len() < 5 + payload_len {
            return Err(SerDeError::MissingField(
                "Buffer too short for payload".to_string(),
            )
            .into());
        }

        let payload = &bytes[5..5 + payload_len];

        match tag {
            0 => Ok(Self::Binary(Bytes::copy_from_slice(payload))),
            1 => {
                let json = serde_json::from_slice(payload)
                    .map_err(|e| SerDeError::Json(e))?;
                Ok(Self::Json(json))
            }
            2 => {
                let text =
                    String::from_utf8(payload.to_vec()).map_err(|_| {
                        SerDeError::MissingField(
                            "Invalid UTF-8 in text".to_string(),
                        )
                    })?;
                Ok(Self::Text(text))
            }
            3 => {
                let query =
                    String::from_utf8(payload.to_vec()).map_err(|_| {
                        SerDeError::MissingField(
                            "Invalid UTF-8 in GraphQL query".to_string(),
                        )
                    })?;
                Ok(Self::GraphQL(query))
            }
            4 => Ok(Self::Proof(Bytes::copy_from_slice(payload))),
            _ => Err(SerDeError::MissingField(format!("Invalid tag: {}", tag))
                .into()),
        }
    }

    /// Creates a complete RUES message by combining headers and value.
    ///
    /// The resulting message format is:
    /// ```text
    /// [4 bytes header length][JSON headers][value bytes]
    /// ```
    ///
    /// # Arguments
    /// * `headers` - Message headers
    ///
    /// # Returns
    /// * `Ok(Bytes)` - Complete message
    /// * `Err(DomainError)` - If message creation fails
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::types::value::RuesValue;
    /// use rusk::http::domain::types::headers::RuesHeaders;
    /// use serde_json::json;
    ///
    /// let value = RuesValue::Json(json!({"test": true}));
    /// let headers = RuesHeaders::builder()
    ///     .content_location("/on/blocks/accepted")
    ///     .content_type("application/json")
    ///     .content_length(42)
    ///     .build()
    ///     .unwrap();
    ///
    /// let message = value.to_message_bytes(&headers).unwrap();
    /// ```
    pub fn to_message_bytes(
        &self,
        headers: &RuesHeaders,
    ) -> Result<Bytes, DomainError> {
        let mut buffer = BytesMut::new();

        // Get headers bytes
        let headers_bytes = headers.to_bytes()?;

        // Write headers length
        buffer.put_u32_le(headers_bytes.len() as u32);

        // Write headers
        buffer.extend_from_slice(&headers_bytes);

        // Write value
        buffer.extend_from_slice(&self.to_bytes()?);

        Ok(buffer.freeze())
    }

    /// Parses a complete RUES message into headers and value.
    ///
    /// # Arguments
    /// * `bytes` - Complete RUES message bytes
    ///
    /// # Returns
    /// * `Ok((RuesHeaders, RuesValue))` - Parsed headers and value
    /// * `Err(DomainError)` - If message is invalid or incomplete
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::types::value::RuesValue;
    ///
    /// // Assume we have a valid RUES message
    /// # let message_bytes = vec![/* ... */];
    /// if let Ok((headers, value)) = RuesValue::from_message_bytes(&message_bytes) {
    ///     println!("Content type: {}", headers.content_type());
    /// }
    /// ```
    pub fn from_message_bytes(
        bytes: &[u8],
    ) -> Result<(RuesHeaders, Self), DomainError> {
        if bytes.len() < 4 {
            return Err(SerDeError::MissingField(
                "Message too short for header length".to_string(),
            )
            .into());
        }

        // Read headers length
        let headers_len =
            u32::from_le_bytes(bytes[0..4].try_into().map_err(|_| {
                SerDeError::MissingField(
                    "Invalid header length bytes".to_string(),
                )
            })?) as usize;

        if bytes.len() < 4 + headers_len {
            return Err(SerDeError::MissingField(
                "Message too short for headers".to_string(),
            )
            .into());
        }

        // Parse headers
        let headers = RuesHeaders::from_bytes(&bytes[4..4 + headers_len])?;

        // Parse value
        let value = Self::from_bytes(&bytes[4 + headers_len..])?;

        Ok((headers, value))
    }

    /// Returns the MIME content type for this value.
    ///
    /// Content types:
    /// - Binary: "application/octet-stream"
    /// - JSON: "application/json"
    /// - Text: "text/plain"
    /// - GraphQL: "application/graphql"
    /// - Proof: "application/octet-stream"
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::types::value::RuesValue;
    /// use serde_json::json;
    ///
    /// let json = RuesValue::Json(json!({"test": true}));
    /// assert_eq!(json.content_type(), "application/json");
    /// ```
    pub fn content_type(&self) -> &'static str {
        match self {
            Self::Binary(_) => "application/octet-stream",
            Self::Json(_) => "application/json",
            Self::Text(_) => "text/plain",
            Self::GraphQL(_) => "application/graphql",
            Self::Proof(_) => "application/octet-stream",
        }
    }

    /// Returns the size of the payload in bytes when transmitted over the
    /// network. This matches the Content-Length header value according to
    /// RUES specification.
    pub fn byte_len(&self) -> Result<usize, DomainError> {
        match self {
            Self::Binary(bytes) | Self::Proof(bytes) => Ok(bytes.len()),
            Self::Json(json) => serde_json::to_vec(json)
                .map(|bytes| bytes.len())
                .map_err(|e| SerDeError::Json(e).into()),
            Self::Text(text) | Self::GraphQL(text) => Ok(text.as_bytes().len()),
        }
    }
}

impl Serialize for RuesValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("RuesValue", 2)?;

        match self {
            Self::Binary(bytes) => {
                state.serialize_field("type", "binary")?;
                state.serialize_field("data", &bytes.to_vec())?;
            }
            Self::Json(json) => {
                state.serialize_field("type", "json")?;
                state.serialize_field("data", json)?;
            }
            Self::Text(text) => {
                state.serialize_field("type", "text")?;
                state.serialize_field("data", text)?;
            }
            Self::GraphQL(query) => {
                state.serialize_field("type", "graphql")?;
                state.serialize_field("data", query)?;
            }
            Self::Proof(bytes) => {
                state.serialize_field("type", "proof")?;
                state.serialize_field("data", &bytes.to_vec())?;
            }
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for RuesValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Type,
            Data,
        }

        struct RuesValueVisitor;

        impl<'de> serde::de::Visitor<'de> for RuesValueVisitor {
            type Value = RuesValue;

            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                formatter.write_str("struct RuesValue")
            }

            fn visit_map<V>(self, mut map: V) -> Result<RuesValue, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut type_str: Option<String> = None;
                let mut data_value: Option<serde_json::Value> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Type => {
                            type_str = Some(map.next_value()?);
                        }
                        Field::Data => {
                            data_value = Some(map.next_value()?);
                        }
                    }
                }

                let type_str = type_str
                    .ok_or_else(|| serde::de::Error::missing_field("type"))?;
                let data = data_value
                    .ok_or_else(|| serde::de::Error::missing_field("data"))?;

                match type_str.as_str() {
                    "binary" => {
                        let vec = data
                            .as_array()
                            .ok_or_else(|| {
                                serde::de::Error::custom(
                                    "expected array for binary data",
                                )
                            })?
                            .iter()
                            .map(|v| {
                                v.as_u64()
                                    .ok_or_else(|| {
                                        serde::de::Error::custom(
                                            "expected number",
                                        )
                                    })
                                    .map(|n| n as u8)
                            })
                            .collect::<Result<Vec<u8>, _>>()?;
                        Ok(RuesValue::Binary(Bytes::from(vec)))
                    }
                    "json" => Ok(RuesValue::Json(data)),
                    "text" => {
                        let text = data
                            .as_str()
                            .ok_or_else(|| {
                                serde::de::Error::custom("expected string")
                            })?
                            .to_string();
                        Ok(RuesValue::Text(text))
                    }
                    "graphql" => {
                        let query = data
                            .as_str()
                            .ok_or_else(|| {
                                serde::de::Error::custom("expected string")
                            })?
                            .to_string();
                        Ok(RuesValue::GraphQL(query))
                    }
                    "proof" => {
                        let vec = data
                            .as_array()
                            .ok_or_else(|| {
                                serde::de::Error::custom(
                                    "expected array for proof data",
                                )
                            })?
                            .iter()
                            .map(|v| {
                                v.as_u64()
                                    .ok_or_else(|| {
                                        serde::de::Error::custom(
                                            "expected number",
                                        )
                                    })
                                    .map(|n| n as u8)
                            })
                            .collect::<Result<Vec<u8>, _>>()?;
                        Ok(RuesValue::Proof(Bytes::from(vec)))
                    }
                    _ => Err(serde::de::Error::custom("invalid type")),
                }
            }
        }

        const FIELDS: &[&str] = &["type", "data"];
        deserializer.deserialize_struct("RuesValue", FIELDS, RuesValueVisitor)
    }
}

impl PartialOrd for RuesValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RuesValue {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            // Compare same variants
            (Self::Binary(a), Self::Binary(b)) => a.cmp(b),
            (Self::Json(a), Self::Json(b)) => json_value_compare(a, b),
            (Self::Text(a), Self::Text(b)) => a.cmp(b),
            (Self::GraphQL(a), Self::GraphQL(b)) => a.cmp(b),
            (Self::Proof(a), Self::Proof(b)) => a.cmp(b),
            // Different variants - define consistent ordering between variants
            _ => {
                let variant_order = |v: &RuesValue| match v {
                    Self::Binary(_) => 0,
                    Self::Json(_) => 1,
                    Self::Text(_) => 2,
                    Self::GraphQL(_) => 3,
                    Self::Proof(_) => 4,
                };
                variant_order(self).cmp(&variant_order(other))
            }
        }
    }
}

impl fmt::Display for RuesValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Binary(bytes) => write!(f, "binary({})", hex::encode(bytes)),
            Self::Json(value) => write!(f, "{}", value),
            Self::Text(text) => write!(f, "{}", text),
            Self::GraphQL(query) => write!(f, "{}", query),
            Self::Proof(bytes) => write!(f, "proof({})", hex::encode(bytes)),
        }
    }
}

fn json_value_compare(a: &JsonValue, b: &JsonValue) -> Ordering {
    // First, compare by type
    match (a, b) {
        // Same type - compare values
        (JsonValue::Null, JsonValue::Null) => Ordering::Equal,
        (JsonValue::Bool(a), JsonValue::Bool(b)) => a.cmp(b),
        (JsonValue::Number(a), JsonValue::Number(b)) => {
            // Handle number comparison considering floats
            if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
                a.partial_cmp(&b).unwrap_or(Ordering::Equal)
            } else {
                // Fall back to string comparison if numbers can't be compared
                // directly
                a.to_string().cmp(&b.to_string())
            }
        }
        (JsonValue::String(a), JsonValue::String(b)) => a.cmp(b),
        (JsonValue::Array(a), JsonValue::Array(b)) => {
            // Compare arrays element by element
            for (a, b) in a.iter().zip(b.iter()) {
                match json_value_compare(a, b) {
                    Ordering::Equal => continue,
                    other => return other,
                }
            }
            // If all elements match, compare lengths
            a.len().cmp(&b.len())
        }
        (JsonValue::Object(a), JsonValue::Object(b)) => {
            // Compare objects by sorted keys
            let mut a_keys: Vec<_> = a.keys().collect();
            let mut b_keys: Vec<_> = b.keys().collect();
            a_keys.sort();
            b_keys.sort();

            // First compare keys
            match a_keys.cmp(&b_keys) {
                Ordering::Equal => {
                    // If keys match, compare values in key order
                    for key in a_keys {
                        match json_value_compare(&a[key], &b[key]) {
                            Ordering::Equal => continue,
                            other => return other,
                        }
                    }
                    Ordering::Equal
                }
                other => other,
            }
        }
        // Different types - compare by type order
        _ => {
            let type_order = |v: &JsonValue| match v {
                JsonValue::Null => 0,
                JsonValue::Bool(_) => 1,
                JsonValue::Number(_) => 2,
                JsonValue::String(_) => 3,
                JsonValue::Array(_) => 4,
                JsonValue::Object(_) => 5,
            };
            type_order(a).cmp(&type_order(b))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use serde_json::json;

    #[test]
    fn test_different_value_types() -> Result<(), DomainError> {
        let test_cases = vec![
            (
                RuesValue::Binary(Bytes::from(vec![1, 2, 3])),
                "application/octet-stream",
            ),
            (
                RuesValue::Json(json!({"test": "value"})),
                "application/json",
            ),
            (RuesValue::Text("Hello".into()), "text/plain"),
            (
                RuesValue::GraphQL("query { test }".into()),
                "application/graphql",
            ),
            (
                RuesValue::Proof(Bytes::from(vec![4, 5, 6])),
                "application/octet-stream",
            ),
        ];

        for (value, expected_type) in test_cases {
            assert_eq!(value.content_type(), expected_type);

            let bytes = value.to_bytes()?;
            assert!(!bytes.is_empty());

            // Test roundtrip
            let decoded = RuesValue::from_bytes(&bytes)?;
            assert_eq!(decoded, value);
        }
        Ok(())
    }

    #[test]
    fn test_serde_roundtrip() -> Result<(), DomainError> {
        let test_cases = vec![
            RuesValue::Binary(Bytes::from(vec![1, 2, 3])),
            RuesValue::Json(json!({ "x": 1 })),
            RuesValue::Text("Hi".into()),
            RuesValue::GraphQL("{}".into()),
            RuesValue::Proof(Bytes::from(vec![1, 2])),
        ];

        for original in test_cases {
            let serialized = serde_json::to_string(&original)?;
            let deserialized: RuesValue = serde_json::from_str(&serialized)?;
            assert_eq!(original, deserialized);
        }
        Ok(())
    }

    #[test]
    fn test_bad_length_handling() {
        // Test slice too short
        let short_slice = vec![0u8; 4]; // Too short for header + length
        let result = RuesValue::from_bytes(&short_slice);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(DomainError::SerDe(SerDeError::MissingField(_)))
        ));

        // Test invalid length field
        let mut bad_length = vec![0u8; 8]; // tag + length + empty payload
        bad_length[0] = 0; // Binary tag
        bad_length[1..5].copy_from_slice(&(100u32).to_le_bytes()); // Length larger than available
        let result = RuesValue::from_bytes(&bad_length);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(DomainError::SerDe(SerDeError::MissingField(_)))
        ));
    }

    #[test]
    fn test_large_data_handling() -> Result<(), DomainError> {
        let large_data = Bytes::from(vec![1u8; 100]);
        let value = RuesValue::Binary(large_data);

        let bytes = value.to_bytes()?;
        let decoded = RuesValue::from_bytes(&bytes)?;

        if let RuesValue::Binary(decoded_data) = decoded {
            assert!(!decoded_data.is_empty());
            assert_eq!(decoded_data[0], 1);
        } else {
            return Err(SerDeError::MissingField(
                "Expected Binary variant".into(),
            )
            .into());
        }
        Ok(())
    }

    #[test]
    fn test_invalid_messages() {
        // Too short
        assert!(RuesValue::from_bytes(&[0, 1, 2]).is_err());

        // Invalid headers length
        let mut invalid = BytesMut::new();
        invalid.put_u32_le(1000);
        invalid.extend_from_slice(&[0; 10]);
        assert!(RuesValue::from_message_bytes(&invalid.freeze()).is_err());

        // Invalid headers JSON
        let mut invalid = BytesMut::new();
        invalid.put_u32_le(5);
        invalid.extend_from_slice(b"invalid");
        assert!(RuesValue::from_message_bytes(&invalid.freeze()).is_err());
    }

    #[test]
    fn test_rues_value_ordering() {
        // Test ordering within same variant
        let binary1 = RuesValue::Binary(Bytes::from(vec![1, 2]));
        let binary2 = RuesValue::Binary(Bytes::from(vec![1, 3]));
        assert!(binary1 < binary2);

        let text1 = RuesValue::Text("abc".into());
        let text2 = RuesValue::Text("def".into());
        assert!(text1 < text2);

        let graphql1 = RuesValue::GraphQL("query1".into());
        let graphql2 = RuesValue::GraphQL("query2".into());
        assert!(graphql1 < graphql2);

        let proof1 = RuesValue::Proof(Bytes::from(vec![1]));
        let proof2 = RuesValue::Proof(Bytes::from(vec![2]));
        assert!(proof1 < proof2);

        // Test JSON ordering
        let json1 = RuesValue::Json(json!(null));
        let json2 = RuesValue::Json(json!(false));
        let json3 = RuesValue::Json(json!(true));
        let json4 = RuesValue::Json(json!(42));
        let json5 = RuesValue::Json(json!("text"));
        let json6 = RuesValue::Json(json!([1, 2]));
        let json7 = RuesValue::Json(json!({"key": "value"}));

        assert!(json1 < json2); // null < bool
        assert!(json2 < json3); // false < true
        assert!(json3 < json4); // bool < number
        assert!(json4 < json5); // number < string
        assert!(json5 < json6); // string < array
        assert!(json6 < json7); // array < object

        // Test JSON array ordering
        let arr1 = RuesValue::Json(json!([1, 2]));
        let arr2 = RuesValue::Json(json!([1, 3]));
        let arr3 = RuesValue::Json(json!([1, 2, 3]));
        assert!(arr1 < arr2); // element comparison
        assert!(arr1 < arr3); // length comparison

        // Test JSON object ordering
        let obj1 = RuesValue::Json(json!({"a": 1}));
        let obj2 = RuesValue::Json(json!({"a": 2}));
        let obj3 = RuesValue::Json(json!({"b": 1}));
        assert!(obj1 < obj2); // value comparison
        assert!(obj1 < obj3); // key comparison

        // Test variant ordering
        let binary = RuesValue::Binary(Bytes::from(vec![1]));
        let json = RuesValue::Json(json!(42));
        let text = RuesValue::Text("text".into());
        let graphql = RuesValue::GraphQL("query".into());
        let proof = RuesValue::Proof(Bytes::from(vec![1]));

        // Verify ordering between different variants
        assert!(binary < json);
        assert!(json < text);
        assert!(text < graphql);
        assert!(graphql < proof);

        // Test transitivity
        assert!(binary < text);
        assert!(json < graphql);
        assert!(text < proof);
    }

    #[test]
    fn test_rues_value_ordering_edge_cases() {
        // Test empty vs non-empty
        let empty_arr = RuesValue::Json(json!([]));
        let nonempty_arr = RuesValue::Json(json!([1]));
        assert!(empty_arr < nonempty_arr);

        let empty_obj = RuesValue::Json(json!({}));
        let nonempty_obj = RuesValue::Json(json!({"key": "value"}));
        assert!(empty_obj < nonempty_obj);

        // Test number ordering
        let int = RuesValue::Json(json!(42));
        let float = RuesValue::Json(json!(42.0));
        let negative = RuesValue::Json(json!(-1));
        assert_eq!(int.cmp(&float), Ordering::Equal);
        assert!(negative < int);

        // Test nested structures
        let nested1 = RuesValue::Json(json!({
            "arr": [1, 2],
            "obj": {"a": 1}
        }));
        let nested2 = RuesValue::Json(json!({
            "arr": [1, 2],
            "obj": {"a": 2}
        }));
        assert!(nested1 < nested2);
    }

    #[test]
    fn test_rues_value_ordering_collections() {
        use std::collections::{BTreeMap, BTreeSet};

        // Test ordering in BTreeSet
        let mut set = BTreeSet::new();
        set.insert(RuesValue::Text("b".into()));
        set.insert(RuesValue::Text("a".into()));
        set.insert(RuesValue::Text("c".into()));

        let mut iter = set.iter();
        assert_eq!(iter.next().unwrap(), &RuesValue::Text("a".into()));
        assert_eq!(iter.next().unwrap(), &RuesValue::Text("b".into()));
        assert_eq!(iter.next().unwrap(), &RuesValue::Text("c".into()));

        // Test ordering as BTreeMap keys
        let mut map = BTreeMap::new();
        map.insert(RuesValue::Json(json!(3)), "third");
        map.insert(RuesValue::Json(json!(1)), "first");
        map.insert(RuesValue::Json(json!(2)), "second");

        let values: Vec<_> = map.values().copied().collect();
        assert_eq!(values, vec!["first", "second", "third"]);
    }

    #[test]
    fn test_rues_value_display() {
        // Binary value
        let binary = RuesValue::Binary(Bytes::from(vec![1, 2, 3]));
        assert_eq!(binary.to_string(), "binary(010203)");

        // JSON value
        let json = RuesValue::Json(json!({"test": true}));
        assert_eq!(json.to_string(), r#"{"test":true}"#);

        // Text value
        let text = RuesValue::Text("Hello".into());
        assert_eq!(text.to_string(), "Hello");

        // GraphQL value
        let graphql = RuesValue::GraphQL("query { test }".into());
        assert_eq!(graphql.to_string(), "query { test }");

        // Proof value
        let proof = RuesValue::Proof(Bytes::from(vec![4, 5, 6]));
        assert_eq!(proof.to_string(), "proof(040506)");
    }

    #[test]
    fn test_byte_len_calculation() {
        // Binary
        let bytes = vec![1, 2, 3];
        let value = RuesValue::Binary(Bytes::from(bytes.clone()));
        assert_eq!(value.byte_len().unwrap(), 3);

        // JSON with unicode
        let json = json!({"test": "Hello 🦀"});
        let value = RuesValue::Json(json.clone());
        let expected_len = serde_json::to_vec(&json).unwrap().len();
        assert_eq!(value.byte_len().unwrap(), expected_len);

        // Text with unicode
        let text = "Hello 🦀";
        let value = RuesValue::Text(text.to_string());
        assert_eq!(value.byte_len().unwrap(), text.as_bytes().len());

        // GraphQL
        let query = "query { field(arg: \"Hello 🦀\") }";
        let value = RuesValue::GraphQL(query.to_string());
        assert_eq!(value.byte_len().unwrap(), query.as_bytes().len());

        // Proof
        let bytes = vec![4, 5, 6];
        let value = RuesValue::Proof(Bytes::from(bytes.clone()));
        assert_eq!(value.byte_len().unwrap(), 3);
    }

    #[test]
    fn test_byte_len_matches_binary_format() {
        let values = vec![
            RuesValue::Binary(Bytes::from(vec![1, 2, 3])),
            RuesValue::Json(json!({"test": "Hello 🦀"})),
            RuesValue::Text("Hello 🦀".into()),
            RuesValue::GraphQL("query { test }".into()),
            RuesValue::Proof(Bytes::from(vec![4, 5, 6])),
        ];

        for value in values {
            let bytes = value.to_bytes().unwrap();
            let payload_len = value.byte_len().unwrap();

            // Skip tag byte and length bytes
            let actual_payload = &bytes[5..];
            assert_eq!(actual_payload.len(), payload_len);
        }
    }
}
