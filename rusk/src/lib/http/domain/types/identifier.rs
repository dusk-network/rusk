//! RUES identifier types and representations.
//!
//! This module provides strongly-typed identifiers used in the RUES protocol.
//! All identifiers are created through the processor pipeline to ensure proper
//! validation and format requirements.
//!
//! # Identifier Types
//!
//! - Session ID: 16 bytes random nonce
//! - Block hash: 32 bytes (256-bit)
//! - Transaction hash: 32 bytes (256-bit)
//! - Contract ID: Variable length
//!
//! # Binary Format
//!
//! All identifiers are stored as binary data and displayed as hexadecimal
//! strings:
//! - Internal storage: RuesValue::Binary
//! - Display format: Base16-encoded string
//! - Length validation: Enforced by processor pipeline
//!
//! # Visibility Design
//!
//! - Public types for API usage
//! - `pub(crate)` fields to enforce processor pipeline usage
//! - Public read-only access methods
//! - Public display formatting
//!
//! # Thread Safety and Performance
//!
//! All types are:
//! - `Send + Sync` for thread safety
//! - `Clone` for ownership transfer
//! - `Hash + Eq + Ord` for collection usage
//! - Zero-copy where possible
//! - Immutable after construction

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::RuesValue;
use std::fmt;

/// Base type for all RUES identifiers.
///
/// # Binary Format
/// - Storage: RuesValue::Binary
/// - Display: Base16-encoded string
///
/// # Thread Safety
/// - Implements Send + Sync
/// - Immutable after construction
/// - Safe for concurrent access
///
/// # Performance
/// - Zero-copy binary storage
/// - Efficient hash and comparison
/// - Minimal allocations
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IdentifierBytes(pub(crate) RuesValue);

impl IdentifierBytes {
    /// Returns a reference to the underlying RUES value.
    ///
    /// This method provides read-only access to the identifier's binary value.
    /// The value will always be `RuesValue::Binary` for valid identifiers.
    ///
    /// Note: identifiers should be created through the RUES processor pipeline
    /// to ensure proper validation and format requirements.
    pub fn value(&self) -> &RuesValue {
        &self.0
    }
}

impl fmt::Display for IdentifierBytes {
    /// Formats the identifier as a hexadecimal string.
    ///
    /// For valid identifiers (containing `RuesValue::Binary`), outputs the
    /// hexadecimal representation of the bytes. For invalid values, outputs
    /// "\<invalid\>".
    ///
    /// # Implementation Note
    ///
    /// Uses the `hex` crate for efficient and secure hexadecimal encoding.
    /// Invalid values should never occur in practice as construction is
    /// controlled through the processor pipeline.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let RuesValue::Binary(bytes) = &self.value() {
            write!(f, "{}", hex::encode(bytes))
        } else {
            write!(f, "<invalid>")
        }
    }
}

/// Session identifier (16 bytes).
///
/// # Binary Format
/// - Length: Exactly 16 bytes
/// - Storage: RuesValue::Binary
/// - Display: 32-character hexadecimal string
///
/// # Validation
/// - Length check: Exactly 16 bytes
/// - Format: Valid binary data
/// - Creation: Through processor pipeline only
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionId(pub(crate) IdentifierBytes);

impl SessionId {
    /// Returns a reference to the underlying RUES value.
    ///
    /// # Usage
    ///
    /// This method provides read-only access to the session identifier's binary
    /// value. Session IDs are created through the RUES processor pipeline,
    /// which ensures:
    /// - Exactly 16 bytes length
    /// - Proper binary format
    /// - Valid hexadecimal encoding
    ///
    /// # Binary Format
    ///
    /// - Length: 16 bytes (128 bits)
    /// - Encoding: Binary value stored as RuesValue::Binary
    /// - Display: 32-character hexadecimal string
    pub fn value(&self) -> &RuesValue {
        self.0.value()
    }
}

