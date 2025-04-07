// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # Database Adapter Infrastructure
//!
//! This module defines the `DatabaseAdapter` trait and its concrete
//! implementation (`RuskDbAdapter`) for interacting with the Rusk node's
//! underlying database.
//!
//! ## Purpose and Design Rationale
//!
//! The primary goal of this module is to **decouple** the JSON-RPC service
//! layer from the specific database implementation used by the Rusk node
//! (currently `node::database::rocksdb::Backend`). Instead of directly using
//! the `Backend` type within RPC handlers, services interact with the database
//! through the `DatabaseAdapter` trait.
//!
//! This abstraction provides several key advantages:
//!
//! 1. **Testability:** RPC handlers and services can be easily unit-tested by
//!    providing a mock implementation of `DatabaseAdapter` (like
//!    `MockDbAdapter` in tests). This avoids the need for a running database
//!    instance during most tests, making them faster and more reliable.
//! 2. **Flexibility & Maintainability:** If the underlying database engine or
//!    its API changes in the future, only the concrete implementation
//!    (`RuskDbAdapter`) needs to be updated. The JSON-RPC services remain
//!    unaffected as long as the `DatabaseAdapter` trait contract is maintained.
//! 3. **Clear Interface:** The `DatabaseAdapter` trait defines the *specific*
//!    data access operations required by the JSON-RPC layer. This provides a
//!    focused API surface, preventing accidental coupling to unnecessary
//!    database details.
//! 4. **Encapsulation:** The `RuskDbAdapter` encapsulates database-specific
//!    logic, such as acquiring read/write locks (`RwLock`), handling potential
//!    database errors (mapping `anyhow::Error` or `rocksdb::Error` to
//!    `DbError`), and managing potentially blocking operations within
//!    `spawn_blocking`. This keeps the RPC handler logic cleaner and focused on
//!    business logic.
//!
//! ## Usage
//!
//! An `Arc`-wrapped instance implementing `DatabaseAdapter` (either
//! `RuskDbAdapter` for production or a mock for testing) is stored within the
//! `AppState`. RPC method handlers access the database via this trait object.
//!
//! ### Example: Accessing the Adapter in an RPC Handler
//!
//! ```rust,ignore
//! // Assume this is within an RPC method implementation that has access to
//! // AppState
//! use rusk::jsonrpc::infrastructure::state::AppState;
//! use rusk::jsonrpc::infrastructure::error::DbError;
//! use rusk::jsonrpc::infrastructure::db::BlockData;
//! use std::sync::Arc;
//!
//! async fn get_block(state: Arc<AppState>, height: u64) -> Result<Option<BlockData>, DbError> {
//!     // Access the adapter through AppState
//!     let db_adapter = state.db_adapter();
//!
//!     // Call trait methods
//!     let block_data = db_adapter.get_block_by_height(height).await?;
//!
//!     if let Some(ref data) = block_data {
//!         println!("Found block {}: {}", data.height, data.hash);
//!     }
//!
//!     Ok(block_data)
//! }
//! ```
//!
//! ### Example: Using a Mock Adapter in Tests
//!
//! ```rust,ignore
//! use rusk::jsonrpc::infrastructure::db::{BlockData, DatabaseAdapter};
//! use rusk::jsonrpc::infrastructure::error::DbError;
//! use async_trait::async_trait;
//! use std::sync::Arc;
//!
//! #[derive(Debug, Clone)]
//! struct MockDbAdapter;
//!
//! #[async_trait]
//! impl DatabaseAdapter for MockDbAdapter {
//!     async fn get_block_by_height(&self, height: u64) -> Result<Option<BlockData>, DbError> {
//!         if height == 100 {
//!             Ok(Some(BlockData { height: 100, hash: "mock_hash_100".to_string() }))
//!         } else {
//!             Ok(None)
//!         }
//!     }
//!     // ... implement other methods as needed ...
//! }
//!
//! #[tokio::test]
//! async fn test_service_with_mock_db() {
//!     let mock_adapter: Arc<dyn DatabaseAdapter> = Arc::new(MockDbAdapter);
//!
//!     // Pass the mock_adapter when creating AppState for the service under test
//!     // let app_state = AppState::new(/* ..., */ mock_adapter, /* ... */);
//!
//!     // Now test the service logic which will use the mock adapter
//!     // let result = service.do_something_with_block_100(app_state).await;
//!     // assert!(result.is_ok());
//! }
//! ```

