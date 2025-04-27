// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # JSON-RPC Transaction Models
//!
//! This module defines the Rust data structures that correspond to the
//! transaction-related objects described in the Rusk JSON-RPC specification.
//!
//! These structures handle the serialization and deserialization of JSON
//! requests and responses involving transactions, their status, execution
//! details, and related events.
//!
//! ## Key Structures:
//!
//! - [`BaseTransaction`]: Common fields identifying a transaction (hash, type,
//!   gas, etc.).
//! - [`TransactionType`]: Enum differentiating `Phoenix` and `Moonlight`
//!   transactions.
//! - [`TransactionStatusType`], [`TransactionStatus`]: Representing whether a
//!   transaction is `Pending`, `Executed`, or `Failed`, along with execution
//!   details if applicable.
//! - [`MoonlightTransactionData`], [`PhoenixTransactionData`]: Specific data
//!   payloads for each transaction type.
//! - [`TransactionDataType`]: Enum holding either `MoonlightTransactionData` or
//!   `PhoenixTransactionData`.
//! - [`TransactionResponse`]: A common response format combining base info,
//!   optional status, and the specific data payload.
//! - [`MempoolTransaction`]: Represents a transaction currently in the mempool.
//! - [`TransactionInfo`]: Detailed information about a confirmed transaction,
//!   including its block context.
//! - [`SimulationResult`]: Outcome of simulating a transaction's execution.
//! - [`ContractEvent`], [`MoonlightEventGroup`]: Structures related to contract
//!   events emitted during transaction execution (primarily for archival
//!   purposes).
//! - [`SpendingIdentifier`], [`SpendingIdType`]: Used to identify transaction
//!   spending sources (nullifiers or account nonces).
//!
//! ## Conversions:
//!
//! `From` implementations facilitate converting internal node data types
//! (from `node_data`, `dusk_core`, etc.) into these JSON-RPC models.
//! Serialization helpers are used for formatting numbers as strings where
//! required by the spec.

use dusk_bytes::Serializable as DuskSerializable; // For to_var_bytes
use dusk_core::transfer::Transaction as ProtocolTransaction;
use hex;
use node_data::ledger::SpendingId as NodeSpendingId;
use node_data::ledger::Transaction as NodeTransaction;
use serde::{Deserialize, Serialize};
use std::convert::From;

// NOTE: Field types use appropriate Rust numerics internally, but
// large u64 values are serialized as Strings via `serde_helper`.

/// Represents a contract event emitted during transaction execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractEvent {
    /// 32-byte contract address that emitted the event.
    /// Serialized as a hex string.
    pub target: String,
    /// The name or topic identifying the type of event.
    pub topic: String,
    /// Event-specific data payload.
    /// Serialized as a hex string.
    pub data: String,
}

/// Groups contract events associated with a single Moonlight transaction.
///
/// This is primarily used for retrieving historical event data from archive
/// nodes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MoonlightEventGroup {
    /// A list of [`ContractEvent`]s emitted by the transaction.
    pub events: Vec<ContractEvent>,
    /// 32-byte hash of the transaction that emitted these events.
    /// Serialized as a 64-character hex string.
    pub tx_hash: String,
    /// Height of the block where the transaction was included.
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub block_height: u64,
}

// --- Base Transaction and Status Types ---

/// Distinguishes between different Rusk transaction protocols.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionType {
    /// A transaction using the Phoenix protocol (privacy-preserving).
    Phoenix,
    /// A transaction using the Moonlight protocol (transparent).
    Moonlight,
}

/// Base transaction information common across different JSON-RPC responses.
///
/// # Examples
///
/// ```
/// use rusk::jsonrpc::model::transaction::{BaseTransaction, TransactionType};
///
/// let base_tx = BaseTransaction {
///     tx_hash: "...hex...".to_string(),
///     version: 1,
///     tx_type: TransactionType::Phoenix,
///     gas_price: 1000,
///     gas_limit: 50000,
///     raw: "...hex...".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaseTransaction {
    /// 32-byte hash uniquely identifying the transaction.
    /// Serialized as a 64-character hex string.
    pub tx_hash: String,
    /// Protocol version number for this transaction type.
    pub version: u32,
    /// The type of transaction protocol used.
    pub tx_type: TransactionType,
    /// Gas price offered by the transaction sender (in atomic units).
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub gas_price: u64,
    /// Maximum gas units the transaction is allowed to consume.
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub gas_limit: u64,
    /// The complete, serialized transaction data.
    /// Serialized as a hex string.
    pub raw: String,
}