impl fmt::Display for SessionId {
    /// Formats the session ID as a 32-character hexadecimal string.
    ///
    /// # Implementation Note
    ///
    /// The output is always 32 characters long for valid session IDs,
    /// as they contain exactly 16 bytes. The display implementation
    /// delegates to the underlying `IdentifierBytes`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for SessionId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let RuesValue::Binary(bytes) = self.0.value() {
            serializer.serialize_str(&hex::encode(bytes))
        } else {
            Err(serde::ser::Error::custom("Invalid session ID format"))
        }
    }
}

impl<'de> Deserialize<'de> for SessionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hex_str = String::deserialize(deserializer)?;
        let bytes = hex::decode(hex_str).map_err(|e| {
            serde::de::Error::custom(format!("Invalid hex string: {}", e))
        })?;

        if bytes.len() != 16 {
            return Err(serde::de::Error::custom(format!(
                "Invalid session ID length: expected 16 bytes, got {}",
                bytes.len()
            )));
        }

        Ok(SessionId(IdentifierBytes(RuesValue::Binary(bytes.into()))))
    }
}

/// Block hash (32 bytes).
///
/// # Binary Format
/// - Length: Exactly 32 bytes (256-bit)
/// - Storage: RuesValue::Binary
/// - Display: 64-character hexadecimal string
///
/// # Validation
/// - Length check: Exactly 32 bytes
/// - Format: Valid binary data
/// - Creation: Through processor pipeline only
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockHash(pub(crate) IdentifierBytes);

impl BlockHash {
    /// Returns a reference to the underlying RUES value.
    ///
    /// # Usage
    ///
    /// This method provides read-only access to the block hash binary
    /// value. Block hashes are created through the RUES processor pipeline
    /// only, which ensures:
    /// - Exactly 32 bytes length
    /// - Proper binary format
    /// - Valid hexadecimal encoding
    ///
    /// # Binary Format
    ///
    /// - Length: 32 bytes (256 bits)
    /// - Encoding: Binary value stored as RuesValue::Binary
    /// - Display: 64-character hexadecimal string
    pub fn value(&self) -> &RuesValue {
        self.0.value()
    }
}

impl fmt::Display for BlockHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Transaction hash (32 bytes).
///
/// # Binary Format
/// - Length: Exactly 32 bytes (256-bit)
/// - Storage: RuesValue::Binary
/// - Display: 64-character hexadecimal string
///
/// # Validation
/// - Length check: Exactly 32 bytes
/// - Format: Valid binary data
/// - Creation: Through processor pipeline only
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TransactionHash(pub(crate) IdentifierBytes);

impl TransactionHash {
    /// Returns a reference to the underlying RUES value.
    ///
    /// # Usage
    ///
    /// This method provides read-only access to the transaction hash binary
    /// value. Transaction hashes are created through the RUES processor
    /// pipeline only, which ensures:
    /// - Exactly 32 bytes length
    /// - Proper binary format
    /// - Valid hexadecimal encoding
    ///
    /// # Binary Format
    ///
    /// - Length: 32 bytes (256 bits)
    /// - Encoding: Binary value stored as RuesValue::Binary
    /// - Display: 64-character hexadecimal string
    pub fn value(&self) -> &RuesValue {
        self.0.value()
    }
}

impl fmt::Display for TransactionHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Contract identifier (variable length).
///
/// # Binary Format
/// - Length: Variable
/// - Storage: RuesValue::Binary
/// - Display: Base16-encoded string
///
/// # Validation
/// - Format: Valid binary data
/// - Creation: Through processor pipeline only
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContractId(pub(crate) IdentifierBytes);

impl ContractId {
    /// Returns a reference to the underlying RUES value.
    ///
    /// Provides read-only access to the contract identifier. Unlike other
    /// identifiers, contract IDs can have variable length.
    ///
    /// # Binary Format
    /// - Length: Variable
    /// - Storage: RuesValue::Binary
    /// - Display: Base16-encoded string
    ///
    /// # Validation
    /// - Format: Valid binary data
    /// - Creation: Through processor pipeline only
    pub fn value(&self) -> &RuesValue {
        self.0.value()
    }
}

