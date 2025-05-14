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

const MAX_BLOCKS_TO_RETRIEVE: u64 = 100;

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

    /// Returns a sequence of blocks within the specified height range.
    ///
    /// # Arguments
    /// * `start_height` - Starting block height.
    /// * `end_height` - Ending block height (inclusive).
    /// * `include_txs` - Optional argument. If true, includes transaction
    ///   details. Defaults to false.
    ///
    /// # Returns
    /// A `Result` containing the an array of block information or an error.
    ///
    /// # Error Codes
    ///
    /// | Code | Message | Description |
    /// |------|---------|-------------|
    /// | -32602 | Invalid params | Invalid height range (negative, too large, or end < start) |
    /// | -32603 | Internal error | Database or internal error |
    #[method(name = "getBlocksRange")]
    async fn get_blocks_range(
        &self,
        start_height: u64,
        end_height: u64,
        include_txs: Option<bool>,
    ) -> Result<Vec<model::block::Block>, ErrorObjectOwned>;

    /// Returns the specified number of most recent blocks.
    ///
    /// # Arguments
    /// * `count` - Number of latest blocks to return.
    /// * `include_txs` - Optional argument. If true, includes transaction
    ///   details. Defaults to false.
    ///
    /// # Returns
    /// A `Result` containing the an array of block information ordered from
    /// newest to oldest starting from the latest block or an error.
    ///
    /// # Error Codes
    ///
    /// | Code | Message | Description |
    /// |------|---------|-------------|
    /// | -32602 | Invalid params | Invalid count (zero or too large) |
    /// | -32603 | Internal error | Database or internal error |
    #[method(name = "getLatestBlocks")]
    async fn get_latest_blocks(
        &self,
        count: u64,
        include_txs: Option<bool>,
    ) -> Result<Vec<model::block::Block>, ErrorObjectOwned>;

    /// Returns the total number of blocks in the blockchain.
    ///
    /// # Arguments
    /// * `finalized_only` - Optional argument. If true, returns only finalized
    ///   blocks count. Defaults to false
    ///
    /// # Returns
    /// A `Result` containing the total number of blocks as numeric string.
    ///
    /// # Error Codes
    ///
    /// | Code | Message | Description |
    /// |------|---------|-------------|
    /// | -32603 | Internal error | Database or internal error |
    #[method(name = "getBlocksCount")]
    async fn get_blocks_count(
        &self,
        finalized_only: Option<bool>,
    ) -> Result<String, ErrorObjectOwned>;

    /// Returns both the latest candidate block with transaction data and the
    /// latest finalized block.
    ///
    /// # Arguments
    /// * `include_txs` - Optional argument. If true, includes transaction
    ///   details in the finalized block. Defaults to false
    ///
    /// # Returns
    /// A `Result` containing the block pair object where the `latest` block is
    /// the latest candidate block and the `finalized` block is the latest
    /// finalized block or error if the fetching of either block fails.
    ///
    /// # Error Codes
    ///
    /// | Code | Message | Description |
    /// |------|---------|-------------|
    /// | -32603 | Internal error | Database or internal error |
    #[method(name = "getBlockPair")]
    async fn get_block_pair(
        &self,
        include_txs: Option<bool>,
    ) -> Result<model::block::BlockPair, ErrorObjectOwned>;

    /// Returns the finalization status of a block identified by its height.
    ///
    /// # Arguments
    /// * `block_height` - The height of the block to check the finalization
    ///   status.
    ///
    /// # Returns
    /// A `Result` containing the block finalization status or the error if the
    /// fetching of the block status fails or block not found.
    ///
    /// # Error Codes
    ///
    /// | Code | Message | Description |
    /// |------|---------|-------------|
    /// | -32602 | Invalid params | Invalid height (non-positive or too large) |
    /// | -32603 | Internal error | Database or internal error |
    /// | -32000 | Block not found | Block with specified height doesn't exist |
    #[method(name = "getBlockStatus")]
    async fn get_block_status(
        &self,
        block_height: u64,
    ) -> Result<model::block::BlockStatusResponse, ErrorObjectOwned>;

    /// Returns events emitted during block execution for a block identified by
    /// its hash.
    ///
    /// # Arguments
    /// * `block_hash` - TThe block hash as a hex-encoded 32-byte string.
    ///
    /// # Returns
    /// A `Result` containing an array of block events or the error if the
    /// fetching of the block events fails, or block not found, or block has no
    /// associated events.
    ///
    /// # Error Codes
    ///
    /// | Code | Message | Description |
    /// |------|---------|-------------|
    /// | -32602 | Invalid params | Invalid hash format (not 64 hex chars) |
    /// | -32603 | Internal error | Database or internal error |
    /// | -32000 | Not found | Block with specified hash doesn't exist or has no associated events |
    #[method(name = "getBlockEventsByHash")]
    async fn get_block_events_by_hash(
        &self,
        block_hash: String,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ErrorObjectOwned>;
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
                Some("Invalid height format (zero or too large)".to_string()),
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

    async fn get_blocks_range(
        &self,
        start_height: u64,
        end_height: u64,
        include_txs: Option<bool>,
    ) -> Result<Vec<model::block::Block>, ErrorObjectOwned> {
        if start_height == 0
            || end_height == 0
            || start_height > u64::MAX
            || end_height > u64::MAX
        {
            return Err(ErrorObjectOwned::owned(
                -32602,
                "Invalid params",
                Some("Invalid height (zero or too large)".to_string()),
            ));
        }

        if start_height > end_height {
            return Err(ErrorObjectOwned::owned(
                -32602,
                "Invalid params",
                Some("Invalid height range (end < start)".to_string()),
            ));
        }

        if start_height == end_height {
            return Err(ErrorObjectOwned::owned(
                -32602,
                "Invalid params",
                Some("Invalid height range (start == end)".to_string()),
            ));
        }

        if end_height - start_height > MAX_BLOCKS_TO_RETRIEVE {
            return Err(ErrorObjectOwned::owned(
                -32602,
                "Invalid params",
                Some(format!(
                    "Invalid height range (range > {})",
                    MAX_BLOCKS_TO_RETRIEVE
                )),
            ));
        }

        let blocks = self
            .app_state
            .get_blocks_range(
                start_height,
                end_height,
                include_txs.unwrap_or(false),
            )
            .await
            .map_err(|e| {
                ErrorObjectOwned::owned(
                    -32603,
                    "Internal error",
                    Some(e.to_string()),
                )
            })?;

        Ok(blocks)
    }

    async fn get_latest_blocks(
        &self,
        count: u64,
        include_txs: Option<bool>,
    ) -> Result<Vec<model::block::Block>, ErrorObjectOwned> {
        if count == 0 || count > u64::MAX {
            return Err(ErrorObjectOwned::owned(
                -32602,
                "Invalid params",
                Some("Invalid count (zero or too large)".to_string()),
            ));
        }

        if count > MAX_BLOCKS_TO_RETRIEVE {
            return Err(ErrorObjectOwned::owned(
                -32602,
                "Invalid params",
                Some(format!(
                    "Invalid count (range > {})",
                    MAX_BLOCKS_TO_RETRIEVE
                )),
            ));
        }

        let blocks = self
            .app_state
            .get_latest_blocks(count, include_txs.unwrap_or(false))
            .await
            .map_err(|e| {
                ErrorObjectOwned::owned(
                    -32603,
                    "Internal error",
                    Some(e.to_string()),
                )
            })?;

        Ok(blocks)
    }

    async fn get_blocks_count(
        &self,
        finalized_only: Option<bool>,
    ) -> Result<String, ErrorObjectOwned> {
        let count = self
            .app_state
            .get_blocks_count(finalized_only.unwrap_or(false))
            .await
            .map_err(|e| {
                ErrorObjectOwned::owned(
                    -32603,
                    "Internal error",
                    Some(e.to_string()),
                )
            })?;

        Ok(count.to_string())
    }

    async fn get_block_pair(
        &self,
        include_txs: Option<bool>,
    ) -> Result<model::block::BlockPair, ErrorObjectOwned> {
        self.app_state
            .get_block_pair(include_txs.unwrap_or(false))
            .await
            .map_err(|e| {
                ErrorObjectOwned::owned(
                    -32603,
                    "Internal error",
                    Some(e.to_string()),
                )
            })
    }

    async fn get_block_status(
        &self,
        block_height: u64,
    ) -> Result<model::block::BlockStatusResponse, ErrorObjectOwned> {
        if block_height == 0 || block_height > u64::MAX {
            return Err(ErrorObjectOwned::owned(
                -32602,
                "Invalid params",
                Some("Invalid height (non-positive or too large)".to_string()),
            ));
        }

        let status = self
            .app_state
            .get_block_status_by_height(block_height)
            .await
            .map_err(|e| {
                ErrorObjectOwned::owned(
                    -32603,
                    "Internal error",
                    Some(e.to_string()),
                )
            })?;

        match status {
            Some(status) => Ok(model::block::BlockStatusResponse { status }),
            None => Err(ErrorObjectOwned::owned(
                -32602,
                "Block not found",
                Some("Block with specified height doesn't exist".to_string()),
            )),
        }
    }

    async fn get_block_events_by_hash(
        &self,
        block_hash: String,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ErrorObjectOwned> {
        // 1. Validate the hash format
        if block_hash.len() != 64
            || !block_hash.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err(ErrorObjectOwned::owned(
                -32602,
                "Invalid params",
                Some("Invalid hash format".to_string()),
            ));
        }

        let events = self
            .app_state
            .get_block_events_by_hash(block_hash)
            .await
            .map_err(|e| {
                ErrorObjectOwned::owned(
                    -32603,
                    "Internal error",
                    Some(e.to_string()),
                )
            })?;

        if events.is_empty() {
            return Err(ErrorObjectOwned::owned(
                -32000,
                "Not found",
                Some("Block with specified hash doesn't exist or has no associated events".to_string()),
            ));
        }

        Ok(events)
    }
}