/// Represents the high-level execution status of a transaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionStatusType {
    /// Transaction is currently in the mempool, awaiting inclusion in a block.
    Pending,
    /// Transaction has been successfully included in a block and executed.
    Executed,
    /// Transaction was included in a block but failed during execution.
    Failed,
}

/// Provides detailed status information about a transaction's execution.
///
/// This is typically returned when specifically querying the status of a known
/// transaction hash.
///
/// # Examples
///
/// ```
/// use rusk::jsonrpc::model::transaction::{TransactionStatus, TransactionStatusType};
///
/// // Example of an executed transaction status
/// let executed_status = TransactionStatus {
///     status: TransactionStatusType::Executed,
///     block_height: Some(12345),
///     block_hash: Some("...hex...".to_string()),
///     gas_spent: Some(45000),
///     timestamp: Some(1678886400000), // Unix epoch seconds
///     error: None,
/// };
///
/// // Example of a pending transaction status
/// let pending_status = TransactionStatus {
///     status: TransactionStatusType::Pending,
///     block_height: None,
///     block_hash: None,
///     gas_spent: None,
///     timestamp: None,
///     error: None,
/// };
///
/// // Example of a failed transaction status
/// let failed_status = TransactionStatus {
///     status: TransactionStatusType::Failed,
///     block_height: Some(12346),
///     block_hash: Some("...hex...".to_string()),
///     gas_spent: Some(50000), // Gas limit might be spent even on failure
///     timestamp: Some(1678887000000),
///     error: Some("Execution error: Out of gas".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransactionStatus {
    /// The high-level execution status.
    pub status: TransactionStatusType,
    /// Block height where the transaction was included, if applicable
    /// (`Executed` or `Failed`).
    /// Serialized as a numeric string.
    #[serde(
        with = "super::serde_helper::opt_u64_to_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub block_height: Option<u64>,
    /// 32-byte hash of the block where the transaction was included, if
    /// applicable.
    /// Serialized as a 64-character hex string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<String>,
    /// Amount of gas consumed during execution, if applicable (`Executed` or
    /// `Failed`).
    /// Serialized as a numeric string.
    #[serde(
        with = "super::serde_helper::opt_u64_to_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub gas_spent: Option<u64>,
    /// Unix timestamp (in seconds) of the block where the transaction was
    /// included, if applicable.
    /// Serialized as a numeric string.
    #[serde(
        with = "super::serde_helper::opt_u64_to_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub timestamp: Option<u64>,
    /// Error message describing the reason for failure, only present if
    /// `status` is `Failed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// --- Transaction Data Payloads ---

/// Represents the specific data payload for a Moonlight transaction.
///
/// Moonlight transactions are transparent, meaning sender, receiver, and value
/// are public.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MoonlightTransactionData {
    /// Base58-encoded BLS public key of the transaction sender.
    pub sender: String,
    /// Optional Base58-encoded BLS public key of the transaction receiver.
    /// `None` typically indicates a contract deployment or interaction without
    /// a direct recipient.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver: Option<String>,
    /// Amount transferred in atomic units (e.g., Lovelace for Dusk).
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub value: u64,
    /// Nonce used by the sender for replay protection.
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub nonce: u64,
    /// Optional arbitrary data included in the transaction.
    /// Serialized as a hex string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}

/// Represents the specific data payload for a Phoenix transaction.
///
/// Phoenix transactions provide privacy using zero-knowledge proofs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PhoenixTransactionData {
    /// List of nullifiers spent by this transaction.
    /// Each nullifier is serialized as a hex string.
    pub nullifiers: Vec<String>,
    /// List of outputs (commitments or encrypted notes) created by this
    /// transaction.
    /// Each output is typically represented by its commitment or stealth
    /// address, serialized as a hex string.
    pub outputs: Vec<String>,
    /// Zero-knowledge proof data validating the transaction.
    /// Serialized as a hex string.
    pub proof: String,
}

