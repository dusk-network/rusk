// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # JSON-RPC Chain Models
//!
//! Defines data structures representing high-level statistics and information
//! about the blockchain itself, intended for use in the JSON-RPC API.
//!
//! ## Key Structures:
//!
//! - [`ChainStats`]: Provides key statistics about the current state of the
//!   blockchain, such as height and tip hash.

use serde::{Deserialize, Serialize};

/// Represents overall statistics about the current state of the blockchain.
///
/// Provides a quick summary of the chain's progress and state.
///
/// # Examples
///
/// ```
/// use rusk::jsonrpc::model::chain::ChainStats;
///
/// let stats = ChainStats {
///     height: 123456,
///     tip_hash: "block_hash_hex".to_string(),
///     state_root_hash: "state_root_hex".to_string(),
/// };
///
/// // Typically obtained via DatabaseAdapter::get_chain_stats()
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainStats {
    /// The height (block number) of the most recent finalized block (chain
    /// tip).
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub height: u64,
    /// The 32-byte hash of the most recent finalized block (chain tip).
    /// Serialized as a 64-character hex string.
    pub tip_hash: String,
    /// The 32-byte Merkle root hash representing the entire blockchain state
    /// after the execution of the chain tip block.
    /// Serialized as a 64-character hex string.
    pub state_root_hash: String,
    // NOTE: Other potentially useful statistics like difficulty, total supply,
    // or network hash rate are not included here. These are either not readily
    // available through the current underlying database traits
    // (`node::database::Metadata`, `node::database::Ledger`) or would require
    // significant computation (like iterating history for supply) that is not
    // suitable for a simple statistics endpoint.
}
