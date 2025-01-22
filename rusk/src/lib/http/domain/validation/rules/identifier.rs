// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Validation rules for RUES identifiers used in routing.
//!
//! This module provides validation rules for different identifier types in the
//! RUES protocol:
//! - Session IDs (16 bytes binary)
//! - Block hashes (32 bytes binary)
//! - Transaction hashes (32 bytes binary)
//! - Contract IDs (variable length binary)
//!
//! Note that these validations focus only on format requirements needed for
//! routing, not on the actual validity or existence of the identified
//! resources.
//!
//! # Examples
//!
//! Validating a session ID:
//! ```rust
//! use rusk::http::domain::validation::rules::identifier::SessionIdValidator;
//! use rusk::http::domain::types::identifier::SessionId;
//! use rusk::http::domain::validation::rules::ValidationRule;
//! use rusk::http::domain::validation::context::ValidationContext;
//! # use rusk::http::domain::testing;
//!
//! let validator = SessionIdValidator::new();
//! let mut ctx = ValidationContext::new();
//!
//! // Validate a session ID
//! let session_id = testing::create_test_session_id();
//! assert!(validator.check(&session_id, &mut ctx).is_ok());
//! ```
//!
//! Validating a block hash:
//! ```rust
//! use rusk::http::domain::validation::rules::identifier::BlockHashValidator;
//! use rusk::http::domain::types::identifier::{BlockHash, TargetIdentifier};
//! use rusk::http::domain::validation::rules::ValidationRule;
//! use rusk::http::domain::validation::context::ValidationContext;
//! # use rusk::http::domain::testing;
//!
//! let validator = BlockHashValidator::new();
//! let mut ctx = ValidationContext::new();
//!
//! let block_hash = testing::create_test_block_hash();
//! if let TargetIdentifier::Block(hash) = block_hash {
//!     assert!(validator.check(&hash, &mut ctx).is_ok());
//! }
//! ```

use crate::http::domain::error::{
    CommonErrorAttributes, DomainError, ValidationError, WithContext,
};
use crate::http::domain::types::identifier::{
    BlockHash, ContractId, IdentifierBytes, SessionId, TargetIdentifier,
    TransactionHash,
};
use crate::http::domain::types::value::RuesValue;
use crate::http::domain::validation::context::ValidationContext;
use crate::http::domain::validation::rules::ValidationRule;

/// Validates session ID format (16 bytes).
///
/// Ensures that session IDs:
/// - Are binary values
/// - Are exactly 16 bytes in length
///
/// This validator only checks format requirements for routing,
/// not the actual validity of the session.
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::validation::rules::identifier::SessionIdValidator;
/// use rusk::http::domain::validation::rules::ValidationRule;
/// use rusk::http::domain::validation::context::ValidationContext;
/// # use rusk::http::domain::testing;
///
/// let validator = SessionIdValidator::new();
/// let mut ctx = ValidationContext::new();
///
/// // Valid session ID
/// let valid_id = testing::create_test_session_id();
/// assert!(validator.check(&valid_id, &mut ctx).is_ok());
///
/// // Invalid session ID (wrong type)
/// let invalid_session_id = testing::create_invalid_session_id();
/// assert!(validator.check(&invalid_session_id, &mut ctx).is_err());
/// ```
#[derive(Debug, Default)]
pub struct SessionIdValidator;

impl SessionIdValidator {
    /// Creates a new session ID validator
    pub fn new() -> Self {
        Self
    }
}

impl ValidationRule<SessionId> for SessionIdValidator {
    fn check(
        &self,
        id: &SessionId,
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("session_id");
        let result = validate_binary_length(id.value(), 16, "session_id");
        ctx.complete_validation("session_id", &result);
        result
    }
}