impl fmt::Display for ContractId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Target-specific identifier types.
///
/// # Variants
/// - Block: 32-byte block hash
/// - Transaction: 32-byte transaction hash
/// - Contract: Variable length contract ID
///
/// # Type Safety
/// - Enforces correct identifier type per target
/// - Prevents mixing of different identifier types
/// - Maintains length and format requirements
///
/// # Thread Safety
/// - Safe for concurrent access
/// - Immutable after construction
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TargetIdentifier {
    /// Block identifier (32-byte hash)
    Block(BlockHash),
    /// Transaction identifier (32-byte hash)
    Transaction(TransactionHash),
    /// Contract identifier (variable length)
    Contract(ContractId),
}

impl fmt::Display for TargetIdentifier {
    /// Formats the target-specific identifier as a hexadecimal string.
    ///
    /// The format varies by variant:
    /// - Block: 64-character hex string (32 bytes)
    /// - Transaction: 64-character hex string (32 bytes)
    /// - Contract: Variable length hex string
    ///
    /// # Implementation Note
    ///
    /// Delegates to the specific identifier type's Display implementation,
    /// maintaining consistent formatting across all identifier types.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Block(hash) => write!(f, "{}", hash),
            Self::Transaction(hash) => write!(f, "{}", hash),
            Self::Contract(id) => write!(f, "{}", id),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::http::domain::testing;

    use super::*;
    use bytes::Bytes;
    use serde_json::{from_value, json, to_value};

    fn create_binary_value(hex: &str) -> RuesValue {
        RuesValue::Binary(Bytes::copy_from_slice(&hex::decode(hex).unwrap()))
    }

    #[test]
    fn test_identifier_bytes_display() {
        let id = IdentifierBytes(create_binary_value("01020304"));
        assert_eq!(id.to_string(), "01020304");
    }

    #[test]
    fn test_session_id() {
        // 16 bytes session ID
        let session = SessionId(IdentifierBytes(create_binary_value(
            "0102030405060708090a0b0c0d0e0f10",
        )));

        assert_eq!(session.to_string(), "0102030405060708090a0b0c0d0e0f10");

        if let RuesValue::Binary(bytes) = session.value() {
            assert_eq!(bytes.len(), 16);
        } else {
            panic!("Expected binary value");
        }
    }

    #[test]
    fn test_session_id_serialization() {
        let session_id = testing::create_test_session_id();
        let value = to_value(&session_id).unwrap();
        assert!(value.is_string());

        let deserialized: SessionId = from_value(value).unwrap();
        assert_eq!(session_id, deserialized);
    }

    #[test]
    fn test_invalid_session_id() {
        // Invalid hex string
        let result = serde_json::from_str::<SessionId>("\"not hex\"");
        assert!(result.is_err());

        // Wrong length
        let result = serde_json::from_str::<SessionId>("\"1234\"");
        assert!(result.is_err());
    }

    #[test]
    fn test_block_hash() {
        // 32 bytes block hash
        let hash = BlockHash(IdentifierBytes(create_binary_value(
            "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20",
        )));

        assert_eq!(
            hash.to_string(),
            "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"
        );

        if let RuesValue::Binary(bytes) = hash.value() {
            assert_eq!(bytes.len(), 32);
        } else {
            panic!("Expected binary value");
        }
    }

    #[test]
    fn test_transaction_hash() {
        // 32 bytes transaction hash
        let hash = TransactionHash(IdentifierBytes(create_binary_value(
            "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20",
        )));

        assert_eq!(
            hash.to_string(),
            "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"
        );

        if let RuesValue::Binary(bytes) = hash.value() {
            assert_eq!(bytes.len(), 32);
        } else {
            panic!("Expected binary value");
        }
    }

