// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! RUES path types and structures.
//!
//! This module provides the core types for representing RUES paths. Note that
//! path validation and string conversion should be performed through the
//! processor pipeline, as these types represent only the validated structure.
//!
//! # Examples
//!
//! Creating and using RUES paths:
//! ```rust
//! use rusk::http::domain::types::path::{Target, RuesPath, LegacyTarget, TargetSpecifier};
//! # use rusk::http::domain::testing;
//!
//! // Modern path with contract target
//! let contract_id = testing::create_test_contract_id();
//! let topic = testing::create_test_topic("deploy");
//! let contract_path = RuesPath::new_modern(
//!     Target::Contracts,
//!     Some(contract_id.clone()),
//!     topic
//! );
//!
//! assert!(contract_path.is_modern());
//! assert!(contract_path.matches_target(Target::Contracts));
//! assert!(contract_path.matches_topic("deploy"));
//!
//! // Simple path without ID
//! let topic = testing::create_test_topic("info");
//! let node_path = RuesPath::new_modern(
//!     Target::Node,
//!     None,
//!     topic
//! );
//!
//! assert!(!Target::Node.supports_id());
//! assert_eq!(node_path.to_string(), "/on/node/info");
//! ```

use crate::http::domain::types::identifier::{
    BlockHash, ContractId, TargetIdentifier, TransactionHash,
};
use std::fmt;

/// RUES path representation.
///
/// This type represents a validated RUES path structure. Note that validation
/// and conversion from strings should be performed through the processor
/// pipeline.
///
/// # Path Format
///
/// RUES paths follow the format `/on/[target]/[topic]` where:
/// - `target` can be either a modern target (with optional ID) or a legacy
///   target
/// - `topic` specifies the event or operation type
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::types::path::{Target, RuesPath, LegacyTarget};
/// # use rusk::http::domain::testing;
///
/// // Modern path with block hash
/// let block_id = testing::create_test_block_hash();
/// let topic = testing::create_test_topic("accepted");
/// let block_path = RuesPath::new_modern(
///     Target::Blocks,
///     Some(block_id),
///     topic
/// );
///
/// assert!(block_path.is_modern());
/// assert!(block_path.matches_target(Target::Blocks));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuesPath {
    /// Target specification (modern or legacy)
    target: TargetSpecifier,
    /// Topic for this path
    topic: Topic,
}

impl RuesPath {
    pub fn new(target: TargetSpecifier, topic: Topic) -> Self {
        Self { target, topic }
    }

    /// Creates a new modern RUES path.
    ///
    /// Modern paths follow the format `/on/[target]/[topic]` or
    /// `/on/[target]:[id]/[topic]`.
    ///
    /// # Arguments
    ///
    /// * `target` - The target component (blocks, transactions, etc.)
    /// * `id` - Optional target identifier (block hash, transaction hash, etc.)
    /// * `topic` - The topic identifying the event or operation
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::types::path::{RuesPath, Target};
    /// # use rusk::http::domain::testing;
    ///
    /// // Path without ID
    /// let topic = testing::create_test_topic("info");
    /// let node_path = RuesPath::new_modern(
    ///     Target::Node,
    ///     None,
    ///     topic
    /// );
    ///
    /// // Path with block hash
    /// let block_id = testing::create_test_block_hash();
    /// let topic = testing::create_test_topic("accepted");
    /// let block_path = RuesPath::new_modern(
    ///     Target::Blocks,
    ///     Some(block_id),
    ///     topic
    /// );
    /// ```
    pub fn new_modern(
        target: Target,
        id: Option<TargetIdentifier>,
        topic: Topic,
    ) -> Self {
        Self {
            target: TargetSpecifier::Modern { target, id },
            topic,
        }
    }

