// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # JSON-RPC Block Models
//!
//! This module defines the Rust data structures that correspond to the
//! block-related objects described in the Rusk JSON-RPC specification.
//!
//! These structures are used for serialization and deserialization of JSON
//! requests and responses involving block information.
//!
//! ## Key Structures:
//!
//! - [`BlockHeader`]: Represents the metadata of a block.
//! - [`Block`]: Represents a full block, potentially including transactions.
//! - [`CandidateBlock`]: Represents a block proposed during consensus but not
//!   yet finalized.
//! - [`BlockStatus`]: Indicates whether a block is `Final` or `Provisional`.
//! - [`ChainTip`]: Represents a potential head of the blockchain (used for fork
//!   detection).
//! - [`BlockFaults`], [`Fault`], [`FaultItem`], [`FaultType`]: Structures for
//!   representing consensus faults reported within a block.
//!
//! ## Conversions:
//!
//! `From` implementations are provided to convert data structures from the
//! `node_data` crate (representing the node's internal ledger state) into
//! these JSON-RPC model structures.
//! Fields requiring specific serialization formats (e.g., `u64` to JSON
//! string) utilize helpers from the `serde_helper` module.

use crate::jsonrpc::model::transaction::TransactionResponse;
use hex;
use node_data::ledger;
use serde::{Deserialize, Serialize};
use std::convert::From;

use dusk_bytes::Serializable;
use node_data::ledger::Fault as NodeFault;
use node_data::message::ConsensusHeader as NodeConsensusHeader;
use node_data::message::SignInfo as NodeSignInfo;

use crate::jsonrpc::model::key::AccountPublicKey;

// NOTE: Field types use appropriate Rust numerics internally, but
// large u64 values are serialized as Strings via `serde_helper`.

/// Represents the header of a block for the JSON-RPC API.
///
/// This structure contains key metadata about a block, formatted according to
/// the JSON-RPC specification.
///
/// # Examples
///
/// ```
/// use rusk::jsonrpc::model::block::BlockHeader;
///
/// let header = BlockHeader {
///     version: 1,
///     height: 12345,
///     previous_hash: "...hex...".to_string(),
///     timestamp: 1678886400000,
///     hash: "...hex...".to_string(),
///     state_hash: "...hex...".to_string(),
///     validator: "...base58...".to_string(),
///     transactions_root: "...hex...".to_string(),
///     gas_limit: 1000000,
///     seed: "...hex...".to_string(),
///     sequence: 0,
/// };
///
/// // Typically obtained via `From<node_data::ledger::Header>`
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Protocol version number.
    pub version: u32, // `node_data::block::Header::version` is u8
    /// Block height (number) in the chain.
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub height: u64,
    /// 32-byte hash of the preceding block in the chain.
    /// Serialized as a 64-character hex string.
    pub previous_hash: String,
    /// Unix timestamp representing the block creation time in
    /// **milliseconds**. Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub timestamp: u64,
    /// 32-byte hash of this block.
    /// Serialized as a 64-character hex string.
    pub hash: String,
    /// 32-byte hash representing the state of the blockchain after this block.
    /// Serialized as a 64-character hex string.
    pub state_hash: String,
    /// Base58-encoded BLS public key of the node that generated this block.
    pub validator: String,
    /// 32-byte Merkle root hash of all transactions included in this block.
    /// Serialized as a 64-character hex string.
    pub transactions_root: String,
    /// The maximum amount of gas allowed for all transactions in this block.
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub gas_limit: u64,
    /// 32-byte random seed generated for this block.
    /// Serialized as a 64-character hex string.
    pub seed: String,
    /// Consensus iteration number within the round (block height) when this
    /// block was proposed.
    pub sequence: u32, // `node_data::block::Header::iteration` is u8
}

// Manual implementation to ensure all fields are compared.
// Manual implementation has solved the issue in tests.
impl PartialEq for BlockHeader {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
            && self.height == other.height
            && self.previous_hash == other.previous_hash
            && self.timestamp == other.timestamp
            && self.hash == other.hash
            && self.state_hash == other.state_hash
            && self.validator == other.validator
            && self.transactions_root == other.transactions_root
            && self.gas_limit == other.gas_limit
            && self.seed == other.seed
            && self.sequence == other.sequence
    }
}
impl Eq for BlockHeader {}

/// Represents the finality status of a block in the JSON-RPC API.
///
/// This indicates whether a block is considered permanently part of the
/// canonical chain (`Final`) or if it could potentially be reorganized out
/// (`Provisional`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BlockFinalityStatus {
    /// Block is finalized and considered irreversible.
    Final,
    /// Block is part of the canonical chain but not yet finalized.
    Accepted,
    /// Block is not known to be part of the canonical chain.
    Unknown,
}

