// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # JSON-RPC Mempool Models
//!
//! Defines data structures representing information about the node's
//! transaction mempool, intended for use in the JSON-RPC API.
//!
//! ## Key Structures:
//!
//! - [`MempoolInfo`]: Provides summary statistics about the mempool, such as
//!   transaction count and fee range.

use serde::{Deserialize, Serialize};

/// Represents summary statistics about the current state of the transaction
/// mempool.
///
/// Provides insights into the pending transaction pool without needing to fetch
/// all individual transactions.
///
/// # Examples
///
/// ```
/// use rusk::jsonrpc::model::mempool::MempoolInfo;
///
/// let info = MempoolInfo {
///     count: 150,
///     max_fee: Some(10000), // Highest gas price in mempool
///     min_fee: Some(50),    // Lowest gas price in mempool
/// };
///
/// // Typically obtained via DatabaseAdapter::get_mempool_info()
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MempoolInfo {
    /// The total number of transactions currently pending in the mempool.
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub count: u64,
    /// The gas price of the transaction with the highest fee currently in the
    /// mempool (in atomic units).
    /// `None` if the mempool is empty.
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::opt_u64_to_string", default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee: Option<u64>,
    /// The gas price of the transaction with the lowest fee currently in the
    /// mempool (in atomic units).
    /// `None` if the mempool is empty.
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::opt_u64_to_string", default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_fee: Option<u64>,
    // NOTE: Total size (in bytes) of all transactions in the mempool is
    // omitted. Calculating this would require fetching and summing the size
    // of all transactions, which can be computationally expensive for a simple
    // info endpoint. Clients needing size information can retrieve all
    // mempool transactions (e.g., via `getMempoolTransactions`) and
    // calculate the size or average fees themselves.
}