    /// Creates a new legacy RUES path.
    ///
    /// Legacy paths support backward compatibility with older systems.
    ///
    /// # Arguments
    ///
    /// * `target` - The legacy target (host, debugger, or none)
    /// * `topic` - The topic identifying the event or operation
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::types::path::{RuesPath, LegacyTarget};
    /// # use rusk::http::domain::testing;
    ///
    /// // Legacy host path
    /// let topic = testing::create_test_topic("event");
    /// let path = RuesPath::new_legacy(
    ///     LegacyTarget::Host("system".into()),
    ///     topic
    /// );
    ///
    /// // Legacy debugger path
    /// let topic = testing::create_test_topic("log");
    /// let path = RuesPath::new_legacy(
    ///     LegacyTarget::Debugger("trace".into()),
    ///     topic
    /// );
    /// ```
    pub fn new_legacy(legacy_target: LegacyTarget, topic: Topic) -> Self {
        Self {
            target: TargetSpecifier::Legacy(legacy_target),
            topic,
        }
    }

    /// Returns a reference to the target specification of this path.
    ///
    /// The target specification includes both the target type and optional
    /// identifier.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::types::path::{Target, RuesPath, LegacyTarget, TargetSpecifier};
    /// # use rusk::http::domain::testing;
    ///
    /// // Modern target
    /// let topic = testing::create_test_topic("accepted");
    /// let path = RuesPath::new_modern(
    ///     Target::Blocks,
    ///     None,
    ///     topic
    /// );
    /// match path.target() {
    ///     TargetSpecifier::Modern { target, id } => {
    ///         assert_eq!(target, &Target::Blocks);
    ///         assert!(id.is_none());
    ///     }
    ///     _ => panic!("Expected modern target"),
    /// }
    ///
    /// // Legacy target
    /// let topic = testing::create_test_topic("event");
    /// let path = RuesPath::new_legacy(
    ///     LegacyTarget::Host("system".into()),
    ///     topic
    /// );
    /// match path.target() {
    ///     TargetSpecifier::Legacy(target) => {
    ///         assert!(matches!(target, LegacyTarget::Host(_)));
    ///     }
    ///     _ => panic!("Expected legacy target"),
    /// }
    /// ```
    pub fn target(&self) -> &TargetSpecifier {
        &self.target
    }

    /// Returns the topic of this path.
    ///
    /// The topic identifies the specific event or operation within the target
    /// component.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::types::path::{Target, RuesPath};
    /// # use rusk::http::domain::testing;
    ///
    /// // Block acceptance topic
    /// let topic = testing::create_test_topic("accepted");
    /// let path = RuesPath::new_modern(
    ///     Target::Blocks,
    ///     None,
    ///     topic
    /// );
    /// assert_eq!(path.topic(), "accepted");
    ///
    /// // GraphQL query topic
    /// let topic = testing::create_test_topic("query");
    /// let path = RuesPath::new_modern(
    ///     Target::GraphQL,
    ///     None,
    ///     topic
    /// );
    /// assert_eq!(path.topic(), "query");
    /// ```
    ///
    /// # Note
    ///
    /// While topics are unrestricted at the type level, certain targets may
    /// have specific topic requirements that are enforced through
    /// validation rules.
    pub fn topic(&self) -> &str {
        &self.topic.as_str()
    }

    /// Returns whether this path's target matches the given target type.
    ///
    /// Only checks the target type, not the identifier. For paths with
    /// identifiers, use `matches_target_with_id`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::types::path::{Target, RuesPath};
    /// # use rusk::http::domain::testing;
    ///
    /// let topic = testing::create_test_topic("accepted");
    /// let path = RuesPath::new_modern(
    ///     Target::Blocks,
    ///     None,
    ///     topic
    /// );
    /// assert!(path.matches_target(Target::Blocks));
    /// assert!(!path.matches_target(Target::Transactions));
    /// ```
    pub fn matches_target(&self, target: Target) -> bool {
        matches!(self.target, TargetSpecifier::Modern { target: t, .. } if t == target)
    }

