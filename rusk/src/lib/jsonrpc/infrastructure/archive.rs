// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # Archive Adapter Infrastructure
//!
//! This module defines the [`ArchiveAdapter`] trait, along with its concrete
//! implementation ([`RuskArchiveAdapter`]) for interacting with the Rusk node's
//! underlying archive data source.
//!
//! ## Purpose and Design Rationale
//!
//! The primary goal of this module is to **decouple** the JSON-RPC service
//! layer from the specific archive (`node::archive::Archive`) component used by
//! the Rusk node. Instead of directly using this concrete type within RPC
//! handlers, services interact with the data source through the
//! `ArchiveAdapter` trait.
//!
//! This abstraction provides several key advantages:
//!
//! 1. **Testability:** RPC handlers and services can be easily unit-tested by
//!    providing mock implementations (like `MockArchiveAdapter` found in
//!    `rusk/tests/jsonrpc/utils.rs`). This avoids the need for running archive
//!    instances during most tests, making them faster and more reliable.
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
//! `Arc`-wrapped instances implementing `ArchiveAdapter` (either concrete
//! implementation like [`RuskArchiveAdapter`] for production or mocks for
//! testing) are stored within the
//! [`AppState`](crate::jsonrpc::infrastructure::state::AppState).
//! RPC method handlers access the data sources via these trait objects.

use crate::jsonrpc::infrastructure::error::ArchiveError;
use crate::jsonrpc::model;
use async_trait::async_trait;
use hex;
use std::fmt::Debug;
use std::sync::Arc;

// Type alias for the complex return type
pub type MoonlightTxResult =
    Result<Vec<model::transaction::MoonlightEventGroup>, ArchiveError>;

// --- ArchiveAdapter Trait Definition ---

