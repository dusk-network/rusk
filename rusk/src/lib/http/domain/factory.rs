//! Factory methods for creating domain types.
//! This module has access to private fields of domain types
//! and is used by the processing module.

use crate::http::domain::{
    BlockHash, ContractId, DomainError, IdentifierBytes, RuesValue, Target,
    TargetIdentifier, Topic, TransactionHash, ValidationError, Version,
};

/// Creates domain types from raw values.
/// Only accessible within the crate, primarily used by processors (directly) or
/// in documentation examples and tests (via `domain::testing` module that is
/// hidden in docs by the `#[doc(hidden)]` attribute).
pub(crate) struct DomainTypesFactory;

impl DomainTypesFactory {
    /// Creates a block identifier from a value.
    pub(crate) fn create_block_hash(
        value: RuesValue,
    ) -> Result<BlockHash, DomainError> {
        if let RuesValue::Binary(bytes) = &value {
            if bytes.len() != 32 {
                return Err(ValidationError::InvalidFieldValue {
                    field: "block_hash".into(),
                    reason: format!(
                        "Invalid length: expected 32, got {}",
                        bytes.len()
                    ),
                }
                .into());
            }
            Ok(BlockHash(IdentifierBytes(value)))
        } else {
            Err(ValidationError::InvalidFieldValue {
                field: "block_hash".into(),
                reason: "Expected binary value".into(),
            }
            .into())
        }
    }

    /// Creates a transaction identifier from a value.
    pub(crate) fn create_transaction_hash(
        value: RuesValue,
    ) -> Result<TransactionHash, DomainError> {
        if let RuesValue::Binary(bytes) = &value {
            if bytes.len() != 32 {
                return Err(ValidationError::InvalidFieldValue {
                    field: "transaction_hash".into(),
                    reason: format!(
                        "Invalid length: expected 32, got {}",
                        bytes.len()
                    ),
                }
                .into());
            }
            Ok(TransactionHash(IdentifierBytes(value)))
        } else {
            Err(ValidationError::InvalidFieldValue {
                field: "transaction_hash".into(),
                reason: "Expected binary value".into(),
            }
            .into())
        }
    }

    /// Creates a contract identifier from a value.
    pub(crate) fn create_contract_id(
        value: RuesValue,
    ) -> Result<ContractId, DomainError> {
        if let RuesValue::Binary(_) = &value {
            Ok(ContractId(IdentifierBytes(value)))
        } else {
            Err(ValidationError::InvalidFieldValue {
                field: "contract_id".into(),
                reason: "Expected binary value".into(),
            }
            .into())
        }
    }

    /// Creates a target identifier from a value and target type.
    pub(crate) fn create_target_identifier(
        value: RuesValue,
        target: Target,
    ) -> Result<TargetIdentifier, DomainError> {
        match target {
            Target::Blocks => {
                Ok(TargetIdentifier::Block(Self::create_block_hash(value)?))
            }
            Target::Transactions => Ok(TargetIdentifier::Transaction(
                Self::create_transaction_hash(value)?,
            )),
            Target::Contracts => {
                Ok(TargetIdentifier::Contract(Self::create_contract_id(value)?))
            }
            _ => Err(ValidationError::InvalidFormat(format!(
                "Target {:?} does not support identifiers",
                target
            ))
            .into()),
        }
    }

    /// Creates a version instance.
    pub(crate) fn create_version(
        major: u8,
        minor: u8,
        patch: u8,
        pre_release: Option<u8>,
    ) -> Version {
        Version::new(major, minor, patch, pre_release)
    }