    /// Returns whether this path's target and identifier match the given ones.
    ///
    /// Checks both the target type and the specific identifier.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::types::path::{Target, RuesPath};
    /// # use rusk::http::domain::testing;
    ///
    /// // Create path with block hash
    /// let block_id = testing::create_test_block_hash();
    /// let topic = testing::create_test_topic("accepted");
    /// let path = RuesPath::new_modern(
    ///     Target::Blocks,
    ///     Some(block_id.clone()),
    ///     topic
    /// );
    ///
    /// // Should match same hash
    /// assert!(path.matches_target_with_id(Target::Blocks, &block_id));
    ///
    /// // Different hash shouldn't match
    /// let different_id = testing::create_different_block_hash();
    /// assert!(!path.matches_target_with_id(Target::Blocks, &different_id));
    ///
    /// // Different target shouldn't match
    /// assert!(!path.matches_target_with_id(Target::Transactions, &block_id));
    /// ```
    pub fn matches_target_with_id(
        &self,
        target: Target,
        id: &TargetIdentifier,
    ) -> bool {
        matches!(
            self.target,
            TargetSpecifier::Modern {
                target: t,
                id: Some(ref target_id),
            } if t == target && target_id == id
        )
    }

    pub fn matches_topic(&self, topic: &str) -> bool {
        self.topic == Topic::new(topic.into())
    }

    /// Returns whether this is a legacy path.
    ///
    /// Legacy paths use the old target format for backward compatibility.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::types::path::{Target, RuesPath, LegacyTarget};
    /// # use rusk::http::domain::testing;
    ///
    /// // Modern path
    /// let topic = testing::create_test_topic("accepted");
    /// let modern = RuesPath::new_modern(
    ///     Target::Blocks,
    ///     None,
    ///     topic
    /// );
    /// assert!(!modern.is_legacy());
    ///
    /// // Legacy path
    /// let topic = testing::create_test_topic("event");
    /// let legacy = RuesPath::new_legacy(
    ///     LegacyTarget::Host("system".into()),
    ///     topic
    /// );
    /// assert!(legacy.is_legacy());
    /// ```
    pub fn is_legacy(&self) -> bool {
        matches!(self.target, TargetSpecifier::Legacy(_))
    }

    /// Returns whether this is a modern path.
    ///
    /// Modern paths use the new target format as defined in the RUES
    /// specification.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::types::path::{Target, RuesPath, LegacyTarget};
    /// # use rusk::http::domain::testing;
    ///
    /// // Modern path
    /// let topic = testing::create_test_topic("accepted");
    /// let modern = RuesPath::new_modern(
    ///     Target::Blocks,
    ///     None,
    ///     topic
    /// );
    /// assert!(modern.is_modern());
    ///
    /// // Legacy path
    /// let topic = testing::create_test_topic("event");
    /// let legacy = RuesPath::new_legacy(
    ///     LegacyTarget::Host("system".into()),
    ///     topic
    /// );
    /// assert!(!legacy.is_modern());
    /// ```
    pub fn is_modern(&self) -> bool {
        matches!(self.target, TargetSpecifier::Modern { .. })
    }
}

impl fmt::Display for RuesPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "/on/{}/{}", self.target, self.topic.as_str())
    }
}

/// Modern RUES target types.
///
/// Represents the standard target types in the RUES protocol. Each target type
/// may optionally support an identifier (see [`Target::supports_id`]).
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::types::path::Target;
///
/// // Blocks support identifiers
/// assert!(Target::Blocks.supports_id());
///
/// // Node is a singleton target
/// assert!(!Target::Node.supports_id());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    /// Block operations (optional hash)
    Blocks,
    /// Transaction operations (optional hash)
    Transactions,
    /// Contract operations (optional contract ID)
    Contracts,
    /// Node-level operations
    Node,
    /// Network-level operations
    Network,
    /// GraphQL queries
    GraphQL,
    /// Prover operations
    Prover,
}

