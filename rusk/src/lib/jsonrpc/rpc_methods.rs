// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # Rusk JSON-RPC Method Definitions
//!
//! This module defines the actual JSON-RPC methods exposed by the Rusk node's
//! RPC server.
//!
//! It utilizes the `jsonrpsee` crate's procedural macros (`#[rpc(server)]`)
//! to define RPC traits and the `#[async_trait]` macro for implementing these
//! traits asynchronously.
//!
//! Each RPC method implementation typically takes an `Arc<AppState>` to access
//! shared node resources like configuration and infrastructure adapters
//! (database, network, VM).
//!
//! Currently includes:
//! - `RuskInfoRpc`: Provides basic node information (version, chain ID, etc.).

use crate::jsonrpc::infrastructure::state::AppState;
use jsonrpsee::core::async_trait;
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Represents basic information about the Rusk node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Rusk node version.
    pub version: String,
    /// Configured network chain ID.
    #[serde(rename = "chainId")]
    pub chain_id: u8,
    /// Bind address for the JSON-RPC HTTP/S server.
    #[serde(rename = "jsonrpcHttpAddress")]
    pub jsonrpc_http_address: String,
    // Add more fields as needed, e.g., network ID, uptime, sync status etc.
}

/// RPC trait for Rusk informational methods.
#[rpc(server, namespace = "rusk")]
pub trait RuskInfoRpc {
    /// Retrieves basic information about the running Rusk node.
    ///
    /// # Returns
    ///
    /// * `Ok(NodeInfo)` - Structure containing node version, chain ID, etc.
    /// * `Err(ErrorObjectOwned)` - If fetching information fails.
    #[method(name = "getNodeInfo")]
    async fn get_node_info(&self) -> Result<NodeInfo, ErrorObjectOwned>;
}

/// Implementation of the `RuskInfoRpcServer` trait.
#[derive(Clone)]
pub struct RuskInfoRpcImpl {
    /// Shared application state containing config and adapters.
    app_state: Arc<AppState>,
}

impl RuskInfoRpcImpl {
    /// Creates a new instance of the RPC implementation.
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self { app_state }
    }
}

#[async_trait]
impl RuskInfoRpcServer for RuskInfoRpcImpl {
    /// Implements the `getNodeInfo` RPC method.
    ///
    /// Fetches data from the `AppState` (primarily the config).
    /// Currently retrieves version, chain ID (via VM adapter), and HTTP bind
    /// address.
    async fn get_node_info(&self) -> Result<NodeInfo, ErrorObjectOwned> {
        // TODO: Replace crate::VERSION with a more dynamic way if needed.
        let version = crate::VERSION.to_string();
        let jsonrpc_http_address =
            self.app_state.config().http.bind_address.to_string();

        // Attempt to get chain ID from VM adapter and `chain` feature enabled
        #[cfg(feature = "chain")]
        let chain_id_result = self.app_state.get_chain_id().await; // Use the public AppState method

        // Handle chain_id result based on feature flag and potential errors
        let chain_id = {
            #[cfg(feature = "chain")]
            {
                match chain_id_result {
                    Ok(id) => id,
                    Err(e) => {
                        // Log the error and return a default or error
                        // indicator? Returning a
                        // specific error might be better.
                        tracing::error!(error = %e, "Failed to get chain ID from VM adapter");
                        // For now, return a placeholder/default. Consider
                        // specific error later.
                        0 // Default/placeholder
                    }
                }
            }
            #[cfg(not(feature = "chain"))]
            {
                0 // Default if chain feature is not enabled
            }
        };

        Ok(NodeInfo {
            version,
            chain_id,
            jsonrpc_http_address,
        })
    }
}