/// Represents the label assigned to a block in the ledger, simplified for
/// the JSON-RPC API.
///
/// This maps the more granular internal `node_data::ledger::Label` variants
/// (Accepted, Attested, Confirmed, Final) to a simpler status relevant for
/// RPC consumers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BlockStatus {
    /// The block has reached finality.
    Final,
    /// The block is accepted but not yet final (includes Accepted, Attested,
    /// Confirmed internal states).
    Provisional,
}

/// Represents a potential head of the blockchain (chain tip).
///
/// This is typically used in responses that might indicate the presence of
/// forks.
///
/// # Examples
///
/// ```
/// use rusk::jsonrpc::model::block::{ChainTip, BlockStatus};
///
/// let tip = ChainTip {
///     hash: "...hex...".to_string(),
///     height: 12345,
///     status: BlockStatus::Final,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainTip {
    /// 32-byte hash of the block at this tip.
    /// Serialized as a 64-character hex string.
    pub hash: String,
    /// Height of the block at this tip.
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub height: u64,
    /// Finality status of the block at this tip.
    pub status: BlockStatus,
}

/// Represents a full block, potentially including its transactions, for the
/// JSON-RPC API.
///
/// # Examples
///
/// ```
/// use rusk::jsonrpc::model::block::{Block, BlockHeader, BlockStatus};
/// use rusk::jsonrpc::model::transaction::TransactionResponse;
///
/// // Header needs to be populated correctly
/// let header = BlockHeader { /* ... fields ... */
/// #    version: 1,
/// #    height: 12345,
/// #    previous_hash: "...".to_string(),
/// #    timestamp: 1678886400000,
/// #    hash: "...".to_string(),
/// #    state_hash: "...".to_string(),
/// #    validator: "...".to_string(),
/// #    transactions_root: "...".to_string(),
/// #    gas_limit: 1000000,
/// #    seed: "...".to_string(),
/// #    sequence: 0,
/// };
///
/// let block = Block {
///     header,
///     status: Some(BlockStatus::Final),
///     transactions: None, // Or Some(vec![...]) if requested
///     faults: None, // Or Some(BlockFaults { faults: vec![] }) if requested
///     transactions_count: 10,
///     block_reward: Some(5000000000), // Example reward
///     total_gas_limit: Some(800000), // Example sum of tx gas limits
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Block {
    /// The header containing metadata for this block.
    pub header: BlockHeader,
    /// The finalization status of the block (`Final` or `Provisional`).
    /// This field might be `None` if the status cannot be determined or is not
    /// applicable in the context (e.g., when converting directly from
    /// `node_data::ledger::Block` without label info).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<BlockStatus>,
    /// Optional list of transactions included in the block.
    /// This is populated only if the client specifically requested
    /// transactions (e.g., via an `include_txs` flag). Otherwise, it is
    /// `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transactions: Option<Vec<TransactionResponse>>,
    /// Optional list of faults detected in the block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faults: Option<BlockFaults>,
    /// The total number of transactions included in this block.
    /// This is always present, even if `transactions` is `None`.
    pub transactions_count: u64, // Serialized as number (u64 fits JS number)
    /// Total reward distributed in this block (e.g., to the generator and
    /// voters).
    /// Serialized as a numeric string.
    /// This field is optional as it might not be readily available without
    /// processing block execution results.
    #[serde(with = "super::serde_helper::opt_u64_to_string", default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_reward: Option<u64>,
    /// The sum of the `gas_limit` specified by each transaction in the block.
    /// **Note:** This is *not* the total gas *consumed* by the block's
    /// execution, but rather the sum of the limits declared by the included
    /// transactions.
    /// Serialized as a numeric string.
    /// This field is optional as it might require iterating through
    /// transactions if not pre-calculated.
    #[serde(with = "super::serde_helper::opt_u64_to_string", default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_gas_limit: Option<u64>,
}

/// Represents a candidate block proposed during consensus, before finalization.
///
/// This structure is similar to [`Block`] but typically always includes the
/// transactions, as it represents a specific proposed block content.
///
/// # Examples
///
/// ```
/// use rusk::jsonrpc::model::block::{CandidateBlock, BlockHeader};
/// use rusk::jsonrpc::model::transaction::TransactionResponse;
///
/// // Header needs to be populated correctly
/// let header = BlockHeader { /* ... fields ... */
/// #    version: 1,
/// #    height: 12346,
/// #    previous_hash: "...".to_string(),
/// #    timestamp: 1678887000000,
/// #    hash: "...".to_string(),
/// #    state_hash: "...".to_string(),
/// #    validator: "...".to_string(),
/// #    transactions_root: "...".to_string(),
/// #    gas_limit: 1000000,
/// #    seed: "...".to_string(),
/// #    sequence: 1,
/// };
/// let transactions: Vec<TransactionResponse> = vec![/* ... transaction responses ... */];
///
/// let candidate_block = CandidateBlock {
///     header,
///     transactions_count: transactions.len() as u64,
///     transactions,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateBlock {
    /// The header containing metadata for this candidate block.
    pub header: BlockHeader,
    /// List of transactions included in this candidate block.
    pub transactions: Vec<TransactionResponse>,
    /// The total number of transactions included in this candidate block.
    pub transactions_count: u64,
}

