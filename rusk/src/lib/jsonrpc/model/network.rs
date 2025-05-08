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

use crate::jsonrpc::model;

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
/// // Typically obtained via NetworkAdapter::get_alive_peers_count()
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeersMetrics {
    /// The total number of peers the node is currently connected to (or aware
    /// of as being alive, depending on the underlying implementation).
    pub peer_count: u32,
}

/// Represents the geographical location information for a network peer.
/// Derived from querying an external IP geolocation service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeerLocation {
    /// Latitude of the peer's location.
    pub lat: Option<f64>,
    /// Longitude of the peer's location.
    pub lon: Option<f64>,
    /// City name associated with the peer's IP address.
    pub city: Option<String>,
    /// Country name associated with the peer's IP address.
    pub country: Option<String>,
    /// Country code (e.g., "US", "NL") associated with the peer's IP address.
    pub country_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Represents the current status of the network as observed by the node.
///
/// This structure provides key metrics and identifiers that describe the
/// node's connectivity and its position within the blockchain network.
pub struct NetworkStatus {
    /// The total number of peers currently connected to the node.
    ///
    /// This value reflects the count of peers that are actively communicating
    /// with the node and are considered "alive" based on the network's
    /// implementation.
    pub connected_peers: u32,

    /// The height of the latest block in the blockchain as observed by the
    /// node.
    ///
    /// This value represents the number of blocks in the chain, starting from
    /// the genesis block (height 0) up to the most recent block. It is an
    /// indicator of the node's synchronization status with the network.
    pub current_block_height: u64,

    /// The unique identifier of the blockchain network the node is connected
    /// to.
    ///
    /// This value is typically assigned by the virtual machine (VM) and is
    /// used to distinguish between different blockchain networks or
    /// environments, such as mainnet, testnet, or custom deployments.
    pub network_id: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// General information about the node, including version, configuration, and
/// network details.
pub struct NodeInfo {
    /// Semantic version of the node
    pub version: String,
    /// Build information of the node
    pub version_build: String,
    /// The unique identifier of the blockchain network the node is connected
    /// to.
    pub chain_id: u8,
    /// Public `SocketAddress` of the Peer. No domain name allowed
    ///
    /// This is the address where other peers can contact you.
    /// This address MUST be accessible from any peer of the network
    pub public_address: std::net::SocketAddr,
    /// List of known bootstrapping kadcast nodes.
    ///
    /// It accepts the same representation of `public_address` but with domain
    /// names allowed
    pub bootstrap_nodes: Vec<String>,
    /// Represents the relevant VM configuration settings exposed via JSON-RPC.
    ///
    /// This provides information about gas limits, deployment costs, and other
    /// parameters influencing transaction execution and block generation
    /// within the VM.
    pub vm_config: model::vm::VmConfig,
}
