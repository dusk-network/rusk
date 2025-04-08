// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Defines core types used throughout the WebSocket subscription system.
//!
//! This module includes identifiers for subscriptions and sessions, the topics
//! clients can subscribe to, and parameters associated with specific
//! subscription types. It serves as the foundation for managing client
//! subscriptions and routing relevant events.
//!
//! # Core Concepts
//!
//! - [`Topic`]: Represents the category of events a client subscribes to (e.g.,
//!   `BlockAcceptance`, `ContractEvents`). Each topic corresponds to a specific
//!   JSON-RPC subscription method and dictates the type of data sent in
//!   notifications.
//! - [`SubscriptionId`]: A unique identifier ([`Uuid`]) assigned by the server
//!   to each active subscription. Clients use this ID to manage (e.g.,
//!   unsubscribe) their specific subscriptions.
//! - [`SessionId`]: An identifier ([`Uuid`]) representing a client's unique
//!   WebSocket connection session. It's used internally to manage all
//!   subscriptions tied to a specific connection, facilitating cleanup when a
//!   client disconnects.
//! - **Subscription Parameters** ([`BlockSubscriptionParams`],
//!   [`ContractSubscriptionParams`], [`MempoolSubscriptionParams`]): Structs
//!   used to capture optional parameters provided by clients during the
//!   subscription request. These parameters allow for fine-grained control over
//!   the data received (e.g., filtering contract events by name, including
//!   transaction details in block notifications).
//!
//! # Usage
//!
//! These types are primarily used internally by the `SubscriptionManager` and
//! the JSON-RPC server implementation.
//!
//! - When a client calls a subscription method (e.g.,
//!   `subscribeContractEvents`), the server deserializes the request parameters
//!   into the corresponding struct (e.g., `ContractSubscriptionParams`).
//! - The `SubscriptionManager` uses `Topic`, `SubscriptionId`, and `SessionId`
//!   to register, track, and manage subscriptions and associated client
//!   connections (sinks).
//! - Parameter structs are used by filters (if implemented) to determine which
//!   events match a specific subscription's criteria.
//!
//! ```
//! use std::str::FromStr;
//! use rusk::jsonrpc::infrastructure::subscription::types::{Topic, SubscriptionId, SessionId, ContractSubscriptionParams};
//!
//! // Example: Representing a subscription request
//! let topic = Topic::ContractEvents;
//! let params: ContractSubscriptionParams = serde_json::from_value(serde_json::json!({ // Typically from JSON RPC
//!     "contractId": "0x123abc",
//!     "eventNames": ["Transfer"],
//!     "includeMetadata": true
//! })).unwrap();
//! // SessionId should be created from an existing identifier string
//! let session_id = SessionId::from_str("ws-conn-12345").expect("Valid session ID"); // ID for the client's connection
//! let subscription_id = SubscriptionId::new(); // ID generated for this specific subscription
//!
//! println!("New subscription ({}) for session {}: Topic=\"{}\", Params={{ contract: {}, events: {:?}, meta: {:?} }}",
//!     subscription_id, session_id, topic, params.contract_id(), params.event_names(), params.include_metadata());
//! ```

use crate::jsonrpc::infrastructure::subscription::error::SubscriptionError;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt::{self, Display};
use std::str::FromStr;
use uuid::Uuid;