// Manual implementation to ensure all fields are compared.
// Manual implementation has solved the issue in tests.
impl PartialEq for CandidateBlock {
    fn eq(&self, other: &Self) -> bool {
        self.header == other.header
            && self.transactions == other.transactions
            && self.transactions_count == other.transactions_count
    }
}
impl Eq for CandidateBlock {}

/// Represents consensus fault information included within a finalized block.
///
/// This structure aggregates multiple individual [`Fault`] instances reported
/// in the block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockFaults {
    /// A list of consensus faults.
    pub faults: Vec<Fault>,
}

/// Represents a single consensus fault, indicating conflicting messages sent by
/// a node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Fault {
    /// The category of the consensus fault.
    pub fault_type: FaultType,
    /// Information identifying the first conflicting message or item.
    pub item1: FaultItem,
    /// Information identifying the second conflicting message or item.
    pub item2: FaultItem,
}

/// Enumerates the different types of consensus faults that can be reported.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FaultType {
    /// A node proposed two different candidate blocks for the same round and
    /// iteration.
    DoubleCandidate,
    /// A node cast two different ratification votes for the same round and
    /// iteration.
    DoubleRatificationVote,
    /// A node cast two different validation votes for the same round and
    /// iteration.
    DoubleValidationVote,
}

/// Represents the identifying information for one of the conflicting items in a
/// consensus fault.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FaultItem {
    /// Header identifying the consensus context (round, iteration,
    /// previous block).
    pub header: ConsensusHeaderJson,
    /// Base58-encoded BLS public key of the node that signed the conflicting
    /// item.
    pub signer_key: String,
}

/// Represents a consensus header, formatted for JSON serialization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsensusHeaderJson {
    /// The consensus round number, typically corresponding to block height.
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub round: u64,
    /// The consensus iteration number within the round.
    pub iteration: u8,
    /// 32-byte hash of the previous block used as context for this consensus
    /// round.
    /// Serialized as a 64-character hex string.
    pub prev_block_hash: String,
}

// --- Conversion Implementations ---

/// Converts the node's internal block header representation
/// (`node_data::ledger::Header`) into the JSON-RPC `BlockHeader` model.
impl From<ledger::Header> for BlockHeader {
    fn from(h: ledger::Header) -> Self {
        let validator_base58 =
            bs58::encode(h.generator_bls_pubkey.inner()).into_string();

        Self {
            version: h.version.into(),
            height: h.height,
            previous_hash: hex::encode(h.prev_block_hash),
            timestamp: h.timestamp,
            hash: hex::encode(h.hash),
            state_hash: hex::encode(h.state_hash),
            validator: validator_base58,
            transactions_root: hex::encode(h.txroot),
            gas_limit: h.gas_limit,
            seed: hex::encode(h.seed.inner()),
            sequence: h.iteration.into(),
        }
    }
}

/// Converts the node's internal ledger label (`node_data::ledger::Label`)
/// into the simplified JSON-RPC `BlockStatus`.
///
/// `Final` maps directly to `Final`. `Accepted`, `Attested`, and `Confirmed`
/// all map to `Provisional`.
impl From<node_data::ledger::Label> for BlockStatus {
    fn from(label: node_data::ledger::Label) -> Self {
        match label {
            node_data::ledger::Label::Final(_) => BlockStatus::Final,
            // Map Accepted, Attested, Confirmed to Provisional
            node_data::ledger::Label::Accepted(_)
            | node_data::ledger::Label::Attested(_)
            | node_data::ledger::Label::Confirmed(_) => {
                BlockStatus::Provisional
            }
        }
    }
}

/// Converts the node's internal finalized block with spent transactions
/// representation (`node_data::ledger::BlockWithSpentTransactions`) into the
/// JSON-RPC `Block` model.
impl From<ledger::BlockWithSpentTransactions> for Block {
    fn from(b: ledger::BlockWithSpentTransactions) -> Self {
        let header = BlockHeader::from(b.header().clone());
        let txs: Vec<TransactionResponse> = b
            .txs()
            .iter()
            .map(|tx| TransactionResponse::from(tx.clone()))
            .collect();
        let faults: BlockFaults = BlockFaults::from(b.faults().clone());
        let status = BlockStatus::from(b.label().clone());

        let total_gas_limit = txs.iter().map(|tx| tx.base.gas_limit).sum();
        let transactions_count = txs.len() as u64;

        Self {
            header,
            status: Some(status),
            transactions: Some(txs),
            faults: Some(faults),
            transactions_count,
            block_reward: None,
            total_gas_limit: Some(total_gas_limit),
        }
    }
}