/// Enum holding the data payload specific to different transaction types.
///
/// This uses `#[serde(untagged)]` so that the JSON serialization directly
/// contains the fields of either [`MoonlightTransactionData`] or
/// [`PhoenixTransactionData`], without an extra wrapping layer.
///
/// # Examples
///
/// ```
/// use rusk::jsonrpc::model::transaction::{TransactionDataType, MoonlightTransactionData, PhoenixTransactionData};
///
/// // Example Moonlight data
/// let moonlight_data = MoonlightTransactionData {
///     sender: "...base58...".to_string(),
///     receiver: Some("...base58...".to_string()),
///     value: 1000,
///     nonce: 1,
///     memo: Some("...hex...".to_string()),
/// };
/// let tx_data_moonlight = TransactionDataType::Moonlight(moonlight_data);
///
/// // Example Phoenix data
/// let phoenix_data = PhoenixTransactionData {
///     nullifiers: vec!["...hex...".to_string()],
///     outputs: vec!["...hex...".to_string()],
///     proof: "...hex...".to_string(),
/// };
/// let tx_data_phoenix = TransactionDataType::Phoenix(phoenix_data);
///
/// // Serialization would look like the inner struct's fields directly.
/// // let json_moonlight = serde_json::to_string(&tx_data_moonlight).unwrap();
/// // let json_phoenix = serde_json::to_string(&tx_data_phoenix).unwrap();
/// // assert!(json_moonlight.contains("sender"));
/// // assert!(json_phoenix.contains("nullifiers"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)] // Serialize as the inner type directly
pub enum TransactionDataType {
    /// Data specific to a Moonlight transaction.
    Moonlight(MoonlightTransactionData),
    /// Data specific to a Phoenix transaction.
    Phoenix(PhoenixTransactionData),
}

// --- Combined Transaction Response ---

/// Represents a standard response structure for queries returning transaction
/// information.
///
/// It combines the [`BaseTransaction`] information, an optional
/// [`TransactionStatus`], and the specific [`TransactionDataType`] payload.
///
/// # Examples
///
/// ```
/// use rusk::jsonrpc::model::transaction::{
///     TransactionResponse, BaseTransaction, TransactionStatus, TransactionStatusType,
///     TransactionDataType, PhoenixTransactionData, TransactionType
/// };
///
/// let base = BaseTransaction {
///     tx_hash: "tx_hash_hex".to_string(),
///     version: 1,
///     tx_type: TransactionType::Phoenix,
///     gas_price: 10, gas_limit: 1000,
///     raw: "raw_tx_hex".to_string()
/// };
/// let status = TransactionStatus {
///     status: TransactionStatusType::Executed,
///     block_height: Some(100), block_hash: Some("block_hash_hex".to_string()),
///     gas_spent: Some(900), timestamp: Some(1600000000), error: None
/// };
/// let data = TransactionDataType::Phoenix(PhoenixTransactionData {
///     nullifiers: vec!["nullifier_hex".to_string()],
///     outputs: vec!["output_hex".to_string()],
///     proof: "proof_hex".to_string()
/// });
///
/// let response = TransactionResponse {
///     base,
///     status: Some(status),
///     transaction_data: data,
/// };
///
/// // Serialization flattens base and status
/// // let json = serde_json::to_string(&response).unwrap();
/// // assert!(json.contains("\"txHash\":\"tx_hash_hex\""));
/// // assert!(json.contains("\"status\":\"Executed\""));
/// // assert!(json.contains("\"nullifiers\":[\"nullifier_hex\"]"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransactionResponse {
    /// Base transaction details (hash, type, gas, etc.).
    /// Fields are flattened into the main JSON object during serialization.
    #[serde(flatten)]
    pub base: BaseTransaction,
    /// Optional detailed execution status.
    /// Populated when status information is available and requested.
    /// Fields are flattened into the main JSON object during serialization.
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub status: Option<TransactionStatus>,
    /// Transaction-type-specific data payload ([`MoonlightTransactionData`] or
    /// [`PhoenixTransactionData`]).
    pub transaction_data: TransactionDataType,
}