/// Validates block hash format (32 bytes).
///
/// Ensures that block hashes:
/// - Are binary values
/// - Are exactly 32 bytes in length
///
/// This validator only checks format requirements for routing,
/// not the actual validity of the block hash.
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::validation::rules::identifier::BlockHashValidator;
/// use rusk::http::domain::types::identifier::TargetIdentifier;
/// use rusk::http::domain::validation::rules::ValidationRule;
/// use rusk::http::domain::validation::context::ValidationContext;
/// # use rusk::http::domain::testing;
///
/// let validator = BlockHashValidator::new();
/// let mut ctx = ValidationContext::new();
///
/// // Valid block hash
/// let valid_hash = testing::create_test_block_hash();
/// if let TargetIdentifier::Block(hash) = valid_hash {
///     assert!(validator.check(&hash, &mut ctx).is_ok());
/// }
///
/// // Invalid block hash (wrong type)
/// let invalid_block_hash = testing::create_invalid_block_hash();
/// assert!(validator.check(&invalid_block_hash, &mut ctx).is_err());
/// ```
#[derive(Debug, Default)]
pub struct BlockHashValidator;

impl BlockHashValidator {
    /// Creates a new block hash validator
    pub fn new() -> Self {
        Self
    }
}

impl ValidationRule<BlockHash> for BlockHashValidator {
    fn check(
        &self,
        hash: &BlockHash,
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("block_hash");
        let result = validate_binary_length(hash.value(), 32, "block_hash");
        ctx.complete_validation("block_hash", &result);
        result
    }
}

/// Validates transaction hash format (32 bytes).
///
/// Ensures that transaction hashes:
/// - Are binary values
/// - Are exactly 32 bytes in length
///
/// This validator only checks format requirements for routing,
/// not the actual validity of the transaction hash.
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::validation::rules::identifier::TransactionHashValidator;
/// use rusk::http::domain::types::identifier::TargetIdentifier;
/// use rusk::http::domain::validation::rules::ValidationRule;
/// use rusk::http::domain::validation::context::ValidationContext;
/// # use rusk::http::domain::testing;
///
/// let validator = TransactionHashValidator::new();
/// let mut ctx = ValidationContext::new();
///
/// // Valid transaction hash
/// let valid_hash = testing::create_test_tx_hash();
/// if let TargetIdentifier::Transaction(hash) = valid_hash {
///     assert!(validator.check(&hash, &mut ctx).is_ok());
/// }
///
/// // Invalid transaction hash (wrong type)
/// let invalid_transaction_hash = testing::create_invalid_transaction_hash();
/// assert!(validator.check(&invalid_transaction_hash, &mut ctx).is_err());
/// ```
#[derive(Debug, Default)]
pub struct TransactionHashValidator;

impl TransactionHashValidator {
    /// Creates a new transaction hash validator
    pub fn new() -> Self {
        Self
    }
}

impl ValidationRule<TransactionHash> for TransactionHashValidator {
    fn check(
        &self,
        hash: &TransactionHash,
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("transaction_hash");
        let result =
            validate_binary_length(hash.value(), 32, "transaction_hash");
        ctx.complete_validation("transaction_hash", &result);
        result
    }
}

/// Validates contract ID format (variable length binary).
///
/// Ensures that contract IDs:
/// - Are binary values
/// - Can be any length
///
/// This validator only checks that the value is binary for routing purposes,
/// not the actual validity or existence of the contract.
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::validation::rules::identifier::ContractIdValidator;
/// use rusk::http::domain::types::identifier::TargetIdentifier;
/// use rusk::http::domain::validation::rules::ValidationRule;
/// use rusk::http::domain::validation::context::ValidationContext;
/// # use rusk::http::domain::testing;
///
/// let validator = ContractIdValidator::new();
/// let mut ctx = ValidationContext::new();
///
/// // Valid contract ID
/// let valid_id = testing::create_test_contract_id();
/// if let TargetIdentifier::Contract(id) = valid_id {
///     assert!(validator.check(&id, &mut ctx).is_ok());
/// }
///
/// // Invalid contract ID (wrong type)
/// let invalid_contract_id = testing::create_invalid_contract_id();
/// assert!(validator.check(&invalid_contract_id, &mut ctx).is_err());
/// ```
#[derive(Debug, Default)]
pub struct ContractIdValidator;