/// Represents the different topics clients can subscribe to via WebSocket.
///
/// Each variant corresponds to a specific type of blockchain event or state
/// change that the client wants to receive real-time notifications about. The
/// topic determines the structure and content of the event data sent to the
/// subscriber.
///
/// # Examples
///
/// Serializing a topic:
///
/// ```
/// # use serde_json;
/// # use rusk::jsonrpc::infrastructure::subscription::types::Topic;
/// let topic = Topic::BlockAcceptance;
/// let serialized = serde_json::to_string(&topic).unwrap();
/// assert_eq!(serialized, "\"BlockAcceptance\"");
/// ```
///
/// Deserializing a topic:
///
/// ```
/// # use serde_json;
/// # use rusk::jsonrpc::infrastructure::subscription::types::Topic;
/// let data = "\"BlockFinalization\"";
/// let deserialized: Topic = serde_json::from_str(data).unwrap();
/// assert_eq!(deserialized, Topic::BlockFinalization);
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Topic {
    /// Subscription topic for notifications when a new block is accepted by
    /// the network node. Corresponds to the `subscribeBlockAcceptance`
    /// WebSocket method.
    ///
    /// Subscribers receive details about the accepted block, potentially
    /// including transaction data if requested via parameters.
    BlockAcceptance,

    /// Subscription topic for notifications when a block becomes finalized
    /// (irreversible). Corresponds to the `subscribeBlockFinalization`
    /// WebSocket method.
    ///
    /// Subscribers receive information about the finalized block, confirming
    /// its permanence in the blockchain.
    BlockFinalization,

    /// Subscription topic for notifications about chain reorganizations
    /// (reorgs). Corresponds to the `subscribeChainReorganization`
    /// WebSocket method.
    ///
    /// Subscribers are notified when the canonical chain changes, receiving
    /// information about the depth of the reorg and the blocks involved.
    ChainReorganization,

    /// Subscription topic for events emitted by specific smart contracts.
    /// Corresponds to the `subscribeContractEvents` WebSocket method.
    ///
    /// Subscribers receive notifications for events matching the specified
    /// contract ID and optional event names provided during subscription.
    ContractEvents,

    /// Subscription topic specifically for transfer events emitted by smart
    /// contracts. Corresponds to the `subscribeContractTransferEvents`
    /// WebSocket method.
    ///
    /// This is a specialized version of `ContractEvents`, focusing only on
    /// transfer-like events, potentially with additional filtering options
    /// like minimum amount.
    ContractTransferEvents,

    /// Subscription topic for notifications when a transaction is accepted
    /// into the mempool. Corresponds to the `subscribeMempoolAcceptance`
    /// WebSocket method.
    ///
    /// Subscribers receive information about transactions entering the
    /// mempool, potentially including full transaction details if
    /// requested.
    MempoolAcceptance,

    /// Subscription topic for general mempool events, such as transaction
    /// removal or replacement. Corresponds to the `subscribeMempoolEvents`
    /// WebSocket method.
    ///
    /// Subscribers receive notifications about various changes occurring
    /// within the mempool beyond simple acceptance.
    MempoolEvents,
}

impl Topic {
    /// Returns the string representation of the topic.
    ///
    /// This method is used for serialization and logging purposes.
    ///
    /// # Examples
    ///
    /// ```
    /// use rusk::jsonrpc::infrastructure::subscription::types::Topic;
    ///
    /// let topic = Topic::BlockAcceptance;
    /// assert_eq!(topic.as_str(), "BlockAcceptance");
    ///
    /// let topic = Topic::BlockFinalization;
    /// assert_eq!(topic.as_str(), "BlockFinalization");
    ///
    /// let topic = Topic::ChainReorganization;
    /// assert_eq!(topic.as_str(), "ChainReorganization");
    ///
    /// let topic = Topic::ContractEvents;
    /// assert_eq!(topic.as_str(), "ContractEvents");
    ///
    /// let topic = Topic::ContractTransferEvents;
    /// assert_eq!(topic.as_str(), "ContractTransferEvents");
    ///
    /// let topic = Topic::MempoolAcceptance;
    /// assert_eq!(topic.as_str(), "MempoolAcceptance");
    ///
    /// let topic = Topic::MempoolEvents;
    /// assert_eq!(topic.as_str(), "MempoolEvents");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Topic::BlockAcceptance => "BlockAcceptance",
            Topic::BlockFinalization => "BlockFinalization",
            Topic::ChainReorganization => "ChainReorganization",
            Topic::ContractEvents => "ContractEvents",
            Topic::ContractTransferEvents => "ContractTransferEvents",
            Topic::MempoolAcceptance => "MempoolAcceptance",
            Topic::MempoolEvents => "MempoolEvents",
        }
    }
}

impl Display for Topic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Topic {
    type Err = SubscriptionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "BlockAcceptance" => Ok(Topic::BlockAcceptance),
            "BlockFinalization" => Ok(Topic::BlockFinalization),
            "ChainReorganization" => Ok(Topic::ChainReorganization),
            "ContractEvents" => Ok(Topic::ContractEvents),
            "ContractTransferEvents" => Ok(Topic::ContractTransferEvents),
            "MempoolAcceptance" => Ok(Topic::MempoolAcceptance),
            "MempoolEvents" => Ok(Topic::MempoolEvents),
            _ => Err(SubscriptionError::InvalidTopic(s.to_string())),
        }
    }
}