    /// Creates a topic from a string.
    pub(crate) fn create_topic(value: impl Into<String>) -> Topic {
        Topic::new(value.into())
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_block_hash_creation() {
        // Valid block hash (32 bytes)
        let value = RuesValue::Binary(Bytes::copy_from_slice(&[1u8; 32]));
        let block_hash = DomainTypesFactory::create_block_hash(value.clone())
            .expect("Should create valid block hash");
        assert_eq!(block_hash.value(), &value);

        // Wrong value type
        let wrong_value = RuesValue::Text("not binary".into());
        assert!(DomainTypesFactory::create_block_hash(wrong_value).is_err());

        // Wrong length
        let short_value = RuesValue::Binary(Bytes::copy_from_slice(&[1u8; 16]));
        assert!(DomainTypesFactory::create_block_hash(short_value).is_err());
    }

    #[test]
    fn test_transaction_hash_creation() {
        // Valid transaction hash (32 bytes)
        let value = RuesValue::Binary(Bytes::copy_from_slice(&[2u8; 32]));
        let tx_hash =
            DomainTypesFactory::create_transaction_hash(value.clone())
                .expect("Should create valid transaction hash");
        assert_eq!(tx_hash.value(), &value);

        // Wrong value type
        let wrong_value = RuesValue::Text("not binary".into());
        assert!(
            DomainTypesFactory::create_transaction_hash(wrong_value).is_err()
        );

        // Wrong length
        let short_value = RuesValue::Binary(Bytes::copy_from_slice(&[2u8; 16]));
        assert!(
            DomainTypesFactory::create_transaction_hash(short_value).is_err()
        );
    }

    #[test]
    fn test_contract_id_creation() {
        // Valid contract ID (any length)
        let value = RuesValue::Binary(Bytes::copy_from_slice(&[3u8; 12]));
        let contract_id = DomainTypesFactory::create_contract_id(value.clone())
            .expect("Should create valid contract ID");
        assert_eq!(contract_id.value(), &value);

        // Wrong value type
        let wrong_value = RuesValue::Text("not binary".into());
        assert!(DomainTypesFactory::create_contract_id(wrong_value).is_err());

        // Different lengths should work for contract IDs
        let short_value = RuesValue::Binary(Bytes::copy_from_slice(&[3u8; 8]));
        assert!(DomainTypesFactory::create_contract_id(short_value).is_ok());
        let long_value = RuesValue::Binary(Bytes::copy_from_slice(&[3u8; 16]));
        assert!(DomainTypesFactory::create_contract_id(long_value).is_ok());
    }

    #[test]
    fn test_target_identifier_creation() {
        // Valid block hash
        let block_value = RuesValue::Binary(Bytes::copy_from_slice(&[1u8; 32]));
        let block_id = DomainTypesFactory::create_target_identifier(
            block_value,
            Target::Blocks,
        )
        .expect("Should create block identifier");
        assert!(matches!(block_id, TargetIdentifier::Block(_)));

        // Valid transaction hash
        let tx_value = RuesValue::Binary(Bytes::copy_from_slice(&[2u8; 32]));
        let tx_id = DomainTypesFactory::create_target_identifier(
            tx_value,
            Target::Transactions,
        )
        .expect("Should create transaction identifier");
        assert!(matches!(tx_id, TargetIdentifier::Transaction(_)));

        // Valid contract ID
        let contract_value =
            RuesValue::Binary(Bytes::copy_from_slice(&[3u8; 12]));
        let contract_id = DomainTypesFactory::create_target_identifier(
            contract_value,
            Target::Contracts,
        )
        .expect("Should create contract identifier");
        assert!(matches!(contract_id, TargetIdentifier::Contract(_)));

        // Target without ID support
        let value = RuesValue::Binary(Bytes::from(vec![1]));
        let result =
            DomainTypesFactory::create_target_identifier(value, Target::Node);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_value_types() {
        let text_value = RuesValue::Text("invalid".into());
        let json_value = RuesValue::Json(serde_json::json!({"invalid": true}));
        let graphql_value = RuesValue::GraphQL("query".into());

        // Try with different targets
        let targets = [Target::Blocks, Target::Transactions, Target::Contracts];
        for target in targets {
            assert!(DomainTypesFactory::create_target_identifier(
                text_value.clone(),
                target
            )
            .is_err());
            assert!(DomainTypesFactory::create_target_identifier(
                json_value.clone(),
                target
            )
            .is_err());
            assert!(DomainTypesFactory::create_target_identifier(
                graphql_value.clone(),
                target
            )
            .is_err());
        }
    }

    #[test]
    fn test_wrong_binary_lengths() {
        // Wrong length for block hash (not 32 bytes)
        let short_block = RuesValue::Binary(Bytes::copy_from_slice(&[1u8; 16]));
        assert!(DomainTypesFactory::create_target_identifier(
            short_block,
            Target::Blocks
        )
        .is_err());

        // Wrong length for transaction hash (not 32 bytes)
        let short_tx = RuesValue::Binary(Bytes::copy_from_slice(&[2u8; 16]));
        assert!(DomainTypesFactory::create_target_identifier(
            short_tx,
            Target::Transactions
        )
        .is_err());

        // Any length should work for contract IDs
        let contract_values = [8, 12, 16, 32];
        for len in contract_values {
            let value =
                RuesValue::Binary(Bytes::copy_from_slice(&vec![3u8; len]));
            assert!(DomainTypesFactory::create_target_identifier(
                value,
                Target::Contracts
            )
            .is_ok());
        }
    }

    #[test]
    fn test_version_creation() {
        // Test release version
        let version = DomainTypesFactory::create_version(1, 2, 3, None);
        assert_eq!(version.major(), 1);
        assert_eq!(version.minor(), 2);
        assert_eq!(version.patch(), 3);
        assert_eq!(version.pre_release(), None);
        assert!(!version.is_pre_release());

        // Test pre-release version
        let pre_release = DomainTypesFactory::create_version(2, 0, 0, Some(1));
        assert_eq!(pre_release.major(), 2);
        assert_eq!(pre_release.minor(), 0);
        assert_eq!(pre_release.patch(), 0);
        assert_eq!(pre_release.pre_release(), Some(1));
        assert!(pre_release.is_pre_release());
    }

    #[test]
    fn test_version_ordering_comprehensive() {
        // Test releases
        let v1_0_0 = DomainTypesFactory::create_version(1, 0, 0, None);
        let v1_0_1 = DomainTypesFactory::create_version(1, 0, 1, None);
        let v1_1_0 = DomainTypesFactory::create_version(1, 1, 0, None);
        let v2_0_0 = DomainTypesFactory::create_version(2, 0, 0, None);

        // Test pre-releases
        let v1_0_0_pre1 = DomainTypesFactory::create_version(1, 0, 0, Some(1));
        let v1_0_0_pre2 = DomainTypesFactory::create_version(1, 0, 0, Some(2));
        let v1_0_1_pre1 = DomainTypesFactory::create_version(1, 0, 1, Some(1));
        let v2_0_0_pre1 = DomainTypesFactory::create_version(2, 0, 0, Some(1));

        // Release ordering
        assert!(v1_0_0 < v1_0_1, "1.0.0 should be less than 1.0.1");
        assert!(v1_0_1 < v1_1_0, "1.0.1 should be less than 1.1.0");
        assert!(v1_1_0 < v2_0_0, "1.1.0 should be less than 2.0.0");

        // Pre-release vs release (same version numbers)
        assert!(v1_0_0_pre1 < v1_0_0, "1.0.0-1 should be less than 1.0.0");
        assert!(v1_0_0_pre2 < v1_0_0, "1.0.0-2 should be less than 1.0.0");
        assert!(v1_0_1_pre1 < v1_0_1, "1.0.1-1 should be less than 1.0.1");

        // Pre-release ordering (same version numbers)
        assert!(
            v1_0_0_pre1 < v1_0_0_pre2,
            "1.0.0-1 should be less than 1.0.0-2"
        );

        // Different major/minor/patch versions take precedence over pre-release
        // status
        assert!(v1_0_0_pre1 < v1_0_1, "1.0.0-1 should be less than 1.0.1");
        assert!(v1_0_1_pre1 < v1_1_0, "1.0.1-1 should be less than 1.1.0");
        assert!(v1_1_0 < v2_0_0_pre1, "1.1.0 should be less than 2.0.0-1"); // Removed this test
        assert!(v2_0_0_pre1 < v2_0_0, "2.0.0-1 should be less than 2.0.0");

        // Transitivity tests
        assert!(
            v1_0_0_pre1 < v1_0_0_pre2 && v1_0_0_pre2 < v1_0_0,
            "Pre-release ordering should be transitive"
        );
        assert!(
            v1_0_0 < v1_0_1 && v1_0_1 < v1_1_0,
            "Version ordering should be transitive"
        );

        // Equality tests
        let v1_0_0_copy = DomainTypesFactory::create_version(1, 0, 0, None);
        let v1_0_0_pre1_copy =
            DomainTypesFactory::create_version(1, 0, 0, Some(1));

        assert_eq!(v1_0_0, v1_0_0_copy, "Equal release versions");
        assert_eq!(v1_0_0_pre1, v1_0_0_pre1_copy, "Equal pre-release versions");
        assert_ne!(
            v1_0_0, v1_0_0_pre1,
            "Release and pre-release should not be equal"
        );

        // Complex ordering chain
        let versions = vec![
            v1_0_0_pre1,
            v1_0_0_pre2,
            v1_0_0,
            v1_0_1_pre1,
            v1_0_1,
            v1_1_0,
            v2_0_0_pre1,
            v2_0_0,
        ];

        // Check if the vector is properly ordered
        assert!(
            versions.windows(2).all(|w| w[0] < w[1]),
            "Version sequence should be strictly ordered"
        );

        // Backward compatibility tests
        assert!(
            v1_0_1.is_compatible_with(&v1_0_0),
            "Patch versions should be compatible"
        );
        assert!(
            v1_1_0.is_compatible_with(&v1_0_0),
            "Minor versions should be compatible"
        );
        assert!(
            !v2_0_0.is_compatible_with(&v1_0_0),
            "Major versions should not be compatible"
        );
        assert!(
            !v1_0_0_pre1.is_compatible_with(&v1_0_0),
            "Pre-release should not be compatible with release"
        );
    }

    #[test]
    fn test_version_pre_release_comparison() {
        let release = DomainTypesFactory::create_version(1, 0, 0, None);
        let pre_release = DomainTypesFactory::create_version(1, 0, 0, Some(1));

        assert!(pre_release < release, "1.0.0-1 should be less than 1.0.0");
        assert!(
            release > pre_release,
            "1.0.0 should be greater than 1.0.0-1"
        );
        assert!(
            !pre_release.is_compatible_with(&release),
            "Pre-release should not be compatible with release"
        );
    }

    #[test]
    fn test_version_compatibility() {
        let v1 = DomainTypesFactory::create_version(1, 0, 0, None);
        let v1_1 = DomainTypesFactory::create_version(1, 1, 0, None);
        let v2 = DomainTypesFactory::create_version(2, 0, 0, None);
        let v1_pre = DomainTypesFactory::create_version(1, 0, 0, Some(1));
        let v1_pre_same = DomainTypesFactory::create_version(1, 0, 0, Some(1));
        let v1_pre2 = DomainTypesFactory::create_version(1, 0, 0, Some(2));

        // Same major version should be compatible
        assert!(v1.is_compatible_with(&v1_1));
        assert!(v1_1.is_compatible_with(&v1));

        // Different major versions should not be compatible
        assert!(!v1.is_compatible_with(&v2));
        assert!(!v2.is_compatible_with(&v1));

        // Pre-release versions
        assert!(v1_pre.is_compatible_with(&v1_pre_same)); // Same pre-release
        assert!(!v1_pre.is_compatible_with(&v1_pre2)); // Different pre-release
        assert!(!v1_pre.is_compatible_with(&v1)); // Pre-release vs release
        assert!(!v1.is_compatible_with(&v1_pre)); // Release vs pre-release
    }

    #[test]
    fn test_create_topic() {
        // Test with different input types
        let string_topic =
            DomainTypesFactory::create_topic(String::from("test"));
        assert_eq!(string_topic.as_str(), "test");

        let str_topic = DomainTypesFactory::create_topic("static");
        assert_eq!(str_topic.as_str(), "static");

        // Test with empty string
        let empty_topic = DomainTypesFactory::create_topic("");
        assert_eq!(empty_topic.as_str(), "");

        // Test with special characters
        let special_chars =
            DomainTypesFactory::create_topic("test.topic-name_123");
        assert_eq!(special_chars.as_str(), "test.topic-name_123");

        // Test with unicode
        let unicode = DomainTypesFactory::create_topic("🦀");
        assert_eq!(unicode.as_str(), "🦀");

        // Test uniqueness
        let topic1 = DomainTypesFactory::create_topic("same");
        let topic2 = DomainTypesFactory::create_topic("same");
        assert_eq!(topic1, topic2);
        assert_ne!(topic1.as_str(), "different");
    }
}