impl ContractIdValidator {
    /// Creates a new contract ID validator
    pub fn new() -> Self {
        Self
    }
}

impl ValidationRule<ContractId> for ContractIdValidator {
    fn check(
        &self,
        id: &ContractId,
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("contract_id");
        let result = validate_binary_type(id.value(), "contract_id");
        ctx.complete_validation("contract_id", &result);
        result
    }
}

/// Helper function to validate binary length.
///
/// # Arguments
///
/// * `value` - Value to validate
/// * `expected_len` - Expected length in bytes
/// * `field` - Field name for error messages
///
/// # Returns
///
/// * `Ok(())` - If value is binary and has correct length
/// * `Err(DomainError)` - If validation fails
fn validate_binary_length(
    value: &RuesValue,
    expected_len: usize,
    field: &str,
) -> Result<(), DomainError> {
    match value {
        RuesValue::Binary(bytes) if bytes.len() == expected_len => Ok(()),
        RuesValue::Binary(bytes) => Err(ValidationError::InvalidFieldValue {
            field: field.into(),
            reason: format!(
                "Invalid length: expected {}, got {}",
                expected_len,
                bytes.len()
            ),
        }
        .into()),
        _ => Err(ValidationError::InvalidFieldValue {
            field: field.into(),
            reason: "Expected binary value".into(),
        }
        .into()),
    }
}