use crate::jsonrpc::infrastructure::error::DbError;
use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::Arc;

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

// --- Concrete Implementation ---

/// Concrete implementation of `DatabaseAdapter` that wraps the actual Rusk
/// database backend (`node::database::rocksdb::Backend`).
///
/// This struct holds a reference to the database client wrapped in
/// `Arc<RwLock<...>>` and implements the trait methods by interacting with the
/// underlying database via the `DB` and `Ledger` traits.
#[cfg(feature = "chain")]
#[derive(Clone)]
pub struct RuskDbAdapter {
    db_client: Arc<tokio::sync::RwLock<node::database::rocksdb::Backend>>,
}

// Manual Debug implementation
#[cfg(feature = "chain")]
impl std::fmt::Debug for RuskDbAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuskDbAdapter")
            .field("db_client", &"<Rusk DB Backend>") // Placeholder for non-Debug field
            .finish()
    }
}

#[cfg(feature = "chain")]
impl RuskDbAdapter {
    /// Creates a new instance of the RuskDbAdapter.
    ///
    /// # Arguments
    ///
    /// * `db_client` - An `Arc<RwLock<...>>` wrapping the RocksDB backend
    ///   instance used by the Rusk node.
    pub fn new(
        db_client: Arc<tokio::sync::RwLock<node::database::rocksdb::Backend>>,
    ) -> Self {
        Self { db_client }
    }
}

#[cfg(feature = "chain")]
#[async_trait]
/// Implements the `DatabaseAdapter` trait for `RuskDbAdapter`.
impl DatabaseAdapter for RuskDbAdapter {
    /// Retrieves block data by its height.
    ///
    /// This implementation acquires a read lock on the database backend,
    /// executes the query within a blocking task, and maps the result
    /// (or potential errors) to the required `Result<Option<BlockData>,
    /// DbError>` type.
    async fn get_block_by_height(
        &self,
        height: u64,
    ) -> Result<Option<BlockData>, DbError> {
        // Import necessary traits and types locally
        use hex::encode as hex_encode;
        use node::database::Ledger;
        use node::database::DB;
        use rocksdb::{Error as RocksDbError, ErrorKind};
        use std::sync::Arc; // Needed for Arc::clone

        // Clone Arc for the async block
        let db_arc = Arc::clone(&self.db_client);

        // Perform the database read in a blocking task to avoid blocking the
        // async runtime
        // `view` might block internally, even if the outer function is async.
        tokio::task::spawn_blocking(move || {
            // Acquire lock inside the blocking task
            let db_guard = db_arc.blocking_read(); // Use blocking read

            // Call view on the Backend instance (obtained from the guard)
            db_guard.view(|tx| {
                // tx implements Ledger trait
                match tx.block_by_height(height) {
                    Ok(Some(block)) => {
                        // Use header() method and hex::encode
                        let hash_hex = hex_encode(block.header().hash);
                        Ok(Some(BlockData {
                            height: block.header().height,
                            hash: hash_hex,
                        }))
                    }
                    Ok(None) => Ok(None), // Block not found at this height
                    Err(e) => {
                        // Attempt to downcast anyhow::Error to rocksdb::Error
                        if let Some(db_err) = e.downcast_ref::<RocksDbError>() {
                            // Check if the rocksdb error kind indicates
                            // 'NotFound'
                            if db_err.kind() == ErrorKind::NotFound {
                                Ok(None) // Treat NotFound as Ok(None)
                            } else {
                                // Other rocksdb error, wrap it
                                Err(anyhow::anyhow!(
                                    "RocksDB error fetching block {}: {}",
                                    height,
                                    db_err
                                ))
                            }
                        } else {
                            // Error wasn't a rocksdb::Error, or downcast failed
                            Err(e)
                        }
                    }
                }
            })
        })
        .await
        .map_err(|join_error| {
            // Map JoinError to DbError
            DbError::QueryFailed(format!("Task join error: {}", join_error))
        })
        .and_then(|view_result: anyhow::Result<Option<BlockData>>| {
            // Map the inner anyhow::Result to our Result<_, DbError>
            view_result.map_err(|internal_db_error| {
                DbError::QueryFailed(internal_db_error.to_string())
            })
        })
    }

    // TODO: Implement other DatabaseAdapter methods here
}