/// Represents a transaction currently residing in the node's mempool (pending
/// inclusion in a block).
///
/// # Examples
///
/// ```
/// use rusk::jsonrpc::model::transaction::{
///     MempoolTransaction, BaseTransaction, TransactionDataType, PhoenixTransactionData, TransactionType
/// };
///
/// let base = BaseTransaction { /* ... */
/// #    tx_hash: "tx_hash_hex".to_string(), version: 1, tx_type: TransactionType::Phoenix,
/// #    gas_price: 10, gas_limit: 1000, raw: "raw_tx_hex".to_string() };
/// let data = TransactionDataType::Phoenix(PhoenixTransactionData { /* ... */
/// #    nullifiers: vec![], outputs: vec![], proof: "...".to_string() });
///
/// let mempool_tx = MempoolTransaction {
///     base,
///     transaction_data: data,
///     received_at: 1678887000, // Unix timestamp (seconds)
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MempoolTransaction {
    /// Base transaction details (hash, type, gas, etc.).
    /// Fields are flattened into the main JSON object during serialization.
    #[serde(flatten)]
    pub base: BaseTransaction,
    /// Transaction-type-specific data payload ([`MoonlightTransactionData`] or
    /// [`PhoenixTransactionData`]).
    pub transaction_data: TransactionDataType,
    /// Unix timestamp (in seconds) indicating when the transaction was
    /// received by the node's mempool.
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub received_at: u64,
}

/// Represents detailed information about a transaction that has been included
/// in a block.
///
/// Includes both the transaction data and its execution context within the
/// block.
///
/// # Examples
///
/// ```
/// use rusk::jsonrpc::model::transaction::{
///    TransactionInfo, BaseTransaction, TransactionDataType, PhoenixTransactionData, TransactionType
/// };
///
/// let base = BaseTransaction { /* ... */
/// #    tx_hash: "tx_hash_hex".to_string(), version: 1, tx_type: TransactionType::Phoenix,
/// #    gas_price: 10, gas_limit: 1000, raw: "raw_tx_hex".to_string() };
/// let data = TransactionDataType::Phoenix(PhoenixTransactionData { /* ... */
/// #    nullifiers: vec![], outputs: vec![], proof: "...".to_string() });
///
/// let tx_info = TransactionInfo {
///     base,
///     transaction_data: data,
///     block_height: 12345,
///     block_hash: "block_hash_hex".to_string(),
///     tx_index: Some(5),
///     gas_spent: 950,
///     timestamp: 1678886400, // Block timestamp (seconds)
///     error: None, // Or Some("Error message".to_string()) if failed
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransactionInfo {
    /// Base transaction details (hash, type, gas, etc.).
    /// Fields are flattened into the main JSON object during serialization.
    #[serde(flatten)]
    pub base: BaseTransaction,
    /// Transaction-type-specific data payload ([`MoonlightTransactionData`] or
    /// [`PhoenixTransactionData`]).
    pub transaction_data: TransactionDataType,

    // Block context
    /// Height of the block where the transaction was included.
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub block_height: u64,
    /// 32-byte hash of the block where the transaction was included.
    /// Serialized as a 64-character hex string.
    pub block_hash: String,
    /// Zero-based index of the transaction within the block's transaction
    /// list.
    /// `None` if the index was not requested or could not be determined.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_index: Option<u32>,
    /// Amount of gas consumed by the transaction during execution.
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub gas_spent: u64,
    /// Unix timestamp (in seconds) of the block where the transaction was
    /// included.
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub timestamp: u64,
    /// Error message if the transaction execution failed. `None` if
    /// successful.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Represents the outcome of simulating a transaction's execution without
/// actually including it in the blockchain.
///
/// Used for methods like `eth_estimateGas` or `eth_call` equivalents.
///
/// # Examples
///
/// ```
/// use rusk::jsonrpc::model::transaction::SimulationResult;
///
/// // Successful simulation
/// let success_result = SimulationResult {
///     success: true,
///     gas_estimate: Some(42000),
///     error: None,
/// };
///
/// // Failed simulation
/// let failure_result = SimulationResult {
///     success: false,
///     gas_estimate: None,
///     error: Some("Execution reverted: Invalid state".to_string()),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulationResult {
    /// Indicates whether the simulation completed successfully.
    pub success: bool,
    /// Estimated amount of gas the transaction would consume if executed.
    /// Present only if `success` is `true`.
    /// Serialized as a numeric string.
    #[serde(
        with = "crate::jsonrpc::model::serde_helper::opt_u64_to_string",
        default
    )] // default needed for Option
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_estimate: Option<u64>,
    /// Error message describing the reason for simulation failure.
    /// Present only if `success` is `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// --- Conversion Implementations ---

