// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Validation rules for RUES protocol paths.
//!
//! This module provides validation rules for RUES paths according to the
//! specification. All rules focus on structural validation, leaving
//! transport-level concerns to the HTTP layer.
//!
//! # Path Format
//!
//! RUES paths follow these formats:
//! - Modern: `/on/\[target\]/\[topic\]` or `/on/\[target\]:\[id\]/\[topic\]`
//! - Legacy: `/on/host:\[data\]/\[topic\]` or `/on/debugger:\[data\]/\[topic\]`
//!
//! # Validation Rules
//!
//! ## Topic Format (`TopicFormatRule`)
//!
//! Topics must:
//! - Not be empty
//! - Contain only ASCII alphanumeric characters (a-z, A-Z, 0-9)
//! - Allow limited special characters (-, _, .)
//! - Not contain URL-reserved characters or whitespace
//!
//! ```rust
//! use rusk::http::domain::types::path::{RuesPath, Target};
//! use rusk::http::domain::validation::rules::ValidationRule;
//! use rusk::http::domain::validation::rules::path::TopicFormatRule;
//! use rusk::http::domain::validation::context::ValidationContext;
//! # use rusk::http::domain::testing;
//!
//! let rule = TopicFormatRule::new();
//! let mut ctx = ValidationContext::new();
//!
//! // Valid topics
//! let valid_path = RuesPath::new_modern(
//!     Target::Blocks,
//!     None,
//!     testing::create_test_topic("block-accepted.v1")
//! );
//! assert!(rule.check(&valid_path, &mut ctx).is_ok());
//!
//! // Invalid topics: contains '/' character
//! let invalid_path = RuesPath::new_modern(
//!     Target::Blocks,
//!     None,
//!     testing::create_test_topic("invalid/topic")
//! );
//! assert!(rule.check(&invalid_path, &mut ctx).is_err());
//! ```
//!
//! ## Target Type (`TargetTypeRule`)
//!
//! Validates that:
//! - Only supported targets have IDs (blocks, transactions, contracts)
//! - Legacy targets have non-empty data
//!
//! ```rust
//! use rusk::http::domain::types::path::{RuesPath, Target};
//! use rusk::http::domain::validation::rules::path::TargetTypeRule;
//! use rusk::http::domain::validation::rules::ValidationRule;
//! use rusk::http::domain::validation::context::ValidationContext;
//! # use rusk::http::domain::testing;
//!
//! let rule = TargetTypeRule::new();
//! let mut ctx = ValidationContext::new();
//!
//! // Valid: blocks can have IDs
//! let valid_path = RuesPath::new_modern(
//!     Target::Blocks,
//!     Some(testing::create_test_block_hash()),
//!     testing::create_test_topic("accepted")
//! );
//! assert!(rule.check(&valid_path, &mut ctx).is_ok());
//!
//! // Invalid: node cannot have ID
//! let invalid_path = RuesPath::new_modern(
//!     Target::Node,
//!     Some(testing::create_test_block_hash()),
//!     testing::create_test_topic("info")
//! );
//! assert!(rule.check(&invalid_path, &mut ctx).is_err());
//! ```
//!
//! ## Target ID (`TargetIdRule`)
//!
//! Ensures that:
//! - Block hashes are valid 32-byte values
//! - Transaction hashes are valid 32-byte values
//! - Contract IDs are valid binary values
//! - ID type matches the target type
//!
//! ```rust
//! use rusk::http::domain::types::path::{RuesPath, Target};
//! use rusk::http::domain::validation::rules::path::TargetIdRule;
//! use rusk::http::domain::validation::rules::ValidationRule;
//! use rusk::http::domain::validation::context::ValidationContext;
//! # use rusk::http::domain::testing;
//!
//! let rule = TargetIdRule::new();
//! let mut ctx = ValidationContext::new();
//!
//! // Valid: correct ID type for target
//! let valid_path = RuesPath::new_modern(
//!     Target::Blocks,
//!     Some(testing::create_test_block_hash()),
//!     testing::create_test_topic("accepted")
//! );
//! assert!(rule.check(&valid_path, &mut ctx).is_ok());
//!
//! // Invalid: transaction hash for block target
//! let invalid_path = RuesPath::new_modern(
//!     Target::Blocks,
//!     Some(testing::create_test_tx_hash()),
//!     testing::create_test_topic("accepted")
//! );
//! assert!(rule.check(&invalid_path, &mut ctx).is_err());
//! ```
//!
//! ## Topic Validity (`TopicValidityRule`)
//!
//! Validates target-specific topic constraints:
//! - GraphQL: only "query" topic
//! - Node: "info", "provisioners", "crs"
//! - Network: only "peers" topic
//! - Other targets: any valid topic
//!
//! ```rust
//! use rusk::http::domain::types::path::{RuesPath, Target};
//! use rusk::http::domain::validation::rules::path::TopicValidityRule;
//! use rusk::http::domain::validation::rules::ValidationRule;
//! use rusk::http::domain::validation::context::ValidationContext;
//! # use rusk::http::domain::testing;
//!
//! let rule = TopicValidityRule::new();
//! let mut ctx = ValidationContext::new();
//!
//! // Valid GraphQL topic
//! let valid_path = RuesPath::new_modern(
//!     Target::GraphQL,
//!     None,
//!     testing::create_test_topic("query")
//! );
//! assert!(rule.check(&valid_path, &mut ctx).is_ok());
//!
//! // Invalid GraphQL topic: GraphQL only accepts "query"
//! let invalid_path = RuesPath::new_modern(
//!     Target::GraphQL,
//!     None,
//!     testing::create_test_topic("invalid")
//! );
//! assert!(rule.check(&invalid_path, &mut ctx).is_err());
//! ```