/// A unique identifier for a WebSocket subscription.
///
/// This ID is generated by the server upon a successful subscription request
/// and is used by the client to manage the subscription (e.g., unsubscribe).
/// It wraps a UUID (`v4`) to ensure uniqueness across all active subscriptions.
///
/// # Examples
///
/// Creating a new `SubscriptionId`:
///
/// ```
/// use rusk::jsonrpc::infrastructure::subscription::types::SubscriptionId;
///
/// let sub_id = SubscriptionId::new();
/// println!("New Subscription ID: {}", sub_id);
/// ```
///
/// Converting to and from a string:
///
/// ```
/// use std::str::FromStr;
/// use rusk::jsonrpc::infrastructure::subscription::types::SubscriptionId;
///
/// let sub_id = SubscriptionId::new();
/// let id_str = sub_id.to_string();
/// let parsed_id = SubscriptionId::from_str(&id_str).unwrap();
/// assert_eq!(sub_id, parsed_id);
///
/// // Example with a known UUID string
/// let known_uuid_str = "f47ac10b-58cc-4372-a567-0e02b2c3d479";
/// let parsed_from_known = SubscriptionId::from_str(known_uuid_str).unwrap();
/// assert_eq!(parsed_from_known.to_string(), known_uuid_str);
/// ```
///
/// JSON Serialization/Deserialization:
///
/// ```
/// use serde_json;
/// use rusk::jsonrpc::infrastructure::subscription::types::SubscriptionId;
///
/// let sub_id = SubscriptionId::new();
/// let json = serde_json::to_string(&sub_id).unwrap();
/// println!("Serialized JSON: {}", json); // e.g., "\"f47ac10b-58cc-4372-a567-0e02b2c3d479\""
///
/// let deserialized: SubscriptionId = serde_json::from_str(&json).unwrap();
/// assert_eq!(sub_id, deserialized);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)] // Serialize/Deserialize as the inner Uuid string
pub struct SubscriptionId(Uuid);

impl SubscriptionId {
    /// Creates a new, unique `SubscriptionId` using a v4 UUID.
    ///
    /// This method generates a new UUID using the `v4` variant of the UUID
    /// algorithm, ensuring the generated ID is universally unique.
    ///
    /// # Examples
    ///
    /// ```
    /// use rusk::jsonrpc::infrastructure::subscription::types::SubscriptionId;
    ///
    /// let sub_id = SubscriptionId::new();
    /// println!("New Subscription ID: {}", sub_id);
    /// ```
    pub fn new() -> Self {
        SubscriptionId(Uuid::new_v4())
    }

    /// Returns the underlying [`Uuid`].
    ///
    /// This method provides direct access to the underlying UUID value,
    /// allowing for inspection or manipulation of the subscription ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use rusk::jsonrpc::infrastructure::subscription::types::SubscriptionId;
    ///
    /// let sub_id = SubscriptionId::new();
    /// let inner_uuid = sub_id.inner();
    /// assert_eq!(sub_id.to_string(), inner_uuid.to_string());
    /// ```
    pub fn inner(&self) -> Uuid {
        self.0
    }
}

impl Default for SubscriptionId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for SubscriptionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use the hyphenated string representation of the UUID
        write!(f, "{}", self.0)
    }
}

impl FromStr for SubscriptionId {
    type Err = SubscriptionError;

    /// Parses a string slice into a `SubscriptionId`.
    ///
    /// The string must be a valid UUID representation.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(SubscriptionId).map_err(|e| {
            SubscriptionError::InvalidSubscriptionIdFormat(e.to_string())
        })
    }
}