impl Target {
    /// Returns whether this target type supports identifiers.
    ///
    /// Only certain targets can have identifiers in RUES paths:
    /// - `Blocks`: Can have a block hash
    /// - `Transactions`: Can have a transaction hash
    /// - `Contracts`: Can have a contract ID
    ///
    /// Other targets (`Node`, `Network`, `GraphQL`, `Prover`) do not support
    /// identifiers.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::types::path::Target;
    ///
    /// // Targets that support IDs
    /// assert!(Target::Blocks.supports_id());
    /// assert!(Target::Transactions.supports_id());
    /// assert!(Target::Contracts.supports_id());
    ///
    /// // Targets that don't support IDs
    /// assert!(!Target::Node.supports_id());
    /// assert!(!Target::Network.supports_id());
    /// assert!(!Target::GraphQL.supports_id());
    /// assert!(!Target::Prover.supports_id());
    /// ```
    pub fn supports_id(self) -> bool {
        matches!(self, Self::Blocks | Self::Transactions | Self::Contracts)
    }

    /// Returns a description of this target type.
    ///
    /// Provides a human-readable description of what this target represents
    /// in the RUES protocol.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::types::path::Target;
    ///
    /// assert_eq!(Target::Blocks.description(), "block operations");
    /// assert_eq!(Target::Node.description(), "node operations");
    /// assert_eq!(Target::GraphQL.description(), "GraphQL queries");
    /// ```
    pub fn description(&self) -> &'static str {
        match self {
            Self::Blocks => "block operations",
            Self::Transactions => "transaction operations",
            Self::Contracts => "contract operations",
            Self::Node => "node operations",
            Self::Network => "network operations",
            Self::GraphQL => "GraphQL queries",
            Self::Prover => "prover operations",
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Blocks => write!(f, "blocks"),
            Self::Transactions => write!(f, "transactions"),
            Self::Contracts => write!(f, "contracts"),
            Self::Node => write!(f, "node"),
            Self::Network => write!(f, "network"),
            Self::GraphQL => write!(f, "graphql"),
            Self::Prover => write!(f, "prover"),
        }
    }
}

/// Legacy target types with associated data.
///
/// These targets are maintained for backward compatibility with older systems.
///
/// # Examples
///
/// ```
/// use rusk::http::domain::types::path::LegacyTarget;
///
/// let host = LegacyTarget::Host("system".into());
/// assert_eq!(host.to_string(), "host:system");
///
/// let none = LegacyTarget::None;
/// assert_eq!(none.to_string(), "");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LegacyTarget {
    /// Legacy host system events
    Host(String),
    /// Legacy debugging events
    Debugger(String),
    /// No specific target
    None,
}

impl fmt::Display for LegacyTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Host(data) => write!(f, "host:{}", data),
            Self::Debugger(data) => write!(f, "debugger:{}", data),
            Self::None => write!(f, ""),
        }
    }
}

/// Complete target specification including both modern and legacy cases.
///
/// # Type Safety
///
/// Target identifiers are strongly typed:
/// - Blocks: BlockHash (32 bytes)
/// - Transactions: TransactionHash (32 bytes)
/// - Contracts: ContractId (variable length)
///
/// This ensures that only appropriate identifier types can be used with each
/// target.
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::types::path::{Target, TargetSpecifier};
/// # use rusk::http::domain::testing;
///
/// let block_id = testing::create_test_block_hash();
/// let target = TargetSpecifier::Modern {
///     target: Target::Blocks,
///     id: Some(block_id),
/// };
///
/// assert!(target.is_modern());
/// assert_eq!(target.modern_target(), Some(Target::Blocks));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TargetSpecifier {
    /// Modern target with type-safe identifier
    Modern {
        target: Target,
        id: Option<TargetIdentifier>,
    },
    /// Legacy target
    Legacy(LegacyTarget),
}

impl TargetSpecifier {
    /// Returns true if this is a modern target
    pub fn is_modern(&self) -> bool {
        matches!(self, Self::Modern { .. })
    }