/// Defines the interface for accessing historical or indexed blockchain data
/// from the node's archive component (`node::archive::Archive`).
///
/// The archive stores data optimized for specific historical queries (like
/// transactions by memo) that might be inefficient or impossible to perform on
/// the live state database (`node::database::rocksdb::Backend`).
///
/// Implementations must be thread-safe (`Send + Sync + 'static`) and provide
/// asynchronous methods.
///
/// # Design
///
/// Separating archive access from live state access via distinct traits
/// ([`DatabaseAdapter`] vs. [`ArchiveAdapter`]) ensures clarity of purpose and
/// allows different storage/query mechanisms optimized for each use case.
/// Like `DatabaseAdapter`, this trait provides testability and flexibility.
///
/// # Errors
///
/// Methods return `Result<_, ArchiveError>` to handle potential issues like
/// archive connection problems, query failures, invalid input, or data not
/// found within the archived history.
#[async_trait]
pub trait ArchiveAdapter: Send + Sync + Debug + 'static {
    /// Retrieves groups of Moonlight transaction events associated with a
    /// specific memo from the archive.
    ///
    /// The archive specifically indexes data to make this type of query
    /// efficient. A single memo might be associated with multiple
    /// transaction events.
    ///
    /// # Arguments
    ///
    /// * `memo_hex`: A string containing the hexadecimal representation of the
    ///   transaction memo to search for.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<model::transaction::MoonlightEventGroup>)` containing groups
    ///   of events matching the memo. The vector is empty if no matches are
    ///   found in the archive.
    /// * `Err(rusk::jsonrpc::infrastructure::error::ArchiveError)` if the input
    ///   memo is invalid hex or an archive access error occurs.
    ///
    /// # Example (Conceptual)
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use async_trait::async_trait;
    /// use std::fmt::Debug;
    /// use rusk::jsonrpc::infrastructure::archive::{ArchiveAdapter, MoonlightTxResult};
    /// use rusk::jsonrpc::infrastructure::error::ArchiveError;
    /// use rusk::jsonrpc::model::transaction::MoonlightEventGroup;
    ///
    /// #[derive(Debug)]
    /// struct MyArchiveAdapter;
    ///
    /// #[async_trait]
    /// impl ArchiveAdapter for MyArchiveAdapter {
    ///   async fn get_moonlight_txs_by_memo(&self, _memo: &str) -> MoonlightTxResult {
    ///     Ok(vec![])
    ///   }
    ///   async fn get_last_archived_block_height(&self) -> Result<u64, ArchiveError> {
    ///     Ok(0)
    ///   }
    /// }
    ///
    /// async fn run(archive_adapter: Arc<dyn ArchiveAdapter>) -> Result<(), ArchiveError> {
    ///     // A valid memo hex string
    ///     let memo = "48656c6c6f20776f726c64";
    ///
    ///     let groups = archive_adapter.get_moonlight_txs_by_memo(memo).await?;
    ///     println!("Found {} groups for memo {}", groups.len(), memo);
    ///
    ///     Ok(())
    /// }
    /// ```
    async fn get_moonlight_txs_by_memo(
        &self,
        memo_hex: &str,
    ) -> MoonlightTxResult;

    /// Retrieves the block height of the last block that was successfully
    /// processed and stored by the archive component.
    ///
    /// This indicates the synchronization point of the archive relative to the
    /// main chain.
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` representing the height of the last block processed by the
    ///   archive.
    /// * `Err(rusk::jsonrpc::infrastructure::error::ArchiveError)` if the
    ///   height cannot be determined (e.g., archive not initialized, query
    ///   failed).
    async fn get_last_archived_block_height(&self)
        -> Result<u64, ArchiveError>;
}

// --- Concrete Archive Implementation ---

/// Concrete implementation of [`ArchiveAdapter`] that wraps the Rusk node's
/// archive component (`node::archive::Archive`).
///
/// This adapter provides access to historical or indexed blockchain data stored
/// separately from the live state database, optimized for specific queries like
/// finding transactions by memo.
///
/// It requires the `archive` feature flag to be enabled.
///
/// ## Thread Safety and Blocking
///
/// The `node::archive::Archive` client is wrapped in an `Arc` as it's designed
/// to be thread-safe for read operations. Operations that might interact with
/// the underlying archive storage (e.g., querying the moonlight DB) are often
/// executed using `tokio::task::spawn_blocking`.
#[cfg(feature = "archive")]
#[derive(Clone)]
pub struct RuskArchiveAdapter {
    // Archive client wrapped in an `Arc<...>` only because it doesn't perform
    // write operations and is thread-safe.
    archive_client: Arc<node::archive::Archive>,
}

// Manual Debug implementation to avoid exposing internal client details.
#[cfg(feature = "archive")]
impl std::fmt::Debug for RuskArchiveAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuskArchiveAdapter")
            .field("archive_client", &"<Rusk Archive Component>")
            .finish()
    }
}

#[cfg(feature = "archive")]
impl RuskArchiveAdapter {
    /// Creates a new instance of the `RuskArchiveAdapter`.
    ///
    /// # Arguments
    ///
    /// * `archive_client`: An `Arc` wrapping the node's
    ///   `node::archive::Archive` instance.
    ///
    /// # Returns
    ///
    /// A new adapter ready to interact with the archive data.
    pub fn new(archive_client: Arc<node::archive::Archive>) -> Self {
        Self { archive_client }
    }
}

#[cfg(feature = "archive")]
#[async_trait]
impl ArchiveAdapter for RuskArchiveAdapter {
    /// Retrieves Moonlight transactions (represented by MoonlightEventGroup)
    /// associated with a specific memo (hex-encoded).
    async fn get_moonlight_txs_by_memo(
        &self,
        memo_hex: &str,
    ) -> MoonlightTxResult {
        let memo_bytes = hex::decode(memo_hex).map_err(|e| {
            ArchiveError::QueryFailed(format!("Invalid hex memo: {}", e))
        })?;

        let client = Arc::clone(&self.archive_client);

        // Database interaction can block, so use spawn_blocking.
        let result = tokio::task::spawn_blocking(move || {
            client.moonlight_txs_by_memo(memo_bytes)
        })
        .await
        .map_err(|e| {
            ArchiveError::InternalError(format!("Task join error: {}", e))
        })?;

        match result {
            Ok(Some(groups)) => Ok(groups
                .into_iter()
                .map(model::transaction::MoonlightEventGroup::from) // Use From trait
                .collect()),
            Ok(None) => Ok(vec![]), // No groups found for this memo
            Err(e) => {
                // Map anyhow::Error to ArchiveError.
                Err(ArchiveError::QueryFailed(format!(
                    "Failed to query moonlight txs by memo: {}",
                    e
                )))
            }
        }
    }

    /// Retrieves the height of the last block successfully processed and stored
    /// by the archive.
    async fn get_last_archived_block_height(
        &self,
    ) -> Result<u64, ArchiveError> {
        // This method in node::archive::Archive is synchronous and just reads a
        // field.
        let client = Arc::clone(&self.archive_client);
        // spawn_blocking might be slight overkill but ensures consistency.
        let height = tokio::task::spawn_blocking(move || {
            client.last_finalized_block_height()
        })
        .await
        .map_err(|e| {
            ArchiveError::InternalError(format!("Task join error: {}", e))
        })?;
        Ok(height)
    }
}