/// Represents a unique identifier for a client's WebSocket connection session.
///
/// Each active WebSocket connection needs a unique identifier to associate
/// multiple subscriptions with the same client. This `SessionId` newtype wraps
/// a [`String`] representation of that identifier (e.g., a string derived from
/// jsonrpsee's `ConnectionId`, or potentially another source like a UUID).
///
/// It is used internally by the `SubscriptionManager` to group all
/// subscriptions belonging to a single client connection. This facilitates
/// efficient cleanup (unsubscribing all related subscriptions) when the
/// underlying WebSocket connection is closed.
///
/// The primary validation performed is ensuring the ID is not empty.
///
/// # Examples
///
/// Creating from a string (validates non-empty):
///
/// ```
/// use rusk::jsonrpc::infrastructure::subscription::types::SessionId;
/// use std::str::FromStr;
///
/// let session_str = "conn-123-xyz";
/// let session_id = SessionId::from_str(session_str).unwrap();
/// assert_eq!(session_id.to_string(), session_str);
///
/// // Empty string is invalid
/// assert!(SessionId::from_str("").is_err());
/// ```
///
/// Using in a collection:
///
/// ```
/// use std::collections::HashSet;
/// use rusk::jsonrpc::infrastructure::subscription::types::SessionId;
/// use std::str::FromStr;
///
/// let mut active_sessions = HashSet::new();
/// let session1 = SessionId::from_str("sess-alpha").unwrap();
/// let session2 = SessionId::from_str("sess-beta").unwrap();
/// active_sessions.insert(session1.clone());
/// active_sessions.insert(session2);
///
/// assert!(active_sessions.contains(&session1));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)] // Remove Deserialize here
#[serde(transparent)]
pub struct SessionId(String);

// Implement Deserialize separately using try_from for validation
impl<'de> Deserialize<'de> for SessionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        SessionId::try_from(s).map_err(serde::de::Error::custom)
    }
}

impl SessionId {
    /// Returns a reference to the underlying session ID string.
    pub fn inner(&self) -> &str {
        &self.0
    }
}

// Implement TryFrom<String> for validation
impl TryFrom<String> for SessionId {
    type Error = SubscriptionError;

    /// Attempts to create a `SessionId` from a [`String`], validating that it
    /// is not empty.
    ///
    /// # Errors
    ///
    /// Returns [`SubscriptionError::InvalidSessionIdFormat`] if the provided
    /// `value` is empty.
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(SubscriptionError::InvalidSessionIdFormat(
                "Session ID cannot be empty".to_string(),
            ))
        } else {
            Ok(SessionId(value))
        }
    }
}

impl Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for SessionId {
    type Err = SubscriptionError;

    /// Parses a string slice into a `SessionId`.
    ///
    /// Performs basic validation (non-empty) via `TryFrom`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rusk::jsonrpc::infrastructure::subscription::types::SessionId;
    /// use std::str::FromStr;
    ///
    /// let session_str = "session-abc-123";
    /// let session_id = SessionId::from_str(session_str).unwrap();
    /// assert_eq!(session_id.to_string(), session_str);
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        SessionId::try_from(s.to_string())
    }
}

/// Parameters for subscribing to block-related events (`BlockAcceptance`,
/// `BlockFinalization`).
///
/// This struct allows clients to specify options when subscribing to block
/// notifications, such as whether to include full transaction data in the
/// notification payload.
///
/// It's typically created via deserialization from JSON RPC parameters but can
/// also be constructed manually using [`BlockSubscriptionParams::builder`].
///
/// # Examples
///
/// Building parameters manually:
///
/// ```
/// use rusk::jsonrpc::infrastructure::subscription::types::BlockSubscriptionParams;
///
/// // Build with default (include_txs = None)
/// let params_default = BlockSubscriptionParams::builder().build();
/// assert_eq!(params_default.include_txs(), None);
///
/// // Build requesting transactions
/// let params_with_txs = BlockSubscriptionParams::builder()
///     .include_txs(true)
///     .build();
/// assert_eq!(params_with_txs.include_txs(), Some(true));
///
/// // Build explicitly *not* requesting transactions
/// let params_no_txs = BlockSubscriptionParams::builder()
///     .include_txs(false)
///     .build();
/// assert_eq!(params_no_txs.include_txs(), Some(false));
/// ```
///
/// JSON Serialization/Deserialization:
///
/// ```
/// use serde_json;
/// use rusk::jsonrpc::infrastructure::subscription::types::BlockSubscriptionParams;
///
/// // Serialize with include_txs = true
/// let params = BlockSubscriptionParams::builder().include_txs(true).build();
/// let json = serde_json::to_string(&params).unwrap();
/// // Expect camelCase
/// assert_eq!(json, r#"{"includeTxs":true}"#);
///
/// // Deserialize with include_txs = true
/// let deserialized: BlockSubscriptionParams = serde_json::from_str(&json).unwrap();
/// assert_eq!(deserialized.include_txs(), Some(true));
///
/// // Serialize with include_txs = false
/// let params_no = BlockSubscriptionParams::builder().include_txs(false).build();
/// let json_no = serde_json::to_string(&params_no).unwrap();
/// // Expect camelCase
/// assert_eq!(json_no, r#"{"includeTxs":false}"#);
///
/// // Deserialize with include_txs = false
/// let deserialized_no: BlockSubscriptionParams = serde_json::from_str(&json_no).unwrap();
/// assert_eq!(deserialized_no.include_txs(), Some(false));
///
/// // Serialize with include_txs = None (default)
/// let params_none = BlockSubscriptionParams::builder().build();
/// let json_none = serde_json::to_string(&params_none).unwrap();
/// // Note: Serde skips serializing `None` for Option<bool> by default
/// assert_eq!(json_none, r#"{}"#);
///
/// // Deserialize from JSON where include_txs is null or missing
/// // Use camelCase for deserialization test too
/// let json_null = r#"{"includeTxs":null}"#;
/// let deserialized_null: BlockSubscriptionParams = serde_json::from_str(json_null).unwrap();
/// assert_eq!(deserialized_null.include_txs(), None);
///
/// let json_missing = r#"{}"#;
/// let deserialized_missing: BlockSubscriptionParams = serde_json::from_str(json_missing).unwrap();
/// assert_eq!(deserialized_missing.include_txs(), None);
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct BlockSubscriptionParams {
    /// Optional flag indicating whether to include full transaction data in
    /// block event notifications.
    ///
    /// - `Some(true)`: Include full transactions.
    /// - `Some(false)`: Do not include transactions.
    /// - `None`: Use the server's default behavior (which might be to include
    ///   or exclude transactions).
    #[serde(skip_serializing_if = "Option::is_none")]
    include_txs: Option<bool>,
}