    /// Returns true if this is a legacy target
    pub fn is_legacy(&self) -> bool {
        matches!(self, Self::Legacy(_))
    }

    /// Returns the modern target type if applicable
    pub fn modern_target(&self) -> Option<Target> {
        match self {
            Self::Modern { target, .. } => Some(*target),
            Self::Legacy(_) => None,
        }
    }

    /// Returns the target ID if present
    pub fn id(&self) -> Option<&TargetIdentifier> {
        match self {
            Self::Modern { id, .. } => id.as_ref(),
            Self::Legacy(_) => None,
        }
    }
}

impl fmt::Display for TargetSpecifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Modern { target, id } => {
                if let Some(id) = id {
                    write!(f, "{}:{}", target, id)
                } else {
                    write!(f, "{}", target)
                }
            }
            Self::Legacy(legacy) => write!(f, "{}", legacy),
        }
    }
}

/// A topic in RUES protocol path.
///
/// Topics are used to identify specific events or operations within a target
/// component. For example:
/// - `/on/blocks/accepted` - "accepted" is the topic
/// - `/on/contracts:id/deploy` - "deploy" is the topic
///
/// Topics can contain alphanumeric characters, dots, dashes, and underscores.
/// Validation of specific characters and formats is handled by validation
/// rules.
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::types::path::Target;
/// # use rusk::http::domain::testing;
///
/// // Create a path with topic for block acceptance
/// let path = testing::create_test_rues_path(
///     Target::Blocks,
///     None,
///     "accepted"
/// );
/// assert_eq!(path.topic(), "accepted");
///
/// // Create a path with topic for contract deployment
/// let path = testing::create_test_rues_path(
///     Target::Contracts,
///     None,
///     "deploy"
/// );
/// assert_eq!(path.topic(), "deploy");
/// ```
///
/// # RUES Protocol
///
/// According to RUES specification, topics are unrestricted strings that
/// identify events or operations. However, certain targets may have specific
/// topic requirements:
/// - GraphQL only accepts "query" topic
/// - Node operations have a predefined set of topics
/// These restrictions are enforced by validation rules, not at the type level.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Topic(pub(crate) String);

impl Topic {
    /// Creates a new topic. Only available within the crate.
    ///
    /// # Arguments
    ///
    /// * `value` - The topic string
    ///
    /// This constructor is crate-visible and used by the factory to ensure
    /// proper topic creation through the processing pipeline.
    pub(crate) fn new(value: String) -> Self {
        Self(value)
    }

    /// Returns a string slice of the topic.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rusk::http::domain::testing;
    ///
    /// let topic = testing::create_test_topic("accepted");
    /// assert_eq!(topic.as_str(), "accepted");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::domain::testing;
    use crate::http::domain::types::identifier::IdentifierBytes;
    use crate::http::domain::types::value::RuesValue;
    use bytes::Bytes;

    // Helper functions for creating test identifiers
    fn create_test_block_hash() -> TargetIdentifier {
        TargetIdentifier::Block(BlockHash(IdentifierBytes(RuesValue::Binary(
            Bytes::copy_from_slice(&[1u8; 32]),
        ))))
    }

    fn create_different_block_hash() -> TargetIdentifier {
        TargetIdentifier::Block(BlockHash(IdentifierBytes(RuesValue::Binary(
            Bytes::copy_from_slice(&[2u8; 32]),
        ))))
    }

    fn create_test_tx_hash() -> TargetIdentifier {
        TargetIdentifier::Transaction(TransactionHash(IdentifierBytes(
            RuesValue::Binary(Bytes::copy_from_slice(&[3u8; 32])),
        )))
    }

    fn create_different_tx_hash() -> TargetIdentifier {
        TargetIdentifier::Transaction(TransactionHash(IdentifierBytes(
            RuesValue::Binary(Bytes::copy_from_slice(&[4u8; 32])),
        )))
    }

