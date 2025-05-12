// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::jsonrpc::infrastructure::state::AppState;
use crate::jsonrpc::model;

use jsonrpsee::core::async_trait;
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::types::ErrorObjectOwned;

use std::sync::Arc;

/// RPC trait for block-related methods.
#[rpc(server)]
pub trait BlockRpc {
    /// Returns detailed information about a block identified by its hash.
    ///
    /// # Arguments
    /// * `block_hash` - The block hash as a hex-encoded 32-byte string
    /// * `include_txs` - Whether to include transaction details in the
    ///   response. Default is false.
    ///
    /// # Returns
    /// A `Result` containing the block information or an error.
    ///
    /// # Error Codes
    ///
    /// | Code | Message | Description |
    /// |------|---------|-------------|
    /// | -32602 | Invalid params | Invalid hash format (not 64 hex chars) |
    /// | -32603 | Internal error | Database or internal error |
    /// | -32000 | Block not found | Block with specified hash doesn't exist |
    #[method(name = "getBlockByHash")]
    async fn get_block_by_hash(
        &self,
        hash: String,
        include_txs: bool,
    ) -> Result<model::block::Block, ErrorObjectOwned>;
}

/// Implementation of the `BlockRpcServer` trait.
#[derive(Clone)]
pub struct BlockRpcImpl {
    app_state: Arc<AppState>,
}

impl BlockRpcImpl {
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self { app_state }
    }
}

#[async_trait]
impl BlockRpcServer for BlockRpcImpl {
    async fn get_block_by_hash(
        &self,
        hash: String,
        include_txs: bool,
    ) -> Result<model::block::Block, ErrorObjectOwned> {
        // 1. Validate the hash format
        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ErrorObjectOwned::owned(
                -32602,
                "Invalid params",
                Some("Invalid hash format".to_string()),
            ));
        }

        let block = self
            .app_state
            .get_block_by_hash(hash.as_str(), include_txs)
            .await
            .map_err(|e| {
                ErrorObjectOwned::owned(
                    -32603,
                    "Internal error",
                    Some(e.to_string()),
                )
            })?;

        match block {
            Some(block) => Ok(block),
            None => Err(ErrorObjectOwned::owned(
                -32000,
                "Block not found",
                Some(format!("Block with hash {} not found", hash)),
            )),
        }
    }
}
