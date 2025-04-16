// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # Database Adapter Infrastructure
//!
//! This module defines the [`DatabaseAdapter`] trait,
//! along with its concrete implementation ([`RuskDbAdapter`]) for interacting
//! with the Rusk node's underlying data sources.
//!
//! ## Purpose and Design Rationale
//!
//! The primary goal of this module is to **decouple** the JSON-RPC service
//! layer from the specific database (`node::database::rocksdb::Backend`) used
//! by the Rusk node. Instead of directly using this concrete type within RPC
//! handlers, services interact with the data sources through the
//! `DatabaseAdapter` trait.
//!
//! This abstraction provides several key advantages:
//!
//! 1. **Testability:** RPC handlers and services can be easily unit-tested by
//!    providing mock implementations (like `MockDbAdapter` and
//!    `MockArchiveAdapter` found in `rusk/tests/jsonrpc/utils.rs`). This avoids
//!    the need for running database or archive instances during most tests,
//!    making them faster and more reliable.
//! 2. **Flexibility & Maintainability:** If the underlying components or their
//!    APIs change, only the concrete adapter implementations need updating. The
//!    JSON-RPC services remain unaffected as long as the adapter trait
//!    contracts are maintained.
//! 3. **Clear Interface:** The adapter traits define the *specific* data access
//!    operations required by the JSON-RPC layer, providing focused APIs and
//!    preventing accidental coupling to unnecessary details of the underlying
//!    components.
//! 4. **Encapsulation:** The concrete adapters encapsulate component-specific
//!    logic (e.g., locking, error mapping, blocking task execution), keeping
//!    RPC handler logic cleaner.
//!
//! ## Usage
//!
//! `Arc`-wrapped instances implementing `DatabaseAdapter` (either concrete
//! implementations like [`RuskDbAdapter`] for production or mocks for testing)
//! are stored within the
//! [`AppState`](crate::jsonrpc::infrastructure::state::AppState).
//! RPC method handlers access the data sources via these trait objects.

use crate::jsonrpc::infrastructure::error::DbError;
use crate::jsonrpc::model;
use async_trait::async_trait;
use hex;
use node::database::rocksdb::MD_HASH_KEY;
use node::database::{Ledger, Metadata, DB};
use node_data::ledger::BlockWithLabel;
use std::fmt::Debug;
use std::sync::Arc;

// --- DatabaseAdapter Trait Definitions ---