    fn create_test_contract_id() -> TargetIdentifier {
        TargetIdentifier::Contract(ContractId(IdentifierBytes(
            RuesValue::Binary(Bytes::from(vec![5u8; 12])),
        )))
    }

    fn create_different_contract_id() -> TargetIdentifier {
        TargetIdentifier::Contract(ContractId(IdentifierBytes(
            RuesValue::Binary(Bytes::from(vec![6u8; 12])),
        )))
    }

    #[test]
    fn test_target_display() {
        assert_eq!(Target::Blocks.to_string(), "blocks");
        assert_eq!(Target::Transactions.to_string(), "transactions");
        assert_eq!(Target::Contracts.to_string(), "contracts");
        assert_eq!(Target::Node.to_string(), "node");
        assert_eq!(Target::Network.to_string(), "network");
        assert_eq!(Target::GraphQL.to_string(), "graphql");
        assert_eq!(Target::Prover.to_string(), "prover");
    }

    #[test]
    fn test_legacy_target_display() {
        assert_eq!(
            LegacyTarget::Host("system".into()).to_string(),
            "host:system"
        );
        assert_eq!(
            LegacyTarget::Debugger("trace".into()).to_string(),
            "debugger:trace"
        );
        assert_eq!(LegacyTarget::None.to_string(), "");
    }

    #[test]
    fn test_target_specifier_display() {
        // Modern without ID
        let target = TargetSpecifier::Modern {
            target: Target::Blocks,
            id: None,
        };
        assert_eq!(target.to_string(), "blocks");

        // Modern with ID
        let target = TargetSpecifier::Modern {
            target: Target::Contracts,
            id: Some(create_test_contract_id()),
        };
        assert!(target.to_string().starts_with("contracts:"));

        // Legacy
        let target = TargetSpecifier::Legacy(LegacyTarget::Host("test".into()));
        assert_eq!(target.to_string(), "host:test");
    }

    #[test]
    fn test_rues_path_display() {
        // Modern path without ID
        let path = RuesPath::new_modern(
            Target::Blocks,
            None,
            testing::create_test_topic("accepted"),
        );
        assert_eq!(path.to_string(), "/on/blocks/accepted");

        // Modern path with ID
        let path = RuesPath::new_modern(
            Target::Contracts,
            Some(create_test_contract_id()),
            testing::create_test_topic("deploy"),
        );
        assert!(path.to_string().starts_with("/on/contracts:"));
        assert!(path.to_string().ends_with("/deploy"));

        // Legacy path
        let path = RuesPath::new_legacy(
            LegacyTarget::Host("system".into()),
            testing::create_test_topic("event"),
        );
        assert_eq!(path.to_string(), "/on/host:system/event");
    }

    #[test]
    fn test_target_supports_id() {
        // Should support IDs
        assert!(Target::Blocks.supports_id());
        assert!(Target::Transactions.supports_id());
        assert!(Target::Contracts.supports_id());

        // Should not support IDs
        assert!(!Target::Node.supports_id());
        assert!(!Target::Network.supports_id());
        assert!(!Target::GraphQL.supports_id());
        assert!(!Target::Prover.supports_id());
    }

    #[test]
    fn test_path_matching() {
        let contract_id = create_test_contract_id();
        let path = RuesPath::new_modern(
            Target::Contracts,
            Some(contract_id.clone()),
            testing::create_test_topic("deploy"),
        );

        // Target matching
        assert!(path.matches_target(Target::Contracts));
        assert!(!path.matches_target(Target::Blocks));

        // Target with ID matching - same ID should match
        assert!(path.matches_target_with_id(Target::Contracts, &contract_id));

        // Different ID shouldn't match
        let different_id = create_different_contract_id();
        assert!(!path.matches_target_with_id(Target::Contracts, &different_id));

        // Wrong target type shouldn't match
        let block_id = create_test_block_hash();
        assert!(!path.matches_target_with_id(Target::Blocks, &block_id));

        // Topic matching
        assert!(path.matches_topic("deploy"));
        assert!(!path.matches_topic("undeploy"));
    }