impl BlockSubscriptionParams {
    /// Creates a new builder for `BlockSubscriptionParams`.
    ///
    /// This is the preferred way to construct the parameters manually.
    pub fn builder() -> BlockSubscriptionParamsBuilder {
        BlockSubscriptionParamsBuilder::default()
    }

    /// Returns the value of the `include_txs` flag.
    pub fn include_txs(&self) -> Option<bool> {
        self.include_txs
    }

    // Private constructor primarily for the builder and potentially serde.
    fn new(include_txs: Option<bool>) -> Self {
        Self { include_txs }
    }
}

/// Builder for [`BlockSubscriptionParams`].
#[derive(Debug, Default)]
pub struct BlockSubscriptionParamsBuilder {
    include_txs: Option<bool>,
}

impl BlockSubscriptionParamsBuilder {
    /// Sets whether to include full transaction data.
    pub fn include_txs(mut self, include: bool) -> Self {
        self.include_txs = Some(include);
        self
    }

    /// Builds the final [`BlockSubscriptionParams`].
    pub fn build(self) -> BlockSubscriptionParams {
        BlockSubscriptionParams::new(self.include_txs)
    }
}

/// Parameters for subscribing to contract-related events (`ContractEvents`,
/// `ContractTransferEvents`).
///
/// This struct allows clients to specify filtering criteria when subscribing to
/// contract events, such as the target contract address (mandatory), specific
/// event names, whether to include metadata, and for transfer events, a minimum
/// amount.
///
/// Use [`ContractSubscriptionParams::builder`] to construct instances manually,
/// leveraging the type-state pattern to ensure during the compile time the
/// mandatory `contract_id` is provided.
///
/// # Examples
///
/// Building parameters manually using the type-state builder:
///
/// ```
/// use rusk::jsonrpc::infrastructure::subscription::types::ContractSubscriptionParams;
///
/// let contract_id = "0x123abc".to_string();
///
/// // Minimal build (only required field)
/// let params_minimal = ContractSubscriptionParams::builder()
///     .contract_id(contract_id.clone())
///     .build();
/// assert_eq!(params_minimal.contract_id(), &contract_id);
/// assert_eq!(params_minimal.event_names(), None);
/// assert_eq!(params_minimal.include_metadata(), None);
/// assert_eq!(params_minimal.min_amount(), None);
///
/// // Build with all optional fields set
/// let event_names = vec!["Transfer".to_string(), "Approval".to_string()];
/// let min_amount = "1000000".to_string(); // Represents 1 unit
///
/// let params_full = ContractSubscriptionParams::builder()
///     .contract_id(contract_id.clone()) // Required field first
///     .event_names(event_names.clone())
///     .include_metadata(true)
///     .min_amount(min_amount.clone())
///     .build();
///
/// assert_eq!(params_full.contract_id(), &contract_id);
/// assert_eq!(params_full.event_names(), Some(&event_names));
/// assert_eq!(params_full.include_metadata(), Some(true));
/// assert_eq!(params_full.min_amount(), Some(&min_amount));
///
/// // Compile-time error if contract_id is missing:
/// // let params_error = ContractSubscriptionParams::builder().build(); // Compile-time error! build() not found
/// // let params_error = ContractSubscriptionParams::builder().event_names(vec![]).build(); Compile-time error!
/// ```
///
/// JSON Serialization/Deserialization:
///
/// ```
/// use serde_json;
/// use rusk::jsonrpc::infrastructure::subscription::types::ContractSubscriptionParams;
///
/// let params = ContractSubscriptionParams::builder()
///     .contract_id("0x789ghi".to_string())
///     .event_names(vec!["Mint".to_string()])
///     .include_metadata(false) // Explicitly false
///     .min_amount("500".to_string())
///     .build();
///
/// let json = serde_json::to_string(&params).unwrap();
/// // Note: include_metadata is present because it's Some(false).
/// // Other None fields are skipped.
/// // Expect camelCase fields
/// assert_eq!(
///     json,
///     r#"{"contractId":"0x789ghi","eventNames":["Mint"],"includeMetadata":false,"minAmount":"500"}"#
/// );
///
/// let deserialized: ContractSubscriptionParams = serde_json::from_str(&json).unwrap();
/// assert_eq!(deserialized.contract_id(), "0x789ghi");
/// assert_eq!(deserialized.event_names(), Some(&vec!["Mint".to_string()]));
/// assert_eq!(deserialized.include_metadata(), Some(false));
/// assert_eq!(deserialized.min_amount(), Some(&"500".to_string()));
///
/// // Example deserializing with only the mandatory field (using camelCase)
/// let json_minimal = r#"{"contractId":"0xabc123"}"#;
/// let deserialized_minimal: ContractSubscriptionParams = serde_json::from_str(json_minimal).unwrap();
/// assert_eq!(deserialized_minimal.contract_id(), "0xabc123");
/// assert_eq!(deserialized_minimal.event_names(), None);
/// assert_eq!(deserialized_minimal.include_metadata(), None);
/// assert_eq!(deserialized_minimal.min_amount(), None);
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContractSubscriptionParams {
    /// The identifier (e.g., address) of the smart contract to monitor. This
    /// field is mandatory.
    contract_id: String,

    /// An optional list of specific event names to subscribe to. If `None` or
    /// an empty list, the client subscribes to all events emitted by the
    /// specified contract.
    #[serde(skip_serializing_if = "Option::is_none")]
    event_names: Option<Vec<String>>,

    /// Optional flag indicating whether to include additional metadata (e.g.,
    /// block hash, transaction index) with the event data.
    ///
    /// - `Some(true)`: Include metadata.
    /// - `Some(false)`: Do not include metadata.
    /// - `None`: Use the server's default behavior.
    #[serde(skip_serializing_if = "Option::is_none")]
    include_metadata: Option<bool>,

    /// Optional minimum amount for filtering `ContractTransferEvents`. This
    /// field is typically relevant only when subscribing to the
    /// `ContractTransferEvents` topic. The amount should be specified as a
    /// string representing an unsigned integer (e.g., "1000000000") to
    /// handle large values precisely.
    ///
    /// If `Some`, only transfer events with an amount greater than or equal to
    /// this value will be sent. If `None`, no amount-based filtering is
    /// applied.
    #[serde(skip_serializing_if = "Option::is_none")]
    min_amount: Option<String>, /* Use String to handle potentially large
                                 * uints precisely */
}

