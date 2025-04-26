// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # JSON-RPC Network Models
//!
//! Defines data structures representing information about the node's network
//! connections and peers, intended for use in the JSON-RPC API.
//!
//! ## Key Structures:
//!
//! - [`PeersMetrics`]: Provides basic metrics about connected peers, such as
//!   the total count.

use serde::{Deserialize, Serialize};

/// Represents basic metrics related to the node's network peers.
///
/// Provides a high-level overview of the node's connectivity.
///
/// # Examples
///
/// ```
/// use rusk::jsonrpc::model::network::PeersMetrics;
///
/// let metrics = PeersMetrics {
///     peer_count: 55,
/// };
///
/// // Typically obtained via DatabaseAdapter::get_peers_metrics() or
/// // NetworkAdapter::get_alive_peers_count()
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeersMetrics {
    /// The total number of peers the node is currently connected to (or aware
    /// of as being alive, depending on the underlying implementation).
    pub peer_count: u32,
}