use crate::http::domain::error::{DomainError, ValidationError};
use crate::http::domain::types::identifier::TargetIdentifier;
use crate::http::domain::types::path::{
    LegacyTarget, RuesPath, Target, TargetSpecifier,
};
use crate::http::domain::validation::context::ValidationContext;
use crate::http::domain::validation::rules::identifier::{
    BlockHashValidator, ContractIdValidator, TransactionHashValidator,
};
use crate::http::domain::validation::rules::ValidationRule;

/// Validates topic format and characters in RUES paths.
///
/// Ensures that topics:
/// - Are not empty
/// - Contain only ASCII alphanumeric characters (a-z, A-Z, 0-9)
/// - Allow limited special characters (-, _, .)
/// - Don't contain URL-reserved characters or whitespace
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::types::path::{RuesPath, Target};
/// use rusk::http::domain::validation::rules::path::TopicFormatRule;
/// use rusk::http::domain::validation::rules::ValidationRule;
/// use rusk::http::domain::validation::context::ValidationContext;
/// # use rusk::http::domain::testing;
///
/// let rule = TopicFormatRule::new();
/// let mut ctx = ValidationContext::new();
///
/// // Valid topic
/// let topic = testing::create_test_topic("block-accepted.v1");
/// let valid_path = RuesPath::new_modern(
///     Target::Blocks,
///     None,
///     topic
/// );
/// assert!(rule.check(&valid_path, &mut ctx).is_ok());
///
/// // Invalid topic (contains '/')
/// let topic = testing::create_test_topic("invalid/topic");
/// let invalid_path = RuesPath::new_modern(
///     Target::Blocks,
///     None,
///     topic
/// );
/// assert!(rule.check(&invalid_path, &mut ctx).is_err());
/// ```
#[derive(Debug, Default)]
pub struct TopicFormatRule;

impl TopicFormatRule {
    /// Creates a new topic format validator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::validation::rules::path::TopicFormatRule;
    ///
    /// let validator = TopicFormatRule::new();
    /// ```
    pub fn new() -> Self {
        Self
    }
}

impl ValidationRule<RuesPath> for TopicFormatRule {
    fn check(
        &self,
        path: &RuesPath,
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("topic_format");

        let result = if path.topic().is_empty() {
            Err(ValidationError::InvalidFormat("Empty topic".into()).into())
        } else if !path.topic().chars().all(|c| {
            c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'
        }) {
            Err(ValidationError::InvalidFormat(
                "Topic contains invalid characters. Only ASCII alphanumeric, '-', '_', and '.' are allowed".into()
            ).into())
        } else {
            Ok(())
        };

        ctx.complete_validation("topic_format", &result);
        result
    }
}

/// Validates target type and permissions in RUES paths.
///
/// Ensures that:
/// - Only supported targets have IDs (blocks, transactions, contracts)
/// - Other targets (node, network, graphql, prover) don't have IDs
/// - Legacy targets have non-empty data
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::types::path::{RuesPath, Target};
/// use rusk::http::domain::validation::rules::path::TargetTypeRule;
/// use rusk::http::domain::validation::rules::ValidationRule;
/// use rusk::http::domain::validation::context::ValidationContext;
/// # use rusk::http::domain::testing;
///
/// let rule = TargetTypeRule::new();
/// let mut ctx = ValidationContext::new();
///
/// // Valid: blocks can have IDs
/// let topic = testing::create_test_topic("accepted");
/// let valid_path = RuesPath::new_modern(
///     Target::Blocks,
///     Some(testing::create_test_block_hash()),
///     topic
/// );
/// assert!(rule.check(&valid_path, &mut ctx).is_ok());
///
/// // Invalid: node cannot have ID
/// let topic = testing::create_test_topic("info");
/// let invalid_path = RuesPath::new_modern(
///     Target::Node,
///     Some(testing::create_test_block_hash()),
///     topic
/// );
/// assert!(rule.check(&invalid_path, &mut ctx).is_err());
/// ```
#[derive(Debug, Default)]
pub struct TargetTypeRule;