// Type states for ContractSubscriptionParamsBuilder
mod contract_builder_states {
    /// Type state indicating the mandatory `contract_id` has not yet been set.
    #[derive(Debug, Default)]
    pub struct NoContractId;
    /// Type state indicating the mandatory `contract_id` has been set.
    #[derive(Debug)]
    pub struct WithContractId(pub String); // Store the contract_id
}

use contract_builder_states::*;

/// Type-state builder for [`ContractSubscriptionParams`].
///
/// Enforces that the mandatory `contract_id` field is set before building.
#[derive(Debug, Default)]
pub struct ContractSubscriptionParamsBuilder<State> {
    state: State,
    event_names: Option<Vec<String>>,
    include_metadata: Option<bool>,
    min_amount: Option<String>,
}

impl ContractSubscriptionParams {
    /// Creates a new type-state builder for `ContractSubscriptionParams`.
    ///
    /// Start by calling `.contract_id(...)` and then chain optional field
    /// methods before calling `.build()`.
    pub fn builder() -> ContractSubscriptionParamsBuilder<NoContractId> {
        ContractSubscriptionParamsBuilder::default()
    }

    /// Returns the contract identifier.
    pub fn contract_id(&self) -> &str {
        &self.contract_id
    }

    /// Returns a reference to the optional list of event names.
    pub fn event_names(&self) -> Option<&Vec<String>> {
        self.event_names.as_ref()
    }

