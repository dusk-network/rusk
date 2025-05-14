// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # JSON-RPC Archive Models
//!
//! This module defines the data structures used for representing information
//! retrieved from the node's archive component within the JSON-RPC API.
//!
//! The archive stores historical blockchain data, optimized for specific
//! queries that might be inefficient on the live state database. These models
//! provide the serialization format for transferring this archived data over
//! JSON-RPC.
//!
//! ## Key Structures:
//!
//! - [`ArchivedEvent`]: Represents a generic event retrieved from the archive,
//!   mirroring the internal `node::archive::sqlite::data::ArchivedEvent` but
//!   adapted for the JSON-RPC layer.
//! - [`ContractEvent`]: Represents a specific event emitted by a contract,
//!   often contained within a [`MoonlightEventGroup`].
//! - [`MoonlightEventGroup`]: Groups related contract events (typically
//!   [`ContractEvent`]s) associated with a single transaction origin (like a
//!   Moonlight transaction hash) retrieved from the archive.
//! - [`Order`]: An enum used to specify the desired sorting order (Ascending or
//!   Descending) for query results, usually based on block height.
//!
//! These models ensure a stable and well-defined interface for clients
//! interacting with the Rusk node's archived data via JSON-RPC.

use hex;
use serde::{Deserialize, Serialize};

/// Represents an event retrieved from the archive.
///
/// This struct mirrors the structure used internally by the node's archive
/// component (`node::archive::sqlite::data::ArchivedEvent`) but is adapted for
/// the JSON-RPC layer.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ArchivedEvent {
    /// The transaction hash or origin identifier associated with the event.
    pub origin: String,
    /// The topic categorizing the event (e.g., "moonlight", "stake").
    pub topic: String,
    /// The source identifier, typically a contract ID, that emitted the event.
    #[serde(rename = "target")]
    pub source: String,
    /// The raw event data payload.
    #[serde(with = "super::serde_helper::base64_vec_u8")]
    pub data: Vec<u8>,
}

/// Represents a single event emitted by a smart contract call within a
/// transaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractEvent {
    /// Contract address (or identifier) that emitted the event.
    /// Serialized as a hex string or other suitable identifier.
    pub target: String,
    /// The name or topic identifying the type of event.
    pub topic: String,
    /// Event-specific data payload.
    /// Serialized typically as a hex string or JSON object depending on the
    /// event.
    pub data: String,
}

/// Groups contract events associated with a single Moonlight transaction
/// origin.
///
/// This is primarily used for retrieving historical event data from archive
/// nodes, often grouped by the transaction hash.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MoonlightEventGroup {
    /// A list of [`ContractEvent`]s emitted by the transaction.
    pub events: Vec<ContractEvent>,
    /// Identifier of the transaction or origin that emitted these events.
    /// Serialized as a 64-character hex string (typically the transaction
    /// hash).
    pub origin: String, /* Renamed from tx_hash to match plan's intent, kept
                         * as String */
    /// Height of the block where the transaction was included.
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub block_height: u64,
}

/// Converts the node's internal Moonlight event group representation
/// (`node::archive::MoonlightGroup`) into the JSON-RPC `MoonlightEventGroup`
/// model.
///
/// Requires the `archive` feature to be enabled as it depends on
/// `node::archive` types.
#[cfg(feature = "archive")]
impl From<node::archive::MoonlightGroup> for MoonlightEventGroup {
    fn from(group: node::archive::MoonlightGroup) -> Self {
        Self {
            // Use the hex-encoded origin hash from the node type
            origin: hex::encode(group.origin()),
            block_height: group.block_height(),
            // Convert each node event within the group
            events: group
                .events()
                .iter()
                .cloned()
                .map(ContractEvent::from) // Assumes ContractEvent::from exists
                .collect(),
        }
    }
}

/// Converts the node's internal contract event representation
/// (`node_data::events::contract::ContractEvent`) into the JSON-RPC
/// `ContractEvent` model.
///
/// Requires the `archive` feature as `ContractEvent` is now part of the archive
/// model.
#[cfg(feature = "archive")]
impl From<node_data::events::contract::ContractEvent> for ContractEvent {
    fn from(node_event: node_data::events::contract::ContractEvent) -> Self {
        Self {
            // Convert target address (ContractId) to string. Assuming Display
            // impl or similar. If hex is needed, adjust
            // accordingly.
            target: node_event.target.to_string(),
            topic: node_event.topic,
            // Convert event data to hex string
            data: hex::encode(node_event.data),
        }
    }
}

/// Specifies the sorting order for query results, typically by block height.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")] // Use camelCase for JSON representation
pub enum Order {
    /// Sort results in ascending order (e.g., oldest block first).
    Ascending,
    /// Sort results in descending order (e.g., newest block first).
    Descending,
}

impl From<node::archive::ArchivedEvent> for ArchivedEvent {
    fn from(node_event: node::archive::ArchivedEvent) -> Self {
        Self {
            origin: node_event.origin,
            topic: node_event.topic,
            source: node_event.source,
            data: node_event.data,
        }
    }
}