/// Converts the node's internal block representation
/// (`node_data::ledger::Block`) into the JSON-RPC `CandidateBlock` model.
///
/// This conversion includes converting the transactions contained within the
/// `ledger::Block` into [`TransactionResponse`] models.
impl From<ledger::Block> for CandidateBlock {
    fn from(node_block: ledger::Block) -> Self {
        let header_model = BlockHeader::from(node_block.header().clone());
        // ledger::Block contains node_data::ledger::Transaction, which needs
        // conversion to TransactionResponse.
        let txs_model = node_block
            .txs()
            .iter()
            .map(|tx| TransactionResponse::from(tx.clone()))
            .collect::<Vec<_>>();
        let transactions_count = txs_model.len() as u64;

        Self {
            header: header_model,
            transactions: txs_model,
            transactions_count,
        }
    }
}

/// Converts the node's internal consensus header
/// (`node_data::message::ConsensusHeader`) into the JSON-RPC
/// `ConsensusHeaderJson` model.
impl From<&NodeConsensusHeader> for ConsensusHeaderJson {
    fn from(h: &NodeConsensusHeader) -> Self {
        Self {
            round: h.round,
            iteration: h.iteration,
            prev_block_hash: hex::encode(h.prev_block_hash),
        }
    }
}

/// Converts the node's internal signing info (`node_data::message::SignInfo`)
/// into the `AccountPublicKey` model (which wraps a base58 string).
///
/// **Note:** This currently extracts the BLS public key. Depending on future
/// changes, this might need adjustment if `SignInfo` changes structure.
impl<'a> From<&'a NodeSignInfo> for AccountPublicKey {
    fn from(si: &'a NodeSignInfo) -> Self {
        // AccountPublicKey wraps BlsPublicKey
        // (dusk_core::signatures::bls::PublicKey). si.signer is of type
        // node_data::bls::PublicKey. We need the inner
        // dusk_core::signatures::bls::PublicKey.
        AccountPublicKey(*si.signer.inner())
    }
}

/// Helper function to create a [`FaultItem`] from node data components.
fn fault_item_from_node_data(
    header: &NodeConsensusHeader,
    sign_info: &NodeSignInfo,
) -> FaultItem {
    // Get the inner BlsPublicKey which implements Serializable
    let inner_signer_key = sign_info.signer.inner();
    let signer_bytes = Serializable::to_bytes(inner_signer_key);
    let signer_key = bs58::encode(signer_bytes).into_string();

    let header_json = ConsensusHeaderJson::from(header);

    FaultItem {
        header: header_json,
        signer_key,
    }
}

/// Converts the node's internal fault representation
/// (`node_data::ledger::Fault`) into the JSON-RPC `Fault` model.
impl From<node_data::ledger::Fault> for Fault {
    fn from(node_fault: node_data::ledger::Fault) -> Self {
        // Use the item1() and item2() methods provided by
        // node_data::ledger::Fault to access header and sign_info
        // without needing to match on variants or access private
        // fields.
        let (header1, sig1) = node_fault.item1();
        let (header2, sig2) = node_fault.item2();

        // Determine FaultType based on node_data::ledger::Fault variant
        let fault_type = match node_fault {
            node_data::ledger::Fault::DoubleCandidate { .. } => {
                FaultType::DoubleCandidate
            }
            node_data::ledger::Fault::DoubleRatificationVote { .. } => {
                FaultType::DoubleRatificationVote
            }
            node_data::ledger::Fault::DoubleValidationVote { .. } => {
                FaultType::DoubleValidationVote
            }
        };

        let item1 = fault_item_from_node_data(header1, sig1);
        let item2 = fault_item_from_node_data(header2, sig2);

        Fault {
            fault_type,
            item1,
            item2,
        }
    }
}

/// Converts a vector of node's internal faults
/// (`Vec<node_data::ledger::Fault>`) into the JSON-RPC `BlockFaults` model.
impl From<Vec<node_data::ledger::Fault>> for BlockFaults {
    fn from(node_faults: Vec<NodeFault>) -> Self {
        let faults = node_faults
            .into_iter()
            .map(Fault::from) // Use the fallible TryFrom conversion for each Fault
            .collect();
        BlockFaults { faults }
    }
}