/// Converts the node's internal transaction representation
/// (`node_data::ledger::Transaction`) into the general JSON-RPC
/// `TransactionResponse` model.
///
/// This involves extracting common fields into [`BaseTransaction`] and
/// protocol-specific fields into the appropriate variant of
/// [`TransactionDataType`].
///
/// **Note:** The `status` field of the `TransactionResponse` is set to `None`
/// because the `node_data::ledger::Transaction` itself doesn't contain
/// execution status information. Status must be determined separately (e.g., by
/// checking if it's in the mempool or by querying for a `SpentTransaction`).
impl From<NodeTransaction> for TransactionResponse {
    fn from(node_tx: NodeTransaction) -> Self {
        let tx_hash = hex::encode(node_tx.id());
        let raw = hex::encode(node_tx.inner.to_var_bytes());

        let (tx_type, transaction_data) = match &node_tx.inner {
            ProtocolTransaction::Phoenix(p) => {
                let nullifiers = p
                    .nullifiers()
                    .iter()
                    .map(|n| hex::encode(n.to_bytes()))
                    .collect();
                let outputs = p
                    .outputs()
                    .iter()
                    .map(|note| hex::encode(note.stealth_address().to_bytes()))
                    .collect();
                let proof = hex::encode(p.proof());

                (
                    TransactionType::Phoenix,
                    TransactionDataType::Phoenix(PhoenixTransactionData {
                        nullifiers,
                        outputs, // Vec<String> of hex-encoded stealth addresses
                        proof,
                    }),
                )
            }
            ProtocolTransaction::Moonlight(m) => {
                let sender = bs58::encode(m.sender().to_bytes()).into_string();
                let receiver = m
                    .receiver()
                    .map(|r| bs58::encode(r.to_bytes()).into_string());
                let value = m.value();
                let nonce = m.nonce();
                let memo = m.memo().map(hex::encode);

                (
                    TransactionType::Moonlight,
                    TransactionDataType::Moonlight(MoonlightTransactionData {
                        sender,
                        receiver,
                        value,
                        nonce,
                        memo,
                    }),
                )
            }
        };

        let base = BaseTransaction {
            tx_hash,
            version: node_tx.version,
            tx_type,
            gas_price: node_tx.gas_price(),
            gas_limit: node_tx.inner.gas_limit(),
            raw,
        };

        TransactionResponse {
            base,
            status: None, // Status is unknown when just fetching from block
            transaction_data,
        }
    }
}

/// Converts the node's internal contract event representation
/// (`node_data::events::contract::ContractEvent`) into the JSON-RPC
/// `ContractEvent` model.
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

/// Converts the node's internal Moonlight event group representation
/// (`node::archive::MoonlightGroup`) into the JSON-RPC `MoonlightEventGroup`
/// model.
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

/// Represents the type of identifier used for tracking spending (nullifier or
/// account nonce).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SpendingIdType {
    /// The identifier is a Phoenix nullifier.
    Nullifier,
    /// The identifier is an account public key combined with a nonce
    /// (Moonlight).
    AccountNonce,
}

/// Represents a spending identifier, which can be either a Phoenix nullifier or
/// a Moonlight account nonce pair.
///
/// This is used, for example, in tracking transaction dependencies or checking
/// for double spends.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpendingIdentifier {
    /// The type of the spending identifier.
    pub id_type: SpendingIdType,
    /// The core identifier value.
    /// - If `id_type` is `Nullifier`, this is the hex-encoded nullifier.
    /// - If `id_type` is `AccountNonce`, this is the base58-encoded account
    ///   public key.
    pub identifier: String,
    /// The nonce associated with the account, only present if `id_type` is
    /// `AccountNonce`.
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::opt_u64_to_string", default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<u64>,
}

/// Converts the node's internal spending ID representation
/// (`node_data::ledger::SpendingId`) into the JSON-RPC `SpendingIdentifier`
/// model.
impl From<NodeSpendingId> for SpendingIdentifier {
    fn from(node_id: NodeSpendingId) -> Self {
        match node_id {
            NodeSpendingId::Nullifier(n) => SpendingIdentifier {
                id_type: SpendingIdType::Nullifier,
                identifier: hex::encode(n),
                nonce: None,
            },
            NodeSpendingId::AccountNonce(account, nonce) => {
                SpendingIdentifier {
                    id_type: SpendingIdType::AccountNonce,
                    identifier: bs58::encode(account.to_bytes()).into_string(),
                    nonce: Some(nonce),
                }
            }
        }
    }
}
