// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk::jsonrpc::infrastructure::error::NetworkError;
use rusk::jsonrpc::infrastructure::network::NetworkAdapter;
use rusk::jsonrpc::model;

use std::fmt::Debug;
use std::net::SocketAddr;

/// Mock implementation of `NetworkAdapter` for testing.
#[derive(Debug, Clone, Default)]
pub struct MockNetworkAdapter {
    /// Force an error on all method calls if Some.
    pub force_error: Option<NetworkError>,
    /// Predefined network info string.
    pub bootstrapping_nodes: Option<Vec<String>>,
    /// Predefined public address.
    pub public_address: Option<SocketAddr>,
    /// Predefined list of alive peers.
    pub alive_peers: Option<Vec<SocketAddr>>,
    /// Predefined count of alive peers.
    pub alive_peers_count: Option<usize>,
    /// Predefined list of peer locations.
    pub peer_locations: Option<Vec<model::network::PeerLocation>>,
}

#[async_trait::async_trait]
impl NetworkAdapter for MockNetworkAdapter {
    async fn broadcast_transaction(
        &self,
        _tx_bytes: Vec<u8>,
    ) -> Result<(), NetworkError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        // Simple Ok for mock
        Ok(())
    }

    async fn get_bootstrapping_nodes(
        &self,
    ) -> Result<Vec<String>, NetworkError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        Ok(self.bootstrapping_nodes.clone().unwrap_or_else(|| {
            vec!["MockNetwork_1".to_string(), "MockNetwork_2".to_string()]
        }))
    }

    async fn get_public_address(&self) -> Result<SocketAddr, NetworkError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        Ok(self
            .public_address
            .unwrap_or_else(|| ([127, 0, 0, 1], 9000).into()))
    }

    async fn get_alive_peers(
        &self,
        _max_peers: usize,
    ) -> Result<Vec<SocketAddr>, NetworkError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        Ok(self.alive_peers.clone().unwrap_or_default())
    }

    async fn get_alive_peers_count(&self) -> Result<usize, NetworkError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        Ok(self.alive_peers_count.unwrap_or_default())
    }

    async fn flood_request(
        &self,
        _inv: node_data::message::payload::Inv,
        _ttl_seconds: Option<u64>,
        _hops: u16,
    ) -> Result<(), NetworkError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        // Simple Ok for mock
        Ok(())
    }

    async fn get_network_peers_location(
        &self,
    ) -> Result<Vec<model::network::PeerLocation>, NetworkError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        Ok(self.peer_locations.clone().unwrap_or_default())
    }
}
