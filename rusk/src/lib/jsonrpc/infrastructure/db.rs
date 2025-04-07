// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # Database Adapter Infrastructure
//!
//! This module defines the `DatabaseAdapter` trait, which serves as an
//! abstraction layer for interacting with the underlying Rusk database or state
//! management system. It allows the JSON-RPC service layer to remain decoupled
//! from the specific database implementation.
//!
//! ## Design
//!
//! - The `DatabaseAdapter` trait uses `async_trait` to support asynchronous
//!   database operations.
//! - It requires implementors to be `Send + Sync + Debug` for safe sharing
//!   across threads.
//! - Methods will be added to this trait to cover common data retrieval needs
//!   of the JSON-RPC services (e.g., fetching blocks, transactions, state
//!   information).
//! - A concrete implementation (e.g., `RuskDbAdapter`) will wrap the actual
//!   Rusk database client or state handle.
//!
//! ## Usage
//!
//! An instance implementing `DatabaseAdapter` (likely wrapped in an `Arc`) will
//! be stored in the `AppState` and made available to JSON-RPC method handlers.

use crate::jsonrpc::infrastructure::error::DbError;
use async_trait::async_trait;
use std::fmt::Debug;

// --- Placeholder Types ---
// These will be replaced with actual types from `node-data` or similar crates
// once the methods are implemented.

/// Placeholder for the data structure representing a block.
#[derive(Debug, Clone)]
pub struct BlockData {
    pub height: u64,
    pub hash: String,
    // Add other relevant block fields later
}

// --- End Placeholder Types ---

/// Trait defining the interface for accessing Rusk's backend data.
///
/// This abstraction allows the JSON-RPC services to interact with the database
/// without being tied to a specific implementation (like RocksDB). Implementors
/// must be thread-safe (`Send + Sync`).
#[async_trait]
pub trait DatabaseAdapter: Send + Sync + Debug {
    /// Retrieves block data by its height.
    ///
    /// # Arguments
    ///
    /// * `height` - The height of the block to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(BlockData))` if the block is found.
    /// * `Ok(None)` if no block exists at the given height.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_block_by_height(
        &self,
        height: u64,
    ) -> Result<Option<BlockData>, DbError>;

    // TODO: Add more methods as needed by RPC services, e.g.:
    // async fn get_block_by_hash(&self, hash: &str) ->
    // Result<Option<BlockData>, DbError>;
    // async fn get_transaction_by_hash(&self, hash: &str) ->
    // Result<Option<TransactionData>, DbError>;
    // async fn get_latest_block_height(&self) -> Result<u64, DbError>;
    // async fn get_mempool_transactions(&self) -> Result<Vec<TransactionData>,
    // DbError>; ... other necessary database interactions
}