    /// Returns the value of the `include_metadata` flag.
    pub fn include_metadata(&self) -> Option<bool> {
        self.include_metadata
    }

    /// Returns a reference to the optional minimum amount string for transfer
    /// events.
    pub fn min_amount(&self) -> Option<&String> {
        self.min_amount.as_ref()
    }

    // Private constructor primarily for the builder and potentially serde.
    fn new(
        contract_id: String,
        event_names: Option<Vec<String>>,
        include_metadata: Option<bool>,
        min_amount: Option<String>,
    ) -> Self {
        Self {
            contract_id,
            event_names,
            include_metadata,
            min_amount,
        }
    }
}

// Methods available in any state (setting optional fields)
impl<State> ContractSubscriptionParamsBuilder<State> {
    /// Sets the optional list of event names to filter by.
    pub fn event_names(mut self, names: Vec<String>) -> Self {
        self.event_names = Some(names);
        self
    }

    /// Sets the optional flag to include event metadata.
    pub fn include_metadata(mut self, include: bool) -> Self {
        self.include_metadata = Some(include);
        self
    }

    /// Sets the optional minimum amount string for transfer event filtering.
    pub fn min_amount(mut self, amount: String) -> Self {
        self.min_amount = Some(amount);
        self
    }
}

// Method available only when ContractId is NOT set (setting the required field)
impl ContractSubscriptionParamsBuilder<NoContractId> {
    /// Sets the mandatory contract identifier.
    ///
    /// This transitions the builder to the `WithContractId` state, allowing
    /// `build()` to be called.
    pub fn contract_id(
        self,
        id: String,
    ) -> ContractSubscriptionParamsBuilder<WithContractId> {
        ContractSubscriptionParamsBuilder {
            state: WithContractId(id),
            event_names: self.event_names,
            include_metadata: self.include_metadata,
            min_amount: self.min_amount,
        }
    }
}

// Method available only when ContractId IS set
impl ContractSubscriptionParamsBuilder<WithContractId> {
    /// Builds the final [`ContractSubscriptionParams`].
    ///
    /// This method is only available after the mandatory `contract_id` has been
    /// set.
    pub fn build(self) -> ContractSubscriptionParams {
        ContractSubscriptionParams::new(
            self.state.0, // Extract contract_id from the state
            self.event_names,
            self.include_metadata,
            self.min_amount,
        )
    }
}