    #[test]
    fn test_identifier_comparison() {
        // Test block hash comparison
        let block_id1 = create_test_block_hash();
        let block_id2 = create_test_block_hash();
        let different_block_id = create_different_block_hash();

        assert_eq!(block_id1, block_id2);
        assert_ne!(block_id1, different_block_id);

        // Test transaction hash comparison
        let tx_id1 = create_test_tx_hash();
        let tx_id2 = create_test_tx_hash();
        let different_tx_id = create_different_tx_hash();

        assert_eq!(tx_id1, tx_id2);
        assert_ne!(tx_id1, different_tx_id);

        // Test contract id comparison
        let contract_id1 = create_test_contract_id();
        let contract_id2 = create_test_contract_id();
        let different_contract_id = create_different_contract_id();

        assert_eq!(contract_id1, contract_id2);
        assert_ne!(contract_id1, different_contract_id);
    }

    #[test]
    fn test_path_type_checking() {
        // Modern path
        let modern_path = RuesPath::new_modern(
            Target::Blocks,
            None,
            testing::create_test_topic("accepted"),
        );
        assert!(modern_path.is_modern());
        assert!(!modern_path.is_legacy());

        // Legacy path
        let legacy_path = RuesPath::new_legacy(
            LegacyTarget::Debugger("trace".into()),
            testing::create_test_topic("event"),
        );
        assert!(legacy_path.is_legacy());
        assert!(!legacy_path.is_modern());
    }

    #[test]
    fn test_target_descriptions() {
        assert_eq!(Target::Blocks.description(), "block operations");
        assert_eq!(
            Target::Transactions.description(),
            "transaction operations"
        );
        assert_eq!(Target::Contracts.description(), "contract operations");
        assert_eq!(Target::Node.description(), "node operations");
        assert_eq!(Target::Network.description(), "network operations");
        assert_eq!(Target::GraphQL.description(), "GraphQL queries");
        assert_eq!(Target::Prover.description(), "prover operations");
    }

    #[test]
    fn test_target_specifier_getters() {
        let contract_id = create_test_contract_id();
        let target = TargetSpecifier::Modern {
            target: Target::Contracts,
            id: Some(contract_id.clone()),
        };

        assert!(target.is_modern());
        assert!(!target.is_legacy());
        assert_eq!(target.modern_target(), Some(Target::Contracts));
        assert_eq!(target.id(), Some(&contract_id));

        let legacy = TargetSpecifier::Legacy(LegacyTarget::Host("test".into()));
        assert!(legacy.is_legacy());
        assert!(!legacy.is_modern());
        assert_eq!(legacy.modern_target(), None);
        assert_eq!(legacy.id(), None);
    }

    #[test]
    fn test_path_construction() {
        // Test modern path construction with contract ID
        let contract_id = create_test_contract_id();
        let path = RuesPath::new_modern(
            Target::Contracts,
            Some(contract_id.clone()),
            testing::create_test_topic("deploy"),
        );
        assert!(path.is_modern());
        assert_eq!(path.topic(), "deploy");

        if let TargetSpecifier::Modern { target, id } = path.target() {
            assert_eq!(*target, Target::Contracts);
            assert_eq!(id.as_ref(), Some(&contract_id));
        } else {
            panic!("Expected modern target");
        }

        // Test legacy path construction
        let path = RuesPath::new_legacy(
            LegacyTarget::Host("system".into()),
            testing::create_test_topic("event"),
        );
        assert!(path.is_legacy());
        assert_eq!(path.topic(), "event");

        if let TargetSpecifier::Legacy(LegacyTarget::Host(host)) = path.target()
        {
            assert_eq!(host, "system");
        } else {
            panic!("Expected legacy host target");
        }
    }