impl TargetTypeRule {
    /// Creates a new target type validator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::validation::rules::path::TargetTypeRule;
    ///
    /// let validator = TargetTypeRule::new();
    /// ```
    pub fn new() -> Self {
        Self
    }
}

impl ValidationRule<RuesPath> for TargetTypeRule {
    fn check(
        &self,
        path: &RuesPath,
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("target_type");

        let result = match path.target() {
            TargetSpecifier::Modern { target, id } => {
                if id.is_some() && !target.supports_id() {
                    Err(ValidationError::InvalidFormat(format!(
                        "Target {} does not support IDs",
                        target
                    ))
                    .into())
                } else {
                    Ok(())
                }
            }
            TargetSpecifier::Legacy(target) => match target {
                LegacyTarget::Host(data) | LegacyTarget::Debugger(data)
                    if data.is_empty() =>
                {
                    Err(ValidationError::InvalidFormat(
                        "Empty legacy target data".into(),
                    )
                    .into())
                }
                _ => Ok(()),
            },
        };

        ctx.complete_validation("target_type", &result);
        result
    }
}

/// Validates target ID format in RUES paths.
///
/// Ensures that:
/// - Block hashes are valid 32-byte values
/// - Transaction hashes are valid 32-byte values
/// - Contract IDs are valid binary values
/// - ID type matches the target type
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::types::path::{RuesPath, Target};
/// use rusk::http::domain::validation::rules::path::TargetIdRule;
/// use rusk::http::domain::validation::rules::ValidationRule;
/// use rusk::http::domain::validation::context::ValidationContext;
/// # use rusk::http::domain::testing;
///
/// let rule = TargetIdRule::new();
/// let mut ctx = ValidationContext::new();
///
/// // Valid: block hash for blocks target
/// let topic = testing::create_test_topic("accepted");
/// let valid_path = RuesPath::new_modern(
///     Target::Blocks,
///     Some(testing::create_test_block_hash()),
///     topic
/// );
/// assert!(rule.check(&valid_path, &mut ctx).is_ok());
///
/// // Invalid: transaction hash for blocks target
/// let topic = testing::create_test_topic("accepted");
/// let invalid_path = RuesPath::new_modern(
///     Target::Blocks,
///     Some(testing::create_test_tx_hash()),
///     topic
/// );
/// assert!(rule.check(&invalid_path, &mut ctx).is_err());
/// ```
#[derive(Debug)]
pub struct TargetIdRule {
    block_validator: BlockHashValidator,
    tx_validator: TransactionHashValidator,
    contract_validator: ContractIdValidator,
}

impl TargetIdRule {
    /// Creates a new target ID validator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::validation::rules::path::TargetIdRule;
    /// use rusk::http::domain::validation::rules::ValidationRule;
    ///
    /// let validator = TargetIdRule::new();
    /// ```
    pub fn new() -> Self {
        Self {
            block_validator: BlockHashValidator::new(),
            tx_validator: TransactionHashValidator::new(),
            contract_validator: ContractIdValidator::new(),
        }
    }
}