/// Parameters for subscribing to mempool-related events (`MempoolAcceptance`,
/// `MempoolEvents`).
///
/// Allows clients to specify options when subscribing to mempool notifications,
/// such as filtering by contract ID or requesting detailed transaction
/// information.
///
/// Use [`MempoolSubscriptionParams::builder`] for manual construction.
///
/// # Examples
///
/// Building parameters manually:
///
/// ```
/// use rusk::jsonrpc::infrastructure::subscription::types::MempoolSubscriptionParams;
///
/// // Build with defaults (all fields None)
/// let params_default = MempoolSubscriptionParams::builder().build();
/// assert_eq!(params_default.contract_id(), None);
/// assert_eq!(params_default.include_details(), None);
///
/// // Build filtering by contract ID
/// let contract_id = "0x987fed".to_string();
/// let params_contract = MempoolSubscriptionParams::builder()
///     .contract_id(contract_id.clone())
///     .build();
/// assert_eq!(params_contract.contract_id(), Some(&contract_id));
/// assert_eq!(params_contract.include_details(), None);
///
/// // Build requesting details
/// let params_details = MempoolSubscriptionParams::builder()
///     .include_details(true)
///     .build();
/// assert_eq!(params_details.contract_id(), None);
/// assert_eq!(params_details.include_details(), Some(true));
///
/// // Build with both options
/// let params_both = MempoolSubscriptionParams::builder()
///     .contract_id(contract_id.clone())
///     .include_details(false) // Explicitly false
///     .build();
/// assert_eq!(params_both.contract_id(), Some(&contract_id));
/// assert_eq!(params_both.include_details(), Some(false));
/// ```
///
/// JSON Serialization/Deserialization:
///
/// ```
/// use serde_json;
/// use rusk::jsonrpc::infrastructure::subscription::types::MempoolSubscriptionParams;
///
/// // Serialize with contract_id and include_details = true
/// let params = MempoolSubscriptionParams::builder()
///     .contract_id("0xabc123".to_string())
///     .include_details(true)
///     .build();
/// let json = serde_json::to_string(&params).unwrap();
/// // Expect camelCase fields
/// assert_eq!(json, r#"{"contractId":"0xabc123","includeDetails":true}"#);
///
/// // Deserialize
/// let deserialized: MempoolSubscriptionParams = serde_json::from_str(&json).unwrap();
/// assert_eq!(deserialized.contract_id(), Some(&"0xabc123".to_string()));
/// assert_eq!(deserialized.include_details(), Some(true));
///
/// // Serialize with only include_details = false
/// let params_no_details = MempoolSubscriptionParams::builder()
///     .include_details(false)
///     .build();
/// let json_no_details = serde_json::to_string(&params_no_details).unwrap();
/// // Expect camelCase field
/// assert_eq!(json_no_details, r#"{"includeDetails":false}"#);
///
/// // Deserialize from missing fields
/// let json_empty = r#"{}"#;
/// let deserialized_empty: MempoolSubscriptionParams = serde_json::from_str(json_empty).unwrap();
/// assert_eq!(deserialized_empty.contract_id(), None);
/// assert_eq!(deserialized_empty.include_details(), None);
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct MempoolSubscriptionParams {
    /// Optional contract identifier (e.g., address) to filter mempool events.
    /// If `Some`, only events related to transactions involving this contract
    /// ID will be sent. If `None`, events for all contracts (or events not
    /// specific to a single contract) are included.
    #[serde(skip_serializing_if = "Option::is_none")]
    contract_id: Option<String>,

    /// Optional flag indicating whether to include detailed information (e.g.,
    /// full transaction data) in the mempool event notifications.
    ///
    /// - `Some(true)`: Include full details.
    /// - `Some(false)`: Include only basic information (e.g., transaction
    ///   hash).
    /// - `None`: Use the server's default level of detail.
    #[serde(skip_serializing_if = "Option::is_none")]
    include_details: Option<bool>,
}

impl MempoolSubscriptionParams {
    /// Creates a new builder for `MempoolSubscriptionParams`.
    ///
    /// This is the preferred way to construct the parameters manually.
    pub fn builder() -> MempoolSubscriptionParamsBuilder {
        MempoolSubscriptionParamsBuilder::default()
    }

    /// Returns a reference to the optional contract identifier.
    pub fn contract_id(&self) -> Option<&String> {
        self.contract_id.as_ref()
    }

    /// Returns the value of the `include_details` flag.
    pub fn include_details(&self) -> Option<bool> {
        self.include_details
    }

    // Private constructor primarily for the builder and potentially serde.
    fn new(contract_id: Option<String>, include_details: Option<bool>) -> Self {
        Self {
            contract_id,
            include_details,
        }
    }
}

/// Builder for [`MempoolSubscriptionParams`].
#[derive(Debug, Default)]
pub struct MempoolSubscriptionParamsBuilder {
    contract_id: Option<String>,
    include_details: Option<bool>,
}

impl MempoolSubscriptionParamsBuilder {
    /// Sets the optional contract ID to filter events by.
    pub fn contract_id(mut self, id: String) -> Self {
        self.contract_id = Some(id);
        self
    }

    /// Sets the optional flag to request detailed event information.
    pub fn include_details(mut self, include: bool) -> Self {
        self.include_details = Some(include);
        self
    }

    /// Builds the final [`MempoolSubscriptionParams`].
    pub fn build(self) -> MempoolSubscriptionParams {
        MempoolSubscriptionParams::new(self.contract_id, self.include_details)
    }
}