/// Defines the interface for accessing the live blockchain state and data.
///
/// This trait abstracts the underlying database implementation (e.g., RocksDB
/// via `node::database::rocksdb::Backend`) used by the Rusk node. It allows
/// JSON-RPC services to query the current state of the chain, such as blocks,
/// headers, transactions, and the chain tip, without direct coupling to the
/// database specifics.
///
/// Implementations must be thread-safe (`Send + Sync + 'static`) and provide
/// asynchronous methods for data retrieval.
///
/// # Design
///
/// Using a trait here enables:
/// - **Testability:** Mock implementations (`MockDbAdapter`) can be used in
///   tests.
/// - **Flexibility:** The underlying database can be changed with minimal
///   impact on RPC handlers if the trait contract is maintained.
/// - **Encapsulation:** Hides database-specific logic like transaction
///   management and data serialization/deserialization.
///
/// # Errors
///
/// Methods typically return `Result<_, DbError>` to indicate potential issues
/// like database connection problems, data not found (e.g., block at a given
/// height/hash doesn't exist), or data deserialization errors.
#[async_trait]
pub trait DatabaseAdapter: Send + Sync + Debug + 'static {
    /// Retrieves a full block by its 32-byte hash.
    ///
    /// This corresponds to querying the `Ledger` trait's `block` or
    /// `light_block` method on the underlying database backend.
    ///
    /// # Arguments
    ///
    /// * `block_hash_hex`: A string containing the 64-character hexadecimal
    ///   representation of the block hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(rusk::jsonrpc::model::block::Block))` if a block with the
    ///   specified hash exists in the live blockchain state.
    /// * `Ok(None)` if no block matches the given hash.
    /// * `Err(rusk::jsonrpc::infrastructure::error::DbError)` if the input hash
    ///   is invalid hex, has incorrect length, or a database access error
    ///   occurs.
    ///
    /// # Example (Conceptual)
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use async_trait::async_trait;
    /// use std::fmt::Debug;
    /// use rusk::jsonrpc::infrastructure::db::DatabaseAdapter;
    /// use rusk::jsonrpc::infrastructure::error::DbError;
    /// use rusk::jsonrpc::model::block::Block;
    ///
    /// #[derive(Debug)]
    /// struct MyDbAdapter;
    ///
    /// #[async_trait]
    /// impl DatabaseAdapter for MyDbAdapter {
    ///   async fn get_block_by_hash(&self, _hash: &str) -> Result<Option<Block>, DbError> {
    ///     Ok(None)
    ///   }
    ///   async fn get_block_by_height(&self, _h: u64) -> Result<Option<Block>, DbError> {
    ///     Ok(None)
    ///   }
    ///   async fn get_latest_block(&self) -> Result<Block, DbError> {
    ///     unimplemented!()
    ///   }
    /// }
    ///
    /// async fn run(db_adapter: Arc<dyn DatabaseAdapter>) -> Result<(), DbError> {
    ///     // A valid block hash hex string
    ///     let hash = "7f42e39b74c3d85b21586837d7fee0b464981c935d271b5852684407b0d1624a";
    ///
    ///     if let Some(block) = db_adapter.get_block_by_hash(hash).await? {
    ///         println!("Found block at height {}", block.header.height);
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    async fn get_block_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<model::block::Block>, DbError>;

    /// Retrieves a full block by its height (block number).
    ///
    /// This corresponds to querying the `Ledger` trait's `block_by_height`
    /// method.
    ///
    /// # Arguments
    ///
    /// * `height`: The block height (`u64`) to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(rusk::jsonrpc::model::block::Block))` if a block exists at
    ///   the specified height in the live blockchain state.
    /// * `Ok(None)` if no block is found at that height (e.g., height is beyond
    ///   the current chain tip).
    /// * `Err(rusk::jsonrpc::infrastructure::error::DbError)` if a database
    ///   error occurs during retrieval.
    async fn get_block_by_height(
        &self,
        height: u64,
    ) -> Result<Option<model::block::Block>, DbError>;

    /// Retrieves the most recent block accepted and finalized by the node (the
    /// current chain tip) from the live state database.
    ///
    /// This involves reading metadata (like the tip hash using `MD_HASH_KEY`)
    /// and then fetching the corresponding block data.
    ///
    /// # Returns
    ///
    /// * `Ok(rusk::jsonrpc::model::block::Block)` containing the latest block
    ///   data.
    /// * `Err(rusk::jsonrpc::infrastructure::error::DbError)` if the latest
    ///   block cannot be determined or retrieved (e.g., database error, node
    ///   not yet synced, metadata missing).
    async fn get_latest_block(&self) -> Result<model::block::Block, DbError>;
}

// --- Concrete DatabaseAdapter Implementations ---

/// Concrete implementation of [`DatabaseAdapter`] that wraps the Rusk node's
/// live blockchain state database (`node::database::rocksdb::Backend`).
///
/// This adapter provides access to the current state of the blockchain, such as
/// blocks and chain metadata, by interacting with the underlying database via
/// the `DB` and `Ledger` traits.
///
/// It requires the `chain` feature flag to be enabled.
///
/// ## Thread Safety and Blocking
///
/// The underlying database backend (`Backend`) is wrapped in an
/// `Arc<tokio::sync::RwLock<...>>` to allow shared access across async tasks.
/// Database operations that might block are executed using
/// `tokio::task::spawn_blocking` to avoid stalling the async runtime.
#[cfg(feature = "chain")]
#[derive(Clone)]
pub struct RuskDbAdapter {
    db_client: Arc<tokio::sync::RwLock<node::database::rocksdb::Backend>>,
}

#[cfg(feature = "chain")]
impl std::fmt::Debug for RuskDbAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuskDbAdapter")
            .field("db_client", &"<Rusk Database Client>")
            .finish_non_exhaustive() // Hide internal details
    }
}