impl Default for TargetIdRule {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationRule<RuesPath> for TargetIdRule {
    fn check(
        &self,
        path: &RuesPath,
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("target_id");

        let result = if let TargetSpecifier::Modern {
            target,
            id: Some(id),
        } = path.target()
        {
            match (target, id) {
                (Target::Blocks, TargetIdentifier::Block(hash)) => {
                    self.block_validator.check(hash, ctx)
                }
                (Target::Transactions, TargetIdentifier::Transaction(hash)) => {
                    self.tx_validator.check(hash, ctx)
                }
                (Target::Contracts, TargetIdentifier::Contract(id)) => {
                    self.contract_validator.check(id, ctx)
                }
                _ => Err(ValidationError::InvalidFormat(format!(
                    "Invalid ID type for target {}",
                    target
                ))
                .into()),
            }
        } else {
            Ok(())
        };

        ctx.complete_validation("target_id", &result);
        result
    }
}

/// Validates target-specific topic constraints in RUES paths.
///
/// Ensures that:
/// - GraphQL paths only use "query" topic
/// - Node paths use valid operation topics ("info", "provisioners", "crs")
/// - Network paths use valid operation topics ("peers")
/// - Other targets accept any valid topic
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::types::path::{RuesPath, Target};
/// use rusk::http::domain::validation::rules::path::TopicValidityRule;
/// use rusk::http::domain::validation::rules::ValidationRule;
/// use rusk::http::domain::validation::context::ValidationContext;
/// # use rusk::http::domain::testing;
///
/// let rule = TopicValidityRule::new();
/// let mut ctx = ValidationContext::new();
///
/// // Valid GraphQL topic
/// let topic = testing::create_test_topic("query");
/// let valid_path = RuesPath::new_modern(
///     Target::GraphQL,
///     None,
///     topic
/// );
/// assert!(rule.check(&valid_path, &mut ctx).is_ok());
///
/// // Invalid GraphQL topic
/// let topic = testing::create_test_topic("invalid");
/// let invalid_path = RuesPath::new_modern(
///     Target::GraphQL,
///     None,
///     topic
/// );
/// assert!(rule.check(&invalid_path, &mut ctx).is_err());
/// ```
#[derive(Debug, Default)]
pub struct TopicValidityRule;

impl TopicValidityRule {
    /// Creates a new topic validity validator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::validation::rules::path::TopicValidityRule;
    /// use rusk::http::domain::validation::rules::ValidationRule;
    ///
    /// let validator = TopicValidityRule::new();
    /// ```
    pub fn new() -> Self {
        Self
    }
}

impl ValidationRule<RuesPath> for TopicValidityRule {
    fn check(
        &self,
        path: &RuesPath,
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("topic_validity");

        let result =
            if let TargetSpecifier::Modern { target, .. } = path.target() {
                match target {
                    Target::GraphQL if path.topic() != "query" => {
                        Err(ValidationError::InvalidFormat(
                            "GraphQL only supports 'query' topic".into(),
                        )
                        .into())
                    }
                    Target::Node => {
                        let valid_topics = ["info", "provisioners", "crs"];
                        if !valid_topics.contains(&path.topic()) {
                            Err(ValidationError::InvalidFormat(format!(
                                "Invalid node topic. Expected one of: {}",
                                valid_topics.join(", ")
                            ))
                            .into())
                        } else {
                            Ok(())
                        }
                    }
                    Target::Network => {
                        if path.topic() != "peers" {
                            Err(ValidationError::InvalidFormat(
                                "Network only supports 'peers' topic".into(),
                            )
                            .into())
                        } else {
                            Ok(())
                        }
                    }
                    _ => Ok(()),
                }
            } else {
                Ok(())
            };

        ctx.complete_validation("topic_validity", &result);
        result.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::domain::testing;
    use crate::http::domain::types::path::Target;
    use crate::http::domain::validation::context::ValidationContext;

    fn setup_context() -> ValidationContext {
        ValidationContext::new()
    }

    #[test]
    fn test_topic_format_rule() {
        let rule = TopicFormatRule::new();
        let mut ctx = setup_context();

        // Positive cases
        let valid_cases = [
            "accepted",          // Simple ASCII
            "item-added",        // With hyphen
            "contract_deployed", // With underscore
            "v1.0",              // With dot
            "camelCase123",      // Mixed case with numbers
            "a",                 // Single char
        ];

        for topic in valid_cases {
            let path =
                testing::create_test_rues_path(Target::Blocks, None, topic);
            assert!(
                rule.check(&path, &mut ctx).is_ok(),
                "Topic should be valid: {}",
                topic
            );
        }

        // Negative cases
        let invalid_cases = [
            "",                  // Empty
            "invalid/topic",     // Contains /
            "invalid:topic",     // Contains :
            "invalid topic",     // Contains space
            "invalid\ttopic",    // Contains tab
            "invalid\ntopic",    // Contains newline
            "topic💡",           // Contains emoji
            "тема",              // Non-ASCII
            "topic@example",     // Contains @
            "topic#1",           // Contains #
            "topic?param=value", // Contains URL chars
            "topic&other",       // Contains &
        ];

        for topic in invalid_cases {
            let path =
                testing::create_test_rues_path(Target::Blocks, None, topic);
            assert!(
                rule.check(&path, &mut ctx).is_err(),
                "Topic should be invalid: {}",
                topic
            );
        }
    }

    #[test]
    fn test_target_type_rule() {
        let rule = TargetTypeRule::new();
        let mut ctx = setup_context();

        // Test targets that support IDs
        let id_supporting_targets = [
            (Target::Blocks, testing::create_test_block_hash()),
            (Target::Transactions, testing::create_test_tx_hash()),
            (Target::Contracts, testing::create_test_contract_id()),
        ];

        for (target, id) in id_supporting_targets {
            let path =
                testing::create_test_rues_path(target, Some(id), "topic");
            assert!(rule.check(&path, &mut ctx).is_ok());
        }

        // Test targets that don't support IDs
        let non_id_targets = [
            Target::Node,
            Target::Network,
            Target::GraphQL,
            Target::Prover,
        ];

        for target in non_id_targets {
            // Without ID (should be valid)
            let path = testing::create_test_rues_path(target, None, "topic");
            assert!(rule.check(&path, &mut ctx).is_ok());

            // With ID (should be invalid)
            let path = testing::create_test_rues_path(
                target,
                Some(testing::create_test_block_hash()),
                "topic",
            );
            assert!(rule.check(&path, &mut ctx).is_err());
        }

        // Test legacy paths
        let valid_legacy = testing::create_test_legacy_path(
            LegacyTarget::Host("system".into()),
            "topic",
        );
        assert!(rule.check(&valid_legacy, &mut ctx).is_ok());

        let invalid_legacy = testing::create_test_legacy_path(
            LegacyTarget::Host("".into()),
            "topic",
        );
        assert!(rule.check(&invalid_legacy, &mut ctx).is_err());
    }

    #[test]
    fn test_target_id_rule() {
        let rule = TargetIdRule::new();
        let mut ctx = setup_context();

        // Valid cases
        let valid_cases = [
            (Target::Blocks, testing::create_test_block_hash()),
            (Target::Transactions, testing::create_test_tx_hash()),
            (Target::Contracts, testing::create_test_contract_id()),
        ];

        for (target, id) in valid_cases {
            let path =
                testing::create_test_rues_path(target, Some(id), "topic");
            assert!(rule.check(&path, &mut ctx).is_ok());
        }

        // Invalid cases - wrong ID type for target
        let invalid_cases = [
            (Target::Blocks, testing::create_test_tx_hash()),
            (Target::Blocks, testing::create_test_contract_id()),
            (Target::Transactions, testing::create_test_block_hash()),
            (Target::Transactions, testing::create_test_contract_id()),
            (Target::Contracts, testing::create_test_block_hash()),
            (Target::Contracts, testing::create_test_tx_hash()),
        ];

        for (target, id) in invalid_cases {
            let path =
                testing::create_test_rues_path(target, Some(id), "topic");
            assert!(rule.check(&path, &mut ctx).is_err());
        }

        // Test path without ID
        let path =
            testing::create_test_rues_path(Target::Blocks, None, "topic");
        assert!(rule.check(&path, &mut ctx).is_ok());
    }

    #[test]
    fn test_topic_validity_rule() {
        let rule = TopicValidityRule::new();
        let mut ctx = setup_context();

        // GraphQL topics
        let valid_graphql =
            testing::create_test_rues_path(Target::GraphQL, None, "query");
        assert!(rule.check(&valid_graphql, &mut ctx).is_ok());

        let invalid_graphql =
            testing::create_test_rues_path(Target::GraphQL, None, "invalid");
        assert!(rule.check(&invalid_graphql, &mut ctx).is_err());

        // Node topics
        let valid_node_topics = ["info", "provisioners", "crs"];
        for topic in valid_node_topics {
            let path =
                testing::create_test_rues_path(Target::Node, None, topic);
            assert!(rule.check(&path, &mut ctx).is_ok());
        }

        let invalid_node =
            testing::create_test_rues_path(Target::Node, None, "invalid");
        assert!(rule.check(&invalid_node, &mut ctx).is_err());

        // Network topics
        let valid_network =
            testing::create_test_rues_path(Target::Network, None, "peers");
        assert!(rule.check(&valid_network, &mut ctx).is_ok());

        let invalid_network =
            testing::create_test_rues_path(Target::Network, None, "invalid");
        assert!(rule.check(&invalid_network, &mut ctx).is_err());

        // Other targets (should accept any valid topic)
        let unrestricted_targets = [
            Target::Blocks,
            Target::Transactions,
            Target::Contracts,
            Target::Prover,
        ];
        for target in unrestricted_targets {
            let path =
                testing::create_test_rues_path(target, None, "any-topic");
            assert!(rule.check(&path, &mut ctx).is_ok());
        }

        // Legacy paths (should accept any valid topic)
        let legacy_path = testing::create_test_legacy_path(
            LegacyTarget::Host("system".into()),
            "any-topic",
        );
        assert!(rule.check(&legacy_path, &mut ctx).is_ok());
    }
}
