// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # JSON-RPC Gas Models
//!
//! Defines data structures related to gas price information, intended for use
//! in the JSON-RPC API.
//!
//! ## Key Structures:
//!
//! - [`GasPriceStats`]: Provides statistics calculated from recent mempool
//!   transaction fees.

use serde::{Deserialize, Serialize};

/// Represents statistics about gas prices observed in the mempool.
///
/// These statistics are typically calculated based on a sample of recent
/// transactions sorted by fee.
///
/// # Examples
///
/// ```
/// use rusk::jsonrpc::model::gas::GasPriceStats;
///
/// let stats = GasPriceStats {
///     average: 1500, // Mean gas price
///     max: 5000,     // Highest gas price in sample
///     median: 1200,  // Median gas price in sample
///     min: 100,      // Lowest gas price in sample
/// };
///
/// // Typically obtained via DatabaseAdapter::get_gas_price()
/// ```
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct GasPriceStats {
    /// The average (mean) gas price calculated from the sample (in atomic
    /// units).
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub average: u64,
    /// The highest gas price observed in the sample (in atomic units).
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub max: u64,
    /// The median gas price observed in the sample (in atomic units).
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub median: u64,
    /// The lowest gas price observed in the sample (in atomic units).
    /// Serialized as a numeric string.
    #[serde(with = "super::serde_helper::u64_to_string")]
    pub min: u64,
}
