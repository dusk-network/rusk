// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Defines data structures representing transaction-related information for the
//! JSON-RPC API

use hex;
use serde::{Deserialize, Serialize};
use std::convert::From;

// NOTE: Field types (String vs specific types) are chosen for direct mapping
// from the spec. Adjustments for internal representation vs. final JSON
// stringification (e.g., u64 -> String) will happen at the serialization
// layer.

/// Represents a contract event as defined in the JSON-RPC specification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractEvent {
    /// 32-byte contract address as hex string.
    pub target: String,
    /// Event topic name.
    pub topic: String,
    /// Event data as hex string.
    pub data: String,
}

/// Represents a group of events related to a single transaction, specifically
/// for Moonlight operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MoonlightEventGroup {
    /// Events associated with the transaction.
    pub events: Vec<ContractEvent>,
    /// Transaction hash as hex string (origin hash internally).
    pub tx_hash: String,
    /// Block height where the transaction was included.
    #[serde(with = "super::serde_helper::u64_to_string")]
    // Serialize as string
    pub block_height: u64,
}

// --- Base Transaction and Status Types ---

/// Represents the transaction type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionType {
    Phoenix,
    Moonlight,
}

/// Base transaction information common to different views.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaseTransaction {
    /// Transaction hash as hex string.
    pub tx_hash: String,
    /// Transaction version number.
    pub version: u32,
    /// Transaction type ("Phoenix" or "Moonlight").
    pub tx_type: TransactionType,
    /// Gas price in atomic units as numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub gas_price: u64,
    /// Gas limit as numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub gas_limit: u64,
    /// Complete transaction data as hex string.
    pub raw: String,
}

/// Represents the execution status of a transaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionStatusType {
    Pending,
    Executed,
    Failed,
}

/// Detailed status information for a transaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransactionStatus {
    /// Execution status.
    pub status: TransactionStatusType,
    /// Block height as numeric string, if executed.
    #[serde(
        with = "super::serde_helper::opt_u64_to_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub block_height: Option<u64>,
    /// Block hash as hex string, if executed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<String>,
    /// Gas spent as numeric string, if executed.
    #[serde(
        with = "super::serde_helper::opt_u64_to_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub gas_spent: Option<u64>,
    /// Block timestamp as numeric string, if executed.
    #[serde(
        with = "super::serde_helper::opt_u64_to_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub timestamp: Option<u64>,
    /// Error message, if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// --- Transaction Data Payloads ---

/// Data specific to a Moonlight transaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MoonlightTransactionData {
    /// Base58-encoded BLS public key of the sender.
    pub sender: String,
    /// Base58-encoded BLS public key of the receiver.
    pub receiver: String,
    /// Amount in Dusk atomic units as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub value: u64,
    /// Transaction nonce as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub nonce: u64,
    /// Optional hex-encoded memo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}

/// Data specific to a Phoenix transaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PhoenixTransactionData {
    /// Array of hex-encoded nullifiers.
    pub nullifiers: Vec<String>,
    /// Array of hex-encoded outputs.
    pub outputs: Vec<String>,
    /// Hex-encoded zero-knowledge proof.
    pub proof: String,
}

/// Enum holding the data payload for different transaction types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)] // Serialize as the inner type directly
pub enum TransactionDataType {
    Moonlight(MoonlightTransactionData),
    Phoenix(PhoenixTransactionData),
}

// --- Combined Transaction Response ---

/// Represents the full response for a transaction query.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransactionResponse {
    #[serde(flatten)]
    pub base: BaseTransaction,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub status: Option<TransactionStatus>,
    /// Transaction-specific data payload.
    pub transaction_data: TransactionDataType,
}

/// Represents the result of a simulated transaction execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulationResult {
    /// Whether the simulation was successful.
    pub success: bool,
    /// Estimated gas usage as a numeric string, present only on success.
    #[serde(
        with = "crate::jsonrpc::model::serde_helper::opt_u64_to_string",
        default
    )] // default needed for Option
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_estimate: Option<u64>,
    /// Error message if the simulation failed, present only on failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// --- Conversion Implementations ---

#[cfg(feature = "archive")]
impl From<node_data::events::contract::ContractEvent> for ContractEvent {
    fn from(node_event: node_data::events::contract::ContractEvent) -> Self {
        Self {
            target: node_event.target.to_string(), /* Convert ContractId to
                                                    * String */
            topic: node_event.topic,
            data: hex::encode(node_event.data), // Convert Vec<u8> to hex String
        }
    }
}

#[cfg(feature = "archive")]
impl From<node::archive::MoonlightGroup> for MoonlightEventGroup {
    fn from(group: node::archive::MoonlightGroup) -> Self {
        Self {
            tx_hash: hex::encode(group.origin()),
            block_height: group.block_height(),
            events: group
                .events()
                .iter()
                .cloned()
                .map(ContractEvent::from)
                .collect(),
        }
    }
}