#[cfg(feature = "chain")]
impl RuskDbAdapter {
    /// Creates a new `RuskDbAdapter` instance.
    ///
    /// # Arguments
    ///
    /// * `db_client`: An `Arc` wrapped `RwLock` around the node's
    ///   `node::database::rocksdb::Backend` instance.
    ///
    /// # Returns
    ///
    /// A new adapter ready to interact with the live database state.
    pub fn new(
        db_client: Arc<tokio::sync::RwLock<node::database::rocksdb::Backend>>,
    ) -> Self {
        Self { db_client }
    }

    /// Internal helper to fetch a block and its label by hash, handling
    /// potential race conditions between reading the block and its label.
    async fn fetch_block_with_label_by_hash(
        &self,
        block_hash: &[u8; 32],
    ) -> Result<Option<BlockWithLabel>, DbError> {
        let db_arc = Arc::clone(&self.db_client);
        // Copy the block hash so it can be moved into the blocking task.
        let block_hash_owned = *block_hash;

        tokio::task::spawn_blocking(move || {
            db_arc.blocking_read().view(|tx| {
                match tx.block(&block_hash_owned[..]) { // Use the owned copy
                    Ok(Some(block)) => {
                        // Block found, now get its label
                        let height = block.header().height;
                        match tx.block_label_by_height(height) {
                            Ok(Some((_, label))) => Ok(Some(
                                BlockWithLabel::new_with_label(block, label),
                            )),
                            Ok(None) => Err(anyhow::anyhow!(
                                "Label not found for block hash {}",
                                hex::encode(block_hash_owned) // Use the owned copy
                            )),
                            Err(e) => Err(anyhow::anyhow!(
                                "DB error fetching label for block hash {}: {}",
                                hex::encode(block_hash_owned), // Use the owned copy
                                e
                            )),
                        }
                    }
                    Ok(None) => Ok(None), // Block not found
                    Err(e) => Err(anyhow::anyhow!(
                        "DB error fetching block by hash {}: {}",
                        hex::encode(block_hash_owned), // Use the owned copy
                        e
                    )),
                }
            })
        })
        .await
        .map_err(|join_error| {
            DbError::InternalError(format!("Task join error: {}", join_error))
        })
        .and_then(|view_result| {
            view_result
                .map_err(|db_err| DbError::QueryFailed(db_err.to_string()))
        })
    }
}

#[cfg(feature = "chain")]
#[async_trait]
impl DatabaseAdapter for RuskDbAdapter {
    /// Retrieves a full block by its hash.
    ///
    /// # Arguments
    ///
    /// * `block_hash_hex` - The hex-encoded hash of the block to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(model::block::Block))` if the block is found.
    /// * `Ok(None)` if no block exists with the given hash.
    /// * `Err(DbError)` if a database error or decoding error occurs.
    async fn get_block_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<model::block::Block>, DbError> {
        let block_hash: [u8; 32] = hex::decode(block_hash_hex)
            .map_err(|e| {
                DbError::QueryFailed(format!("Invalid block hash hex: {}", e))
            })?
            .try_into()
            .map_err(|_| {
                DbError::QueryFailed("Invalid block hash length".to_string())
            })?;

        // Correctly handle the Result<Option<...>> from the helper
        match self.fetch_block_with_label_by_hash(&block_hash).await {
            Ok(Some(bwl)) => {
                // Construct model::block::Block
                let header_model = model::block::BlockHeader::from(
                    bwl.inner().header().clone(),
                );
                let status_model = model::block::BlockStatus::from(bwl.label());
                let block_reward = 0u64; // Placeholder
                let total_fees = 0u64; // Placeholder
                let total_gas_spent = 0u64; // Placeholder

                Ok(Some(model::block::Block {
                    header: header_model,
                    status: status_model,
                    transactions: None,
                    transactions_count: bwl.inner().txs().len() as u64,
                    block_reward,
                    total_fees,
                    total_gas_spent,
                }))
            }
            Ok(None) => Ok(None), // Block not found
            Err(e) => Err(e),     // Propagate error
        }
    }