    #[test]
    fn test_contract_id() {
        let id = ContractId(IdentifierBytes(create_binary_value(
            "0102030405060708090a0b0c",
        )));

        assert_eq!(id.to_string(), "0102030405060708090a0b0c");

        if let RuesValue::Binary(bytes) = id.value() {
            assert_eq!(bytes.len(), 12); // Example length
        } else {
            panic!("Expected binary value");
        }
    }

    #[test]
    fn test_target_identifier_display() {
        let block_id = TargetIdentifier::Block(BlockHash(IdentifierBytes(
            create_binary_value("0102"),
        )));
        assert_eq!(block_id.to_string(), "0102");

        let tx_id = TargetIdentifier::Transaction(TransactionHash(
            IdentifierBytes(create_binary_value("0304")),
        ));
        assert_eq!(tx_id.to_string(), "0304");

        let contract_id = TargetIdentifier::Contract(ContractId(
            IdentifierBytes(create_binary_value("0506")),
        ));
        assert_eq!(contract_id.to_string(), "0506");
    }

    #[test]
    fn test_identifier_ordering() {
        let id1 = IdentifierBytes(create_binary_value("0102"));
        let id2 = IdentifierBytes(create_binary_value("0103"));
        assert!(id1 < id2);
    }

    #[test]
    fn test_identifier_hashing() {
        use std::collections::HashSet;

        let mut set = HashSet::new();

        let id1 = BlockHash(IdentifierBytes(create_binary_value("0102")));
        let id2 = BlockHash(IdentifierBytes(create_binary_value("0103")));

        set.insert(id1.clone());
        set.insert(id2.clone());
        set.insert(id1.clone()); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&id1));
        assert!(set.contains(&id2));
    }

    #[test]
    fn test_target_identifier_value_access() {
        let target_id = TargetIdentifier::Block(BlockHash(IdentifierBytes(
            create_binary_value("0102"),
        )));

        match target_id {
            TargetIdentifier::Block(hash) => {
                if let RuesValue::Binary(bytes) = hash.value() {
                    // Convert Bytes to &[u8] using as_ref()
                    assert_eq!(bytes.as_ref(), &[1, 2]);
                } else {
                    panic!("Expected binary value");
                }
            }
            _ => panic!("Expected block hash"),
        }
    }

    #[test]
    fn test_invalid_rues_value() {
        // Test displaying an identifier with invalid RuesValue type
        let invalid_id = IdentifierBytes(RuesValue::Text("not binary".into()));
        assert_eq!(invalid_id.to_string(), "<invalid>");
    }

    #[test]
    fn test_rues_value_equality() {
        // Test Binary equality
        assert_eq!(
            RuesValue::Binary(Bytes::from(vec![1, 2, 3])),
            RuesValue::Binary(Bytes::from(vec![1, 2, 3]))
        );
        assert_ne!(
            RuesValue::Binary(Bytes::from(vec![1, 2, 3])),
            RuesValue::Binary(Bytes::from(vec![1, 2, 4]))
        );

        // Test Json equality
        assert_eq!(
            RuesValue::Json(json!({"a": 1, "b": 2})),
            RuesValue::Json(json!({"a": 1, "b": 2}))
        );
        assert_ne!(
            RuesValue::Json(json!({"a": 1})),
            RuesValue::Json(json!({"a": 2}))
        );

        // Test Text equality
        assert_eq!(
            RuesValue::Text("hello".into()),
            RuesValue::Text("hello".into())
        );
        assert_ne!(
            RuesValue::Text("hello".into()),
            RuesValue::Text("world".into())
        );

        // Test GraphQL equality
        assert_eq!(
            RuesValue::GraphQL("query { test }".into()),
            RuesValue::GraphQL("query { test }".into())
        );
        assert_ne!(
            RuesValue::GraphQL("query1".into()),
            RuesValue::GraphQL("query2".into())
        );

        // Test Proof equality
        assert_eq!(
            RuesValue::Proof(Bytes::from(vec![1, 2, 3])),
            RuesValue::Proof(Bytes::from(vec![1, 2, 3]))
        );
        assert_ne!(
            RuesValue::Proof(Bytes::from(vec![1, 2, 3])),
            RuesValue::Proof(Bytes::from(vec![1, 2, 4]))
        );

        // Test different variants inequality
        assert_ne!(
            RuesValue::Binary(Bytes::from(vec![1, 2, 3])),
            RuesValue::Proof(Bytes::from(vec![1, 2, 3]))
        );
        assert_ne!(
            RuesValue::Text("test".into()),
            RuesValue::GraphQL("test".into())
        );
    }

    #[test]
    fn test_rues_value_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::collections::HashSet;
        use std::hash::{Hash, Hasher};

        fn calculate_hash<T: Hash>(t: &T) -> u64 {
            let mut hasher = DefaultHasher::new();
            t.hash(&mut hasher);
            hasher.finish()
        }

        // Test hash equality for equal values
        let value1 = RuesValue::Binary(Bytes::from(vec![1, 2, 3]));
        let value2 = RuesValue::Binary(Bytes::from(vec![1, 2, 3]));
        assert_eq!(calculate_hash(&value1), calculate_hash(&value2));

        let json1 = RuesValue::Json(json!({"a": 1, "b": 2}));
        let json2 = RuesValue::Json(json!({"a": 1, "b": 2}));
        assert_eq!(calculate_hash(&json1), calculate_hash(&json2));

        // Test hash inequality for different values
        let text1 = RuesValue::Text("hello".into());
        let text2 = RuesValue::Text("world".into());
        assert_ne!(calculate_hash(&text1), calculate_hash(&text2));

        // Test HashSet behavior
        let mut set = HashSet::new();

        // Add different values
        set.insert(RuesValue::Binary(Bytes::from(vec![1, 2])));
        set.insert(RuesValue::Json(json!(42)));
        set.insert(RuesValue::Text("text".into()));
        set.insert(RuesValue::GraphQL("query".into()));
        set.insert(RuesValue::Proof(Bytes::from(vec![3, 4])));

        // Try to insert duplicates
        assert!(!set.insert(RuesValue::Binary(Bytes::from(vec![1, 2]))));
        assert!(!set.insert(RuesValue::Json(json!(42))));

        // Verify set size
        assert_eq!(set.len(), 5);

        // Test complex JSON values in HashSet
        let mut json_set = HashSet::new();
        json_set.insert(RuesValue::Json(json!({
            "array": [1, 2, 3],
            "nested": {
                "value": true,
                "data": null
            }
        })));

        // Insert same JSON structure
        assert!(!json_set.insert(RuesValue::Json(json!({
            "array": [1, 2, 3],
            "nested": {
                "value": true,
                "data": null
            }
        }))));

        assert_eq!(json_set.len(), 1);
    }

    #[test]
    fn test_rues_value_complex_equality() {
        // Test nested JSON structures
        assert_eq!(
            RuesValue::Json(json!({
                "array": [1, 2, {"nested": true}],
                "object": {"a": null, "b": [1, 2]}
            })),
            RuesValue::Json(json!({
                "array": [1, 2, {"nested": true}],
                "object": {"a": null, "b": [1, 2]}
            }))
        );

        // Test JSON array ordering doesn't affect equality
        assert_eq!(
            RuesValue::Json(json!({"arr": [1, 2, 3]})),
            RuesValue::Json(json!({"arr": [1, 2, 3]}))
        );

        // Test JSON object key ordering doesn't affect equality
        assert_eq!(
            RuesValue::Json(json!({"a": 1, "b": 2})),
            RuesValue::Json(json!({"b": 2, "a": 1}))
        );

        // Test empty values equality
        assert_eq!(
            RuesValue::Binary(Bytes::from(vec![])),
            RuesValue::Binary(Bytes::from(vec![]))
        );
        assert_eq!(RuesValue::Json(json!({})), RuesValue::Json(json!({})));
        assert_eq!(RuesValue::Text("".into()), RuesValue::Text("".into()));
    }
}
