// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # JSON-RPC Network Adapter
//!
//! This module provides an abstraction layer for interacting with the node's
//! underlying network components (like `Kadcast`). It defines the
//! [`NetworkAdapter`] trait, which specifies the required network operations
//! (e.g., broadcasting transactions, querying network info, managing peers)
//! needed by the JSON-RPC service.
//!
//! The primary implementation is [`RuskNetworkAdapter`], which wraps the actual
//! `node::Network` implementation (feature-gated behind `chain`). Using an
//! adapter decouples the JSON-RPC layer from the specific network
//! implementation details, improving testability (allowing mocks like
//! [`MockNetworkAdapter`] found in test utilities) and maintainability.
//!
//! Errors specific to network operations are defined in [`NetworkError`].
//!
//! For detailed adapter method comparisons vs. the legacy HTTP server, see:
//! [`Network Adapter Methods
//! Comparison`](../../../../docs/network_adapter_methods.md)

use crate::jsonrpc::infrastructure::error::NetworkError;
use async_trait::async_trait;
#[cfg(feature = "chain")]
use node::Network;
use node_data::ledger;
use node_data::message::payload::Inv;
use node_data::message::Message;
use node_data::Serializable;
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::jsonrpc::model;

/// Trait defining the interface for network operations needed by the JSON-RPC
/// service.
///
/// This trait abstracts the interaction with the underlying node's network
/// components, providing methods to broadcast transactions, query network
/// state, and interact with peers. Implementations of this trait wrap the
/// actual network client (like [`node::Network`](../../../../node/src/lib.rs)
/// or its specific implementation
/// [`node::network::Kadcast`](../../../../node/src/network.rs)).
#[async_trait]
pub trait NetworkAdapter: Send + Sync + fmt::Debug + 'static {
    /// Broadcasts a transaction to the network.
    ///
    /// Corresponds to the `node::Network::broadcast` functionality.
    ///
    /// # Required Method
    ///
    /// # Arguments
    ///
    /// * `tx_bytes` - The serialized transaction bytes to be broadcast.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the broadcast request was successfully initiated.
    /// * `Err(NetworkError)` - If the broadcast failed (e.g., network
    ///   unavailable, serialization issues, internal errors).
    async fn broadcast_transaction(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<(), NetworkError>;

    /// Retrieves general information about the network state.
    ///
    /// Corresponds to the `node::Network::get_info` functionality.
    ///
    /// # Required Method
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - A string containing network information (format
    ///   determined by the underlying implementation).
    /// * `Err(NetworkError)` - If querying the network information failed.
    async fn get_network_info(&self) -> Result<String, NetworkError>;

    /// Retrieves the public network address of this node.
    ///
    /// Corresponds to the `node::Network::public_addr` functionality.
    ///
    /// # Required Method
    ///
    /// # Returns
    ///
    /// * `Ok(SocketAddr)` - The public socket address of the node.
    /// * `Err(NetworkError)` - If the public address could not be determined.
    async fn get_public_address(&self) -> Result<SocketAddr, NetworkError>;

    /// Retrieves a list of currently alive peers known to the node.
    ///
    /// Corresponds to the underlying logic for retrieving alive nodes (e.g.,
    /// iterating through a peer list).
    ///
    /// # Required Method
    ///
    /// # Arguments
    ///
    /// * `max_peers` - The maximum number of peer addresses to return.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<SocketAddr>)` - A vector containing the socket addresses of
    ///   alive peers, up to `max_peers`.
    /// * `Err(NetworkError)` - If retrieving the peer list failed.
    async fn get_alive_peers(
        &self,
        max_peers: usize,
    ) -> Result<Vec<SocketAddr>, NetworkError>;

    /// Retrieves the count of currently alive peers known to the node.
    ///
    /// Corresponds to the `node::Network::alive_nodes_count` functionality.
    ///
    /// # Required Method
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The number of alive peers.
    /// * `Err(NetworkError)` - If counting the peers failed.
    async fn get_alive_peers_count(&self) -> Result<usize, NetworkError>;

    /// Floods an inventory message (`Inv`) across the network.
    ///
    /// Corresponds to the `node::Network::flood_request` functionality.
    ///
    /// # Required Method
    ///
    /// # Arguments
    ///
    /// * `inv` - The inventory message to flood.
    /// * `ttl_seconds` - Optional time-to-live for the flood request in
    ///   seconds.
    /// * `hops` - The number of hops the message should propagate.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the flood request was successfully initiated.
    /// * `Err(NetworkError)` - If initiating the flood request failed.
    async fn flood_request(
        &self,
        inv: Inv,
        ttl_seconds: Option<u64>,
        hops: u16,
    ) -> Result<(), NetworkError>;

    /// Retrieves metrics about the node's connected peers.
    ///
    /// Corresponds to the `node::Network::alive_nodes_count` functionality,
    /// wrapped in the `model::network::PeersMetrics` model.
    ///
    /// # Default Method
    ///
    /// This method has a default implementation that uses
    /// [`get_alive_peers_count`](NetworkAdapter::get_alive_peers_count).
    ///
    /// # Returns
    ///
    /// * `Ok(PeersMetrics)` - Metrics containing the peer count.
    /// * `Err(NetworkError)` - If retrieving the peer count failed.
    async fn get_peers_metrics(
        &self,
    ) -> Result<model::network::PeersMetrics, NetworkError> {
        let count = self.get_alive_peers_count().await?;
        // Convert usize to u32, handling potential overflow (though unlikely
        // for peer count)
        let peer_count = count.try_into().map_err(|_| {
            NetworkError::InternalError(
                "Peer count overflowed u32 capacity".to_string(),
            )
        })?;
        Ok(model::network::PeersMetrics { peer_count })
    }
}

// RuskNetworkAdapter implementation (requires 'chain' feature)

#[cfg(feature = "chain")]
pub struct RuskNetworkAdapter<N: Network> {
    /// Shared, thread-safe access to the network client.
    network_client: Arc<RwLock<N>>,
}

#[cfg(feature = "chain")]
impl<N: Network> RuskNetworkAdapter<N> {
    /// Creates a new `RuskNetworkAdapter`.
    ///
    /// # Arguments
    ///
    /// * `network_client` - An `Arc<RwLock<N>>` pointing to the node's network
    ///   component.
    pub fn new(network_client: Arc<RwLock<N>>) -> Self {
        Self { network_client }
    }
}

// Manual Debug implementation to avoid requiring N: Debug and potentially
// leaking sensitive info.
#[cfg(feature = "chain")]
impl<N: Network> fmt::Debug for RuskNetworkAdapter<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuskNetworkAdapter")
            .field("network_client", &"Arc<RwLock<N: Network>>")
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "chain")]
#[async_trait]
impl<N: Network> NetworkAdapter for RuskNetworkAdapter<N> {
    async fn broadcast_transaction(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<(), NetworkError> {
        // Deserialize the transaction bytes into a ledger::Transaction
        let tx = ledger::Transaction::read(&mut tx_bytes.as_slice()).map_err(
            |e| {
                NetworkError::QueryFailed(format!(
                    "Failed to deserialize transaction bytes: {}",
                    e
                ))
            },
        )?;

        // Wrap the transaction in the appropriate Message variant
        let message = Message::from(tx); // Use From<ledger::Transaction>

        let client = self.network_client.read().await;
        client
            .broadcast(&message)
            .await
            .map_err(|e| NetworkError::QueryFailed(e.to_string()))
    }

    async fn get_network_info(&self) -> Result<String, NetworkError> {
        let client = self.network_client.read().await;
        client
            .get_info()
            .map_err(|e| NetworkError::QueryFailed(e.to_string()))
    }

    async fn get_public_address(&self) -> Result<SocketAddr, NetworkError> {
        let client = self.network_client.read().await;
        Ok(*client.public_addr())
    }

    async fn get_alive_peers(
        &self,
        max_peers: usize,
    ) -> Result<Vec<SocketAddr>, NetworkError> {
        // Retrieve the alive nodes from the underlying Kadcast implementation
        let client = self.network_client.read().await;
        let peers = client.alive_nodes(max_peers).await;
        Ok(peers)
    }

    async fn get_alive_peers_count(&self) -> Result<usize, NetworkError> {
        let client = self.network_client.read().await;
        Ok(client.alive_nodes_count().await)
    }

    async fn flood_request(
        &self,
        inv: Inv,
        ttl_seconds: Option<u64>,
        hops: u16,
    ) -> Result<(), NetworkError> {
        let client = self.network_client.read().await;
        client
            .flood_request(&inv, ttl_seconds, hops)
            .await
            .map_err(|e| NetworkError::QueryFailed(e.to_string()))
    }
}