fn validate_binary_type(
    value: &RuesValue,
    field: &str,
) -> Result<(), DomainError> {
    match value {
        RuesValue::Binary(_) => Ok(()),
        _ => Err(ValidationError::InvalidFieldValue {
            field: field.into(),
            reason: "Expected binary value".into(),
        }
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::domain::testing;
    use crate::http::domain::types::identifier::TargetIdentifier;
    use bytes::Bytes;
    use serde_json::json;

    fn setup_context() -> ValidationContext {
        ValidationContext::new()
    }

    #[test]
    fn test_session_id_validation() {
        let validator = SessionIdValidator;
        let mut ctx = setup_context();

        // Positive case
        let valid_id = testing::create_test_session_id();
        assert!(validator.check(&valid_id, &mut ctx).is_ok());

        // Different valid ID
        let another_valid_id = testing::create_different_session_id();
        assert!(validator.check(&another_valid_id, &mut ctx).is_ok());

        // Wrong type (using invalid identifier helper)
        let invalid_id = SessionId(testing::create_invalid_identifier());
        let err = validator.check(&invalid_id, &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation(ValidationError::InvalidFieldValue { field, .. })
            if field == "session_id"
        ));
    }

    #[test]
    fn test_block_hash_validation() {
        let validator = BlockHashValidator;
        let mut ctx = setup_context();

        // Positive case
        let valid_hash = testing::create_test_block_hash();
        if let TargetIdentifier::Block(hash) = valid_hash {
            assert!(validator.check(&hash, &mut ctx).is_ok());
        }

        // Different valid hash
        let different_hash = testing::create_different_block_hash();
        if let TargetIdentifier::Block(hash) = different_hash {
            assert!(validator.check(&hash, &mut ctx).is_ok());
        }

        // Invalid type
        let invalid_hash = BlockHash(testing::create_invalid_identifier());
        let err = validator.check(&invalid_hash, &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation(ValidationError::InvalidFieldValue { field, .. })
            if field == "block_hash"
        ));
    }

    #[test]
    fn test_transaction_hash_validation() {
        let validator = TransactionHashValidator;
        let mut ctx = setup_context();

        // Positive case
        let valid_hash = testing::create_test_tx_hash();
        if let TargetIdentifier::Transaction(hash) = valid_hash {
            assert!(validator.check(&hash, &mut ctx).is_ok());
        }

        // Different valid hash
        let different_hash = testing::create_different_tx_hash();
        if let TargetIdentifier::Transaction(hash) = different_hash {
            assert!(validator.check(&hash, &mut ctx).is_ok());
        }

        // Invalid type
        let invalid_hash =
            TransactionHash(testing::create_invalid_identifier());
        let err = validator.check(&invalid_hash, &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation(ValidationError::InvalidFieldValue { field, .. })
            if field == "transaction_hash"
        ));
    }

    #[test]
    fn test_contract_id_validation() {
        let validator = ContractIdValidator;
        let mut ctx = setup_context();

        // Positive case
        let valid_id = testing::create_test_contract_id();
        if let TargetIdentifier::Contract(id) = valid_id {
            assert!(validator.check(&id, &mut ctx).is_ok());
        }

        // Different valid ID
        let different_id = testing::create_different_contract_id();
        if let TargetIdentifier::Contract(id) = different_id {
            assert!(validator.check(&id, &mut ctx).is_ok());
        }

        // Invalid type
        let invalid_id = ContractId(testing::create_invalid_identifier());
        let err = validator.check(&invalid_id, &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation(ValidationError::InvalidFieldValue { field, .. })
            if field == "contract_id"
        ));
    }

    #[test]
    fn test_validation_metrics() {
        let validator = SessionIdValidator;
        let mut ctx = setup_context();

        // Successful validation should record metrics
        let valid_id = testing::create_test_session_id();
        assert!(validator.check(&valid_id, &mut ctx).is_ok());

        // Failed validation should record error metrics
        let invalid_id = SessionId(testing::create_invalid_identifier());
        assert!(validator.check(&invalid_id, &mut ctx).is_err());
    }

    #[test]
    fn test_validate_binary_length() {
        // Positive cases
        assert!(validate_binary_length(
            &RuesValue::Binary(Bytes::from(vec![1; 16])),
            16,
            "test"
        )
        .is_ok());

        // Edge case - empty binary with expected length 0
        assert!(validate_binary_length(
            &RuesValue::Binary(Bytes::from(vec![])),
            0,
            "test"
        )
        .is_ok());

        // Negative cases

        // Wrong length
        let err = validate_binary_length(
            &RuesValue::Binary(Bytes::from(vec![1; 15])),
            16,
            "test",
        )
        .unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation(ValidationError::InvalidFieldValue { field, reason })
            if field == "test" && reason.contains("Invalid length")
        ));

        // Wrong type
        let wrong_types = vec![
            RuesValue::Json(json!({"test": true})),
            RuesValue::Text("test".into()),
            RuesValue::GraphQL("query".into()),
            RuesValue::Proof(Bytes::from(vec![1])),
        ];

        for value in wrong_types {
            let err = validate_binary_length(&value, 16, "test").unwrap_err();
            assert!(matches!(
                err,
                DomainError::Validation(ValidationError::InvalidFieldValue { field, reason })
                if field == "test" && reason == "Expected binary value"
            ));
        }
    }

    #[test]
    fn test_validate_binary_type() {
        // Positive cases
        assert!(validate_binary_type(
            &RuesValue::Binary(Bytes::from(vec![1; 16])),
            "test"
        )
        .is_ok());

        // Edge case - empty binary
        assert!(validate_binary_type(
            &RuesValue::Binary(Bytes::from(vec![])),
            "test"
        )
        .is_ok());

        // Negative cases - wrong types
        let wrong_types = vec![
            RuesValue::Json(json!({"test": true})),
            RuesValue::Text("test".into()),
            RuesValue::GraphQL("query".into()),
            RuesValue::Proof(Bytes::from(vec![1])),
        ];

        for value in wrong_types {
            let err = validate_binary_type(&value, "test").unwrap_err();
            assert!(matches!(
                err,
                DomainError::Validation(ValidationError::InvalidFieldValue { field, reason })
                if field == "test" && reason == "Expected binary value"
            ));
        }
    }
}