    #[test]
    fn test_identifier_type_safety() {
        // Block hash should only work with Block target
        let block_id = create_test_block_hash();
        let path = RuesPath::new_modern(
            Target::Blocks,
            Some(block_id.clone()),
            testing::create_test_topic("accepted"),
        );
        assert!(path.matches_target_with_id(Target::Blocks, &block_id));

        // Transaction hash should only work with Transaction target
        let tx_id = create_test_tx_hash();
        let path = RuesPath::new_modern(
            Target::Transactions,
            Some(tx_id.clone()),
            testing::create_test_topic("executed"),
        );
        assert!(path.matches_target_with_id(Target::Transactions, &tx_id));

        // Contract ID should only work with Contract target
        let contract_id = create_test_contract_id();
        let path = RuesPath::new_modern(
            Target::Contracts,
            Some(contract_id.clone()),
            testing::create_test_topic("deploy"),
        );
        assert!(path.matches_target_with_id(Target::Contracts, &contract_id));
    }

    #[test]
    fn test_topic_creation() {
        // Positive cases
        let topic = testing::create_test_topic("accepted");
        assert_eq!(topic.as_str(), "accepted");

        // Different string types
        let string_topic = testing::create_test_topic(String::from("deployed"));
        assert_eq!(string_topic.as_str(), "deployed");

        let static_str_topic = testing::create_test_topic("static");
        assert_eq!(static_str_topic.as_str(), "static");

        // Edge cases
        // Single character
        let single_char = testing::create_test_topic("x");
        assert_eq!(single_char.as_str(), "x");

        // Long string
        let long_topic =
            testing::create_test_topic("very_long_topic_name_with_underscores");
        assert_eq!(
            long_topic.as_str(),
            "very_long_topic_name_with_underscores"
        );

        // Unicode
        let unicode_topic = testing::create_test_topic("hòla");
        assert_eq!(unicode_topic.as_str(), "hòla");

        // Special characters (valid)
        let special_chars =
            testing::create_test_topic("topic-with.special_chars@123");
        assert_eq!(special_chars.as_str(), "topic-with.special_chars@123");
    }

    #[test]
    fn test_topic_equality() {
        let topic1 = testing::create_test_topic("test");
        let topic2 = testing::create_test_topic("test");
        let topic3 = testing::create_test_topic("different");

        assert_eq!(topic1, topic2);
        assert_ne!(topic1, topic3);
        assert_ne!(topic2, topic3);
    }

    #[test]
    fn test_topic_clone() {
        let original = testing::create_test_topic("original");
        let cloned = original.clone();

        assert_eq!(original, cloned);
        assert_eq!(original.as_str(), cloned.as_str());
    }

    #[test]
    fn test_topic_hash() {
        use std::collections::HashSet;

        let mut topics = HashSet::new();

        // Insert some topics
        topics.insert(testing::create_test_topic("one"));
        topics.insert(testing::create_test_topic("two"));
        topics.insert(testing::create_test_topic("one")); // Duplicate

        // Should only contain unique topics
        assert_eq!(topics.len(), 2);
        assert!(topics.contains(&testing::create_test_topic("one")));
        assert!(topics.contains(&testing::create_test_topic("two")));
    }

    #[test]
    fn test_rues_path_topic() {
        // Test topic access through RuesPath
        let path =
            testing::create_test_rues_path(Target::Blocks, None, "accepted");
        assert_eq!(path.topic(), "accepted");

        // Test with different topic types
        let path_with_dots = testing::create_test_rues_path(
            Target::Contracts,
            None,
            "deploy.success",
        );
        assert_eq!(path_with_dots.topic(), "deploy.success");

        // Test with unicode topic
        let path_unicode =
            testing::create_test_rues_path(Target::Node, None, "тема");
        assert_eq!(path_unicode.topic(), "тема");
    }

    #[test]
    fn test_topic_debug_format() {
        let topic = testing::create_test_topic("test");
        assert_eq!(format!("{:?}", topic), r#"Topic("test")"#);
    }
}
