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
    /// * `include_txs` - Optional argument. If true, includes transaction
    ///   details. Defaults to false.
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
        include_txs: Option<bool>,
    ) -> Result<model::block::Block, ErrorObjectOwned>;

    /// Returns detailed information about a block at the specified height.
    ///
    /// # Arguments
    /// * `height` - The block height as a u64
    /// * `include_txs` - Optional argument. If true, includes transaction
    ///   details. Defaults to false.
    ///
    /// # Returns
    /// A `Result` containing the block information or an error.
    ///
    /// # Error Codes
    ///
    /// | Code | Message | Description |
    /// |------|---------|-------------|
    /// | -32602 | Invalid params | Invalid height format (negative or too large) |
    /// | -32603 | Internal error | Database or internal error |
    /// | -32000 | Block not found | Block with specified height doesn't exist |
    #[method(name = "getBlockByHeight")]
    async fn get_block_by_height(
        &self,
        height: u64,
        include_txs: Option<bool>,
    ) -> Result<Option<model::block::Block>, ErrorObjectOwned>;

    /// Returns information about the most recent block.
    ///
    /// # Arguments
    /// * `include_txs` - Optional argument. If true, includes transaction
    ///   details. Defaults to false.
    ///
    /// # Returns
    /// A `Result` containing the block information or an error.
    ///
    /// # Error Codes
    ///
    /// | Code | Message | Description |
    /// |------|---------|-------------|
    /// | -32603 | Internal error | Database or internal error |
    #[method(name = "getLatestBlock")]
    async fn get_latest_block(
        &self,
        include_txs: Option<bool>,
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
        include_txs: Option<bool>,
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
            .get_block_by_hash(hash.as_str(), include_txs.unwrap_or(false))
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

    async fn get_block_by_height(
        &self,
        height: u64,
        include_txs: Option<bool>,
    ) -> Result<Option<model::block::Block>, ErrorObjectOwned> {
        // 1. Validate the height format
        if height == 0 || height > u64::MAX {
            return Err(ErrorObjectOwned::owned(
                -32602,
                "Invalid params",
                Some(
                    "Invalid height format (negative or too large)".to_string(),
                ),
            ));
        }

        let block = self
            .app_state
            .get_block_by_height(height, include_txs.unwrap_or(false))
            .await
            .map_err(|e| {
                ErrorObjectOwned::owned(
                    -32603,
                    "Internal error",
                    Some(e.to_string()),
                )
            })?;

        match block {
            Some(block) => Ok(Some(block)),
            None => Err(ErrorObjectOwned::owned(
                -32000,
                "Block not found",
                Some(format!("Block with height {} not found", height)),
            )),
        }
    }

    async fn get_latest_block(
        &self,
        include_txs: Option<bool>,
    ) -> Result<model::block::Block, ErrorObjectOwned> {
        let block = self
            .app_state
            .get_latest_block(include_txs.unwrap_or(false))
            .await
            .map_err(|e| {
                ErrorObjectOwned::owned(
                    -32603,
                    "Internal error",
                    Some(e.to_string()),
                )
            })?;

        Ok(block)
    }
}
