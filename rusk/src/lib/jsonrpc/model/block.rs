// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Defines data structures representing block-related information for the
//! JSON-RPC API, based on the specifications.

use crate::jsonrpc::model::transaction::TransactionResponse;
use hex;
use node_data::ledger;
use serde::{Deserialize, Serialize};
use std::convert::From;

// NOTE: Field types use appropriate Rust numerics internally, but
// large u64 values are serialized as Strings via `serde_helper`.

/// Represents the header of a block as defined in the JSON-RPC specification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockHeader {
    /// Protocol version number.
    pub version: u32, // `node_data::block::Header::version` is u8
    /// Block height as numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub height: u64,
    /// Previous block hash as hex string.
    pub previous_hash: String,
    /// Unix timestamp in milliseconds.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub timestamp: u64,
    /// Block hash as hex string.
    pub hash: String,
    /// State hash as hex string.
    pub state_hash: String,
    /// Validator's BLS public key as base58 string.
    pub validator: String,
    /// Merkle root of transactions as hex string.
    pub transactions_root: String,
    /// Maximum gas allowed in the block as numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub gas_limit: u64,
    /// Random seed for the block as hex string.
    pub seed: String,
    /// Block sequence number in consensus round.
    pub sequence: u32, // `node_data::block::Header::iteration` is u8
}

/// Represents the finalization status of a block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BlockStatus {
    /// Block has reached finality.
    Final,
    /// Block is accepted but not yet final.
    Provisional,
}

/// Represents a full block as defined in the JSON-RPC specification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Block {
    /// The block header.
    pub header: BlockHeader,
    /// The finalization status of the block.
    pub status: BlockStatus,
    /// Optional list of transactions included in the block.
    /// Initially None, filled only if `include_txs` was requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transactions: Option<Vec<TransactionResponse>>,
    /// Count of transactions in the block.
    pub transactions_count: u64, // Serialized as number (u64 fits JS number)
    /// Block reward in atomic units as numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub block_reward: u64,
    /// Sum of transaction fees in atomic units as numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub total_fees: u64,
    /// Total gas consumed by transactions in the block as numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub total_gas_spent: u64,
}

// --- Conversion Implementations ---

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

impl From<ledger::Label> for BlockStatus {
    fn from(label: ledger::Label) -> Self {
        match label {
            ledger::Label::Final(_) => BlockStatus::Final,
            _ => BlockStatus::Provisional, /* Treat Accepted, Attested,
                                            * Confirmed as Provisional for
                                            * API */
        }
    }
}

impl From<ledger::Block> for Block {
    fn from(b: ledger::Block) -> Self {
        let header_model = BlockHeader::from(b.header().clone());
        let status = BlockStatus::Provisional; // Placeholder - must be determined from Label

        // Placeholder values for reward/fees/gas - these need
        // calculation/fetching
        let block_reward = 0u64; // Placeholder
        let total_fees = 0u64; // Placeholder
        let total_gas_spent = 0u64; // Placeholder

        Self {
            header: header_model,
            status, // Placeholder
            transactions: None,
            transactions_count: b.txs().len() as u64,
            block_reward,    // Placeholder
            total_fees,      // Placeholder
            total_gas_spent, // Placeholder
        }
    }
}