    /// Retrieves a full block by its height.
    ///
    /// # Arguments
    ///
    /// * `height` - The height of the block to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(model::block::Block))` if the block is found.
    /// * `Ok(None)` if no block exists at the given height.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_block_by_height(
        &self,
        height: u64,
    ) -> Result<Option<model::block::Block>, DbError> {
        let db_arc = Arc::clone(&self.db_client);

        let block_with_label_opt_res = tokio::task::spawn_blocking(move || {
            db_arc.blocking_read().view(|tx| {
                match tx.block_label_by_height(height) {
                    Ok(Some((hash, label))) => {
                        // Label found, now get the block
                        match tx.block(&hash[..]) { // Pass hash as slice
                            Ok(Some(block)) => Ok(Some(BlockWithLabel::new_with_label(block, label))),
                            Ok(None) => Err(anyhow::anyhow!("Block not found for hash {} at height {}", hex::encode(hash), height)),
                            Err(e) => Err(anyhow::anyhow!("DB error fetching block by hash {} at height {}: {}", hex::encode(hash), height, e)),
                        }
                    }
                    Ok(None) => Ok(None), // No block at this height
                    Err(e) => Err(anyhow::anyhow!("DB error fetching label for height {}: {}", height, e)),
                }
            })
        })
        .await
        .map_err(|join_error| DbError::InternalError(format!("Task join error: {}", join_error)))
        .and_then(|view_result| view_result.map_err(|db_err| DbError::QueryFailed(db_err.to_string())));

        // Correctly handle the Result<Option<...>> from spawn_blocking
        match block_with_label_opt_res {
            Ok(Some(bwl)) => {
                // Construct model::block::Block
                let header_model = model::block::BlockHeader::from(
                    bwl.inner().header().clone(),
                );
                let status_model = model::block::BlockStatus::from(bwl.label());
                let block_reward = 0u64; // Placeholder
                let total_fees = 0u64; // Placeholder
                let total_gas_spent = 0u64; // Placeholder

                Ok(Some(model::block::Block {
                    header: header_model,
                    status: status_model,
                    transactions: None,
                    transactions_count: bwl.inner().txs().len() as u64,
                    block_reward,
                    total_fees,
                    total_gas_spent,
                }))
            }
            Ok(None) => Ok(None), // Block not found at height
            Err(e) => Err(e),     // Propagate error
        }
    }

    /// Retrieves the latest block accepted by the node.
    ///
    /// # Returns
    ///
    /// * `Ok(model::block::Block)` containing the latest block data.
    /// * `Err(DbError)` if the latest block cannot be retrieved (e.g., DB
    ///   error, node not synced).
    async fn get_latest_block(&self) -> Result<model::block::Block, DbError> {
        let db_arc = Arc::clone(&self.db_client);

        // Need to read tip hash from metadata first
        let tip_hash_bytes = tokio::task::spawn_blocking(move || {
            db_arc.blocking_read().view(|tx| tx.op_read(MD_HASH_KEY))
        })
        .await
        .map_err(|join_error| {
            DbError::InternalError(format!("Task join error: {}", join_error))
        })
        .and_then(|view_result| {
            view_result
                .map_err(|db_err| DbError::QueryFailed(db_err.to_string()))
        })?;

        let tip_hash: [u8; 32] = tip_hash_bytes
            .ok_or_else(|| {
                DbError::NotFound(
                    "Latest block hash (tip) not found in metadata".to_string(),
                )
            })?
            .try_into()
            .map_err(|_| {
                DbError::InternalError(
                    "Invalid tip hash length in metadata".to_string(),
                )
            })?;

        // Now fetch the block and its label using the tip hash
        // Correctly handle the Result<Option<...>>
        match self.fetch_block_with_label_by_hash(&tip_hash).await? {
            Some(block_with_label) => {
                // Construct model::block::Block
                let header_model = model::block::BlockHeader::from(
                    block_with_label.inner().header().clone(),
                );
                let status_model =
                    model::block::BlockStatus::from(block_with_label.label());
                let block_reward = 0u64; // Placeholder
                let total_fees = 0u64; // Placeholder
                let total_gas_spent = 0u64; // Placeholder

                Ok(model::block::Block {
                    header: header_model,
                    status: status_model,
                    transactions: None,
                    transactions_count: block_with_label.inner().txs().len()
                        as u64,
                    block_reward,
                    total_fees,
                    total_gas_spent,
                })
            }
            None => Err(DbError::NotFound(format!(
                "Latest block not found for hash {}",
                hex::encode(tip_hash)
            ))),
        }
    }
}
