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
use crate::jsonrpc::model;

use jsonrpsee::core::async_trait;
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::types::ErrorObjectOwned;

use std::sync::Arc;

/// RPC trait for Rusk informational methods.
#[rpc(server)]
pub trait RuskInfoRpc {
    /// General information about the node, including version, configuration,
    /// and network details.
    ///
    /// # Returns
    ///
    /// * `Ok(model::network::NodeInfo)` - A struct containing:
    ///   - `version`: The version of the node.
    ///   - `version_build`: The build number of the node.
    ///   - `network_id`: The ID of the chain the node is running on.
    ///   - `public_address`: The public address of the node.
    ///   - `bootstrap_nodes`: The vector of strings containing the list of
    ///     known bootstrapping kadcast nodes.
    ///   - `vm_config`: The configuration of the virtual machine the node is
    ///     running on.
    /// * `Err(ErrorObjectOwned)` - If fetching information fails.
    #[method(name = "getNodeInfo")]
    async fn get_node_info(
        &self,
    ) -> Result<model::network::NodeInfo, ErrorObjectOwned>;
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
    async fn get_node_info(
        &self,
    ) -> Result<model::network::NodeInfo, ErrorObjectOwned> {
        self.app_state.get_node_info().await.map_err(|e| {
            ErrorObjectOwned::owned(
                -32603,
                "Internal error",
                Some(e.to_string()),
            )
        })
    }
}
