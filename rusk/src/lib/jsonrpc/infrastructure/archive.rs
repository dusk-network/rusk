// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # JSON-RPC Archive Adapter Infrastructure
//!
//! This module defines the [`ArchiveAdapter`] trait and its concrete
//! implementation, [`RuskArchiveAdapter`], providing a standardized interface
//! for the JSON-RPC layer to interact with the Rusk node's underlying archive
//! data source (`node::archive::Archive`).
//!
//! ## Purpose and Design
//!
//! The primary goal is to **decouple** the JSON-RPC service logic from the
//! specifics of the `node::archive::Archive` component. Services interact with
//! the archive data solely through the `ArchiveAdapter` trait, enabling:
//!
//! * **Testability**: RPC handlers can be unit-tested using mock adapters
//!   (e.g., `MockArchiveAdapter` in `rusk/tests/jsonrpc/utils.rs`), removing
//!   the need for a running archive instance during tests.
//! * **Flexibility**: Changes to the underlying `node::archive` implementation
//!   only require updates to `RuskArchiveAdapter`, minimizing impact on the RPC
//!   layer as long as the trait contract is upheld.
//! * **Clear Interface**: The trait defines only the data access operations
//!   needed by the JSON-RPC layer, preventing tight coupling.
//! * **Encapsulation**: Adapters handle component-specific details like error
//!   mapping and asynchronous execution (`spawn_blocking` for synchronous node
//!   methods), keeping RPC logic cleaner.
//!
//! ## Usage
//!
//! An `Arc<dyn ArchiveAdapter>` (either `RuskArchiveAdapter` for production or
//! a mock for testing) is typically stored within the `AppState`
//! ([`crate::jsonrpc::infrastructure::state::AppState`]). RPC method handlers
//! access the archive data source via this trait object.
//!
//! ## Error Handling
//!
//! All methods within the `ArchiveAdapter` trait return
//! `Result<_, ArchiveError>`, where
//! [`ArchiveError`](crate::jsonrpc::infrastructure::error::ArchiveError) is an
//! enum encompassing potential issues like database errors, query failures,
//! invalid input, or data conversion problems.

use crate::jsonrpc::infrastructure::error::ArchiveError;
use crate::jsonrpc::model;
use crate::jsonrpc::model::archive::MoonlightEventGroup;
use async_trait::async_trait;
use bs58;
use dusk_bytes::DeserializableSlice;
use dusk_core::signatures::bls::PublicKey as NodePublicKey;
use node::archive::Order as NodeOrder;
use std::fmt::Debug;
use std::sync::Arc;

// Type alias for the complex return type
pub type MoonlightTxResult =
    Result<Vec<model::archive::MoonlightEventGroup>, ArchiveError>;

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
///
/// # Examples
///
/// ```rust
/// # use rusk::jsonrpc::infrastructure::archive::{ArchiveAdapter, RuskArchiveAdapter};
/// # use rusk::jsonrpc::model::archive::{ArchivedEvent, Order, MoonlightEventGroup};
/// # use std::sync::Arc;
/// # use async_trait::async_trait;
/// # use std::fmt::Debug;
/// # use rusk::jsonrpc::infrastructure::error::ArchiveError;
/// #
/// // Mock adapter implementation for the example
/// #[derive(Debug)]
/// struct ExampleMockArchiveAdapter;
///
/// #[async_trait]
/// impl ArchiveAdapter for ExampleMockArchiveAdapter {
///     async fn get_moonlight_txs_by_memo(&self, _memo: Vec<u8>) -> Result<Option<Vec<MoonlightEventGroup>>, ArchiveError> { Ok(None) }
///     async fn get_last_archived_block(&self) -> Result<(u64, String), ArchiveError> { Ok((100, "hash100".to_string())) }
///     async fn get_block_events_by_hash(&self, _hex_block_hash: &str) -> Result<Vec<ArchivedEvent>, ArchiveError> {
///         Ok(vec![ArchivedEvent { // Return one dummy event
///             origin: "origin1".to_string(),
///             topic: "topic1".to_string(),
///             source: "source1".to_string(),
///             data: vec![1,2,3]
///         }])
///     }
///     async fn get_block_events_by_height(&self, _block_height: u64) -> Result<Vec<ArchivedEvent>, ArchiveError> { Ok(vec![]) }
///     async fn get_latest_block_events(&self) -> Result<Vec<ArchivedEvent>, ArchiveError> { Ok(vec![]) }
///     async fn get_contract_finalized_events(&self, _contract_id: &str) -> Result<Vec<ArchivedEvent>, ArchiveError> { Ok(vec![]) }
///     async fn get_next_block_with_phoenix_transaction(&self, _block_height: u64) -> Result<Option<u64>, ArchiveError> { Ok(None) }
///     async fn get_moonlight_transaction_history(&self, _pk_bs58: String, _ord: Option<Order>, _from_block: Option<u64>, _to_block: Option<u64>) -> Result<Option<Vec<MoonlightEventGroup>>, ArchiveError> { Ok(None) }
/// }
///
/// // Function demonstrating usage of the adapter trait
/// async fn process_events(adapter: Arc<dyn ArchiveAdapter>, hex_hash: &str) -> Result<(), ArchiveError> {
///     let events: Vec<ArchivedEvent> = adapter.get_block_events_by_hash(hex_hash).await?;
///     println!("Found {} events for hash {}", events.len(), hex_hash);
///     // ... further processing ...
///     Ok(())
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), ArchiveError> {
///     // In a real application, `adapter` would be obtained from AppState or similar.
///     // Here, we use the mock implementation directly for the example.
///     let adapter: Arc<dyn ArchiveAdapter> = Arc::new(ExampleMockArchiveAdapter);
///     let hex_hash = "valid_hex_hash_string";
///
///     process_events(adapter, hex_hash).await?;
///
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait ArchiveAdapter: Send + Sync + Debug + 'static {
    /// Fetches Moonlight transaction groups associated with a specific memo.
    ///
    /// Moonlight transactions often include an encrypted memo field. This
    /// method allows querying the archive for all transaction groups
    /// (representing single transactions) that contain a specific memo byte
    /// sequence.
    ///
    /// # Arguments
    ///
    /// * `memo`: The raw byte sequence of the memo to search for.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Vec<MoonlightEventGroup>))`: If transactions with the given
    ///   memo are found, returns a vector of corresponding event groups.
    /// * `Ok(None)`: If no transactions with the given memo are found in the
    ///   archive.
    /// * `Err(ArchiveError)`: If the query fails due to database issues or
    ///   other internal errors.
    async fn get_moonlight_txs_by_memo(
        &self,
        memo: Vec<u8>,
    ) -> Result<Option<Vec<model::archive::MoonlightEventGroup>>, ArchiveError>;

    /// Fetches the height and hash of the most recent block marked as finalized
    /// within the archive.
    ///
    /// The archive might lag slightly behind the node's absolute latest block,
    /// so this represents the tip of the *archived* finalized chain.
    ///
    /// # Returns
    ///
    /// * `Ok((u64, String))`: A tuple containing the block height and its
    ///   corresponding hex-encoded block hash.
    /// * `Err(ArchiveError::NotFound)`: If no finalized blocks are found in the
    ///   archive (e.g., during initial sync).
    /// * `Err(ArchiveError::QueryFailed)`: If the query fails due to database
    ///   issues.
    async fn get_last_archived_block(
        &self,
    ) -> Result<(u64, String), ArchiveError>;

    /// Fetches all archived VM events associated with a specific block hash.
    ///
    /// This retrieves events regardless of whether the block itself is
    /// currently marked as finalized or unfinalized within the archive's
    /// perspective.
    ///
    /// # Arguments
    ///
    /// * `hex_block_hash`: The hex-encoded string representation of the block
    ///   hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ArchivedEvent>)`: A vector containing all archived events for
    ///   the specified block. Returns an empty vector if the block is found but
    ///   has no associated events, or if the block hash is not found in the
    ///   archive.
    /// * `Err(ArchiveError::QueryFailed)`: If the query fails due to database
    ///   issues.
    async fn get_block_events_by_hash(
        &self,
        hex_block_hash: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError>;

    /// Fetches all archived VM events associated with a specific block height.
    ///
    /// Similar to `get_block_events_by_hash`, this retrieves events regardless
    /// of the block's finalization status in the archive.
    ///
    /// # Arguments
    ///
    /// * `block_height`: The block height number.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ArchivedEvent>)`: A vector containing all archived events for
    ///   the specified block height. Returns an empty vector if the block is
    ///   found but has no associated events, or if the block height is not
    ///   found in the archive.
    /// * `Err(ArchiveError::QueryFailed)`: If the query fails due to database
    ///   issues.
    async fn get_block_events_by_height(
        &self,
        block_height: u64,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError>;

    /// Fetches all archived VM events from the latest block known to the
    /// archive (regardless of finalization status).
    ///
    /// This is useful for getting the most recent events indexed by the
    /// archive, which might include events from blocks not yet marked as
    /// finalized.
    ///
    /// # Implementation Note
    ///
    /// This method is typically implemented by first calling
    /// [`get_last_archived_block`](ArchiveAdapter::get_last_archived_block) to
    /// find the latest height and then calling
    /// [`get_block_events_by_height`](ArchiveAdapter::get_block_events_by_height)
    /// with that height.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ArchivedEvent>)`: A vector containing all archived events for
    ///   the latest block found in the archive. Returns an empty vector if the
    ///   latest block has no events.
    /// * `Err(ArchiveError)`: If fetching the last block height or fetching
    ///   events by height fails.
    async fn get_latest_block_events(
        &self,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError>;

    /// Fetches all **finalized** VM events emitted by a specific contract ID.
    ///
    /// This retrieves only events from blocks that are marked as finalized
    /// within the archive.
    ///
    /// # Arguments
    ///
    /// * `contract_id`: The identifier string of the contract (e.g., a
    ///   hex-encoded ID).
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ArchivedEvent>)`: A vector containing all finalized events
    ///   emitted by the specified contract. Returns an empty vector if the
    ///   contract has emitted no finalized events or is not found.
    /// * `Err(ArchiveError::QueryFailed)`: If the query fails due to database
    ///   issues.
    async fn get_contract_finalized_events(
        &self,
        contract_id: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError>;

    /// Finds the height of the next block **after** the given height that
    /// contains at least one Phoenix transaction.
    ///
    /// Phoenix transactions are a specific type within the Dusk ecosystem.
    /// This query helps in navigating the chain based on the presence of these
    /// transactions.
    ///
    /// # Arguments
    ///
    /// * `block_height`: The height *after* which to start searching for a
    ///   block containing a Phoenix transaction.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(u64))`: The height of the next block containing a Phoenix
    ///   transaction.
    /// * `Ok(None)`: If no subsequent block contains a Phoenix transaction.
    /// * `Err(ArchiveError::QueryFailed)`: If the query fails due to database
    ///   issues.
    async fn get_next_block_with_phoenix_transaction(
        &self,
        block_height: u64,
    ) -> Result<Option<u64>, ArchiveError>;

    /// Fetches the full Moonlight transaction history for a given account,
    /// identified by its public key.
    ///
    /// Allows filtering by block range and specifying the order of results.
    /// Moonlight history includes transactions where the given account was
    /// either the sender or the receiver.
    ///
    /// # Arguments
    ///
    /// * `pk_bs58`: The Base58 encoded public key string of the account.
    /// * `ord`: An optional [`Order`](model::archive::Order) enum specifying
    ///   whether to sort results `Ascending` or `Descending` by block height.
    ///   Defaults typically to descending (newest first) if `None`.
    /// * `from_block`: An optional block height to start the history from
    ///   (inclusive).
    /// * `to_block`: An optional block height to end the history at
    ///   (inclusive).
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Vec<MoonlightEventGroup>))`: If history is found for the
    ///   account within the specified range, returns a vector of event groups,
    ///   sorted according to `ord`.
    /// * `Ok(None)`: If no Moonlight transaction history is found for the
    ///   account in the specified range.
    /// * `Err(ArchiveError::QueryFailed)`: If the input `pk_bs58` is invalid,
    ///   if the query fails due to database issues, or other internal errors.
    async fn get_moonlight_transaction_history(
        &self,
        pk_bs58: String,
        ord: Option<model::archive::Order>,
        from_block: Option<u64>,
        to_block: Option<u64>,
    ) -> Result<Option<Vec<model::archive::MoonlightEventGroup>>, ArchiveError>;

    // --- Default Methods ---

    /// Fetches finalized events from a specific contract, filtered by event
    /// topic.
    ///
    /// This is a convenience method that first calls
    /// [`get_contract_finalized_events`](ArchiveAdapter::get_contract_finalized_events)
    /// and then filters the results based on the provided `topic` string.
    ///
    /// # Arguments
    ///
    /// * `contract_id`: The identifier string of the contract.
    /// * `topic`: The exact event topic string to filter by.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ArchivedEvent>)`: A vector containing finalized events from
    ///   the contract that match the specified topic. Returns an empty vector
    ///   if no matching events are found.
    /// * `Err(ArchiveError)`: If the underlying call to
    ///   `get_contract_finalized_events` fails.
    async fn get_contract_events_by_topic(
        &self,
        contract_id: &str,
        topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        // Default implementation logic:
        let events = self.get_contract_finalized_events(contract_id).await?;
        Ok(events.into_iter().filter(|e| e.topic == topic).collect())
    }

    /// Fetches the height of the last block finalized in the archive.
    ///
    /// A convenience method that calls
    /// [`get_last_archived_block`](ArchiveAdapter::get_last_archived_block) and
    /// extracts only the height component.
    ///
    /// # Returns
    ///
    /// * `Ok(u64)`: The height of the last finalized block in the archive.
    /// * `Err(ArchiveError)`: If the underlying call to
    ///   `get_last_archived_block` fails.
    async fn get_last_archived_block_height(
        &self,
    ) -> Result<u64, ArchiveError> {
        self.get_last_archived_block().await.map(|(h, _)| h)
    }

    /// Fetches all finalized events emitted by a specific contract.
    ///
    /// This is an alias for
    /// [`get_contract_finalized_events`](ArchiveAdapter::get_contract_finalized_events).
    /// It provides a potentially more intuitive name depending on the calling
    /// context.
    ///
    /// # Arguments
    ///
    /// * `contract_id`: The identifier string of the contract.
    ///
    /// # Returns
    ///
    /// See [`get_contract_finalized_events`](ArchiveAdapter::get_contract_finalized_events).
    async fn get_contract_events(
        &self,
        contract_id: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        self.get_contract_finalized_events(contract_id).await
    }

    /// Fetches events from a specific block height, filtered by source contract
    /// ID.
    ///
    /// Calls [`get_block_events_by_height`](ArchiveAdapter::get_block_events_by_height)
    /// and filters the results, keeping only events where the `source` field
    /// matches the provided `contract_id`.
    ///
    /// # Arguments
    ///
    /// * `block_height`: The block height number.
    /// * `contract_id`: The identifier string of the source contract to filter
    ///   by.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ArchivedEvent>)`: A vector containing events from the
    ///   specified block height whose source matches the `contract_id`. Returns
    ///   an empty vector if no matching events are found.
    /// * `Err(ArchiveError)`: If the underlying call to
    ///   `get_block_events_by_height` fails.
    async fn get_contract_events_by_block_height(
        &self,
        block_height: u64,
        contract_id: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        let events = self.get_block_events_by_height(block_height).await?;
        Ok(events
            .into_iter()
            .filter(|e| e.source == contract_id)
            .collect())
    }

    /// Fetches events from a specific block hash, filtered by source contract
    /// ID.
    ///
    /// Calls [`get_block_events_by_hash`](ArchiveAdapter::get_block_events_by_hash)
    /// and filters the results, keeping only events where the `source` field
    /// matches the provided `contract_id`.
    ///
    /// # Arguments
    ///
    /// * `hex_block_hash`: The hex-encoded string of the block hash.
    /// * `contract_id`: The identifier string of the source contract to filter
    ///   by.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ArchivedEvent>)`: A vector containing events from the
    ///   specified block hash whose source matches the `contract_id`. Returns
    ///   an empty vector if no matching events are found.
    /// * `Err(ArchiveError)`: If the underlying call to
    ///   `get_block_events_by_hash` fails.
    async fn get_contract_events_by_block_hash(
        &self,
        hex_block_hash: &str,
        contract_id: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        let events = self.get_block_events_by_hash(hex_block_hash).await?;
        Ok(events
            .into_iter()
            .filter(|e| e.source == contract_id)
            .collect())
    }

    /// Fetches finalized contract events considered as 'transactions' (alias
    /// for [`get_contract_events`](ArchiveAdapter::get_contract_events)).
    ///
    /// Provides an alternative naming convention where general contract events
    /// are referred to as transactions.
    ///
    /// # Arguments
    ///
    /// * `contract_id`: The identifier string of the contract.
    ///
    /// # Returns
    ///
    /// See [`get_contract_events`](ArchiveAdapter::get_contract_events).
    async fn get_contract_transactions(
        &self,
        contract_id: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        self.get_contract_events(contract_id).await
    }

    /// Fetches finalized contract events from a specific block height
    /// considered as 'transactions' (alias for
    /// [`get_contract_events_by_block_height`](ArchiveAdapter::get_contract_events_by_block_height)).
    ///
    /// Provides an alternative naming convention.
    ///
    /// # Arguments
    ///
    /// * `block_height`: The block height number.
    /// * `contract_id`: The identifier string of the source contract.
    ///
    /// # Returns
    ///
    /// See [`get_contract_events_by_block_height`](ArchiveAdapter::get_contract_events_by_block_height).
    async fn get_contract_transactions_by_block_height(
        &self,
        block_height: u64,
        contract_id: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        self.get_contract_events_by_block_height(block_height, contract_id)
            .await
    }

    /// Fetches finalized contract events from a specific block hash considered
    /// as 'transactions' (alias for
    /// [`get_contract_events_by_block_hash`](ArchiveAdapter::get_contract_events_by_block_hash)).
    ///
    /// Provides an alternative naming convention.
    ///
    /// # Arguments
    ///
    /// * `hex_block_hash`: The hex-encoded string of the block hash.
    /// * `contract_id`: The identifier string of the source contract.
    ///
    /// # Returns
    ///
    /// See [`get_contract_events_by_block_hash`](ArchiveAdapter::get_contract_events_by_block_hash).
    async fn get_contract_transactions_by_block_hash(
        &self,
        hex_block_hash: &str,
        contract_id: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        self.get_contract_events_by_block_hash(hex_block_hash, contract_id)
            .await
    }

    // --- Topic-Specific Event Getters ---

    // These methods are thin wrappers around `get_contract_events_by_topic`
    // for commonly queried event topics. Callers must provide the exact topic
    // string constants (e.g., from `node_data::events::contract`).

    /// Fetches finalized 'item added' events from a specific contract.
    ///
    /// This is a convenience method that calls
    /// [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic)
    /// with the provided `item_added_topic` string.
    ///
    /// # Arguments
    ///
    /// * `contract_id`: The identifier string of the contract.
    /// * `item_added_topic`: The exact topic string constant representing 'item
    ///   added' events (e.g., from `node_data::events::contract`).
    ///
    /// # Returns
    ///
    /// Ok(Vec<model::archive::ArchivedEvent>) if the events are found.
    /// Err(ArchiveError) if an error occurs.
    ///
    /// See [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic).
    async fn get_item_added_events(
        &self,
        contract_id: &str,
        item_added_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        self.get_contract_events_by_topic(contract_id, item_added_topic)
            .await
    }

    /// Fetches finalized 'item removed' events from a specific contract.
    ///
    /// This is a convenience method that calls
    /// [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic)
    /// with the provided `item_removed_topic` string.
    ///
    /// # Arguments
    ///
    /// * `contract_id`: The identifier string of the contract.
    /// * `item_removed_topic`: The exact topic string constant representing
    ///   'item removed' events.
    ///
    /// # Returns
    ///
    /// Ok(Vec<model::archive::ArchivedEvent>) if the events are found.
    /// Err(ArchiveError) if an error occurs.
    ///
    /// See [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic).
    async fn get_item_removed_events(
        &self,
        contract_id: &str,
        item_removed_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        self.get_contract_events_by_topic(contract_id, item_removed_topic)
            .await
    }

    /// Fetches finalized 'item modified' events from a specific contract.
    ///
    /// This is a convenience method that calls
    /// [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic)
    /// with the provided `item_modified_topic` string.
    ///
    /// # Arguments
    ///
    /// * `contract_id`: The identifier string of the contract.
    /// * `item_modified_topic`: The exact topic string constant representing
    ///   'item modified' events.
    ///
    /// # Returns
    ///
    /// Ok(Vec<model::archive::ArchivedEvent>) if the events are found.
    /// Err(ArchiveError) if an error occurs.
    ///
    /// See [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic).
    async fn get_item_modified_events(
        &self,
        contract_id: &str,
        item_modified_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        self.get_contract_events_by_topic(contract_id, item_modified_topic)
            .await
    }

    /// Fetches finalized 'stake' events from a specific contract.
    ///
    /// This is a convenience method that calls
    /// [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic)
    /// with the provided `stake_topic` string.
    ///
    /// # Arguments
    ///
    /// * `contract_id`: The identifier string of the contract.
    /// * `stake_topic`: The exact topic string constant representing 'stake'
    ///   events.
    ///
    /// # Returns
    ///
    /// Ok(Vec<model::archive::ArchivedEvent>) if the events are found.
    /// Err(ArchiveError) if an error occurs.
    ///
    /// See [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic).
    async fn get_stake_events(
        &self,
        contract_id: &str,
        stake_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        self.get_contract_events_by_topic(contract_id, stake_topic)
            .await
    }

    /// Fetches finalized 'unstake' events from a specific contract.
    ///
    /// This is a convenience method that calls
    /// [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic)
    /// with the provided `unstake_topic` string.
    ///
    /// # Arguments
    ///
    /// * `contract_id`: The identifier string of the contract.
    /// * `unstake_topic`: The exact topic string constant representing
    ///   'unstake' events.
    ///
    /// # Returns
    ///
    /// Ok(Vec<model::archive::ArchivedEvent>) if the events are found.
    /// Err(ArchiveError) if an error occurs.
    ///
    /// See [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic).
    async fn get_unstake_events(
        &self,
        contract_id: &str,
        unstake_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        self.get_contract_events_by_topic(contract_id, unstake_topic)
            .await
    }

    /// Fetches finalized 'slash' events from a specific contract.
    ///
    /// This is a convenience method that calls
    /// [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic)
    /// with the provided `slash_topic` string.
    ///
    /// # Arguments
    ///
    /// * `contract_id`: The identifier string of the contract.
    /// * `slash_topic`: The exact topic string constant representing 'slash'
    ///   events.
    ///
    /// # Returns
    ///
    /// Ok(Vec<model::archive::ArchivedEvent>) if the events are found.
    /// Err(ArchiveError) if an error occurs.
    ///
    /// See [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic).
    async fn get_slash_events(
        &self,
        contract_id: &str,
        slash_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        self.get_contract_events_by_topic(contract_id, slash_topic)
            .await
    }

    /// Fetches finalized 'hard slash' events from a specific contract.
    ///
    /// This is a convenience method that calls
    /// [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic)
    /// with the provided `hard_slash_topic` string.
    ///
    /// # Arguments
    ///
    /// * `contract_id`: The identifier string of the contract.
    /// * `hard_slash_topic`: The exact topic string constant representing 'hard
    ///   slash' events.
    ///
    /// # Returns
    ///
    /// Ok(Vec<model::archive::ArchivedEvent>) if the events are found.
    /// Err(ArchiveError) if an error occurs.
    ///
    /// See [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic).
    async fn get_hard_slash_events(
        &self,
        contract_id: &str,
        hard_slash_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        self.get_contract_events_by_topic(contract_id, hard_slash_topic)
            .await
    }

    /// Fetches finalized 'provisioner changes' events from a specific contract.
    ///
    /// This is a convenience method that calls
    /// [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic)
    /// with the provided `provisioner_changes_topic` string.
    ///
    /// # Arguments
    ///
    /// * `contract_id`: The identifier string of the contract.
    /// * `provisioner_changes_topic`: The exact topic string constant
    ///   representing 'provisioner changes' events.
    ///
    /// # Returns
    ///
    /// Ok(Vec<model::archive::ArchivedEvent>) if the events are found.
    /// Err(ArchiveError) if an error occurs.
    ///
    /// See [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic).
    async fn get_provisioner_changes(
        &self,
        contract_id: &str,
        provisioner_changes_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        self.get_contract_events_by_topic(
            contract_id,
            provisioner_changes_topic,
        )
        .await
    }

    /// Fetches finalized 'transfer' events from a specific contract (e.g.,
    /// "moonlight").
    ///
    /// This is a convenience method that calls
    /// [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic)
    /// with the provided `transfer_topic` string.
    ///
    /// # Arguments
    ///
    /// * `contract_id`: The identifier string of the contract.
    /// * `transfer_topic`: The exact topic string constant representing
    ///   'transfer' events (e.g.,
    ///   `node_data::events::contract::MOONLIGHT_TOPIC`).
    ///
    /// # Returns
    ///
    /// Ok(Vec<model::archive::ArchivedEvent>) if the events are found.
    /// Err(ArchiveError) if an error occurs.
    ///
    /// See [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic).
    async fn get_transfer_events(
        &self,
        contract_id: &str,
        transfer_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        self.get_contract_events_by_topic(contract_id, transfer_topic)
            .await
    }

    /// Fetches finalized 'deposit' events from a specific contract.
    ///
    /// This is a convenience method that calls
    /// [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic)
    /// with the provided `deposit_topic` string.
    ///
    /// # Arguments
    ///
    /// * `contract_id`: The identifier string of the contract.
    /// * `deposit_topic`: The exact topic string constant representing
    ///   'deposit' events.
    ///
    /// # Returns
    ///
    /// Ok(Vec<model::archive::ArchivedEvent>) if the events are found.
    /// Err(ArchiveError) if an error occurs.
    ///
    /// See [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic).
    async fn get_deposit_events(
        &self,
        contract_id: &str,
        deposit_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        self.get_contract_events_by_topic(contract_id, deposit_topic)
            .await
    }

    /// Fetches finalized 'withdraw' events from a specific contract.
    ///
    /// This is a convenience method that calls
    /// [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic)
    /// with the provided `withdraw_topic` string.
    ///
    /// # Arguments
    ///
    /// * `contract_id`: The identifier string of the contract.
    /// * `withdraw_topic`: The exact topic string constant representing
    ///   'withdraw' events (e.g.,
    ///   `node_data::events::contract::WITHDRAW_TOPIC`).
    ///
    /// # Returns
    ///
    /// Ok(Vec<model::archive::ArchivedEvent>) if the events are found.
    /// Err(ArchiveError) if an error occurs.
    ///
    /// See [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic).
    async fn get_withdraw_events(
        &self,
        contract_id: &str,
        withdraw_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        self.get_contract_events_by_topic(contract_id, withdraw_topic)
            .await
    }

    /// Fetches finalized 'convert' events from a specific contract.
    ///
    /// This is a convenience method that calls
    /// [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic)
    /// with the provided `convert_topic` string.
    ///
    /// # Arguments
    ///
    /// * `contract_id`: The identifier string of the contract.
    /// * `convert_topic`: The exact topic string constant representing
    ///   'convert' events (e.g., `node_data::events::contract::CONVERT_TOPIC`).
    ///
    /// # Returns
    ///
    /// Ok(Vec<model::archive::ArchivedEvent>) if the events are found.
    /// Err(ArchiveError) if an error occurs.
    ///
    /// See [`get_contract_events_by_topic`](ArchiveAdapter::get_contract_events_by_topic).
    async fn get_convert_events(
        &self,
        contract_id: &str,
        convert_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        self.get_contract_events_by_topic(contract_id, convert_topic)
            .await
    }
}

// --- Concrete Archive Implementation ---

/// Concrete implementation of [`ArchiveAdapter`] that wraps the Rusk node's
/// underlying archive component (`node::archive::Archive`).
///
/// This struct adapts the potentially synchronous or asynchronous methods of
/// the `node::archive::Archive` to the fully asynchronous interface defined by
/// [`ArchiveAdapter`], handling necessary data conversions and error mapping.
///
/// It requires the `archive` feature flag to be enabled, as it directly depends
/// on types within the `node` crate, specifically `node::archive::Archive`.
///
/// ## Thread Safety and Blocking
///
/// The inner `archive_client: Arc<node::archive::Archive>` is assumed to be
/// thread-safe (`Send + Sync`).
/// - For underlying `node::archive::Archive` methods that are **synchronous**
///   (like `moonlight_txs_by_memo`, `full_moonlight_history`), this adapter
///   uses [`tokio::task::spawn_blocking`] to execute them on a blocking thread
///   pool, preventing them from starving the async runtime.
/// - For underlying methods that are **asynchronous** (like
///   `fetch_events_by_hash`, `fetch_last_finalized_block`), this adapter
///   directly `.await`s them.
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
    /// Fetches Moonlight transaction groups associated with a specific memo.
    ///
    /// Moonlight transactions often include an encrypted memo field. This
    /// method allows querying the archive for all transaction groups
    /// (representing single transactions) that contain a specific memo byte
    /// sequence.
    ///
    /// # Arguments
    ///
    /// * `memo`: The raw byte sequence of the memo to search for.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Vec<MoonlightEventGroup>))`: If transactions with the given
    ///   memo are found, returns a vector of corresponding event groups.
    /// * `Ok(None)`: If no transactions with the given memo are found in the
    ///   archive.
    /// * `Err(ArchiveError)`: If the query fails due to database issues or
    ///   other internal errors.
    ///
    /// # Implementation details:
    /// - Calls the synchronous `node::archive::Archive::moonlight_txs_by_memo`.
    /// - Executes the call within `tokio::task::spawn_blocking`.
    /// - Converts `node::archive::MoonlightGroup` to
    ///   `model::archive::MoonlightEventGroup`.
    /// - Maps errors to `ArchiveError`.
    async fn get_moonlight_txs_by_memo(
        &self,
        memo: Vec<u8>,
    ) -> Result<Option<Vec<model::archive::MoonlightEventGroup>>, ArchiveError>
    {
        let client = Arc::clone(&self.archive_client);
        let result = tokio::task::spawn_blocking(move || {
            client.moonlight_txs_by_memo(memo)
        })
        .await
        .map_err(|e| {
            ArchiveError::InternalError(format!("Task join error: {}", e))
        })?;

        result
            .map(|opt_node_groups| {
                opt_node_groups.map(|node_groups| {
                    node_groups
                        .into_iter()
                        .map(MoonlightEventGroup::from)
                        .collect()
                })
            })
            .map_err(|node_err| {
                ArchiveError::QueryFailed(format!(
                    "Archive query failed fetching moonlight txs by memo: {}",
                    node_err
                ))
            })
    }

    /// Fetches the height and hash of the most recent block marked as finalized
    /// within the archive.
    ///
    /// The archive might lag slightly behind the node's absolute latest block,
    /// so this represents the tip of the *archived* finalized chain.
    ///
    /// # Returns
    ///
    /// * `Ok((u64, String))`: A tuple containing the block height and its
    ///   corresponding hex-encoded block hash.
    /// * `Err(ArchiveError::NotFound)`: If no finalized blocks are found in the
    ///   archive (e.g., during initial sync).
    /// * `Err(ArchiveError::QueryFailed)`: If the query fails due to database
    ///   issues.
    ///
    /// # Implementation details:
    /// - Calls the asynchronous
    ///   `node::archive::Archive::fetch_last_finalized_block`.
    /// - Directly `.await`s the result.
    /// - Maps errors to `ArchiveError`.
    async fn get_last_archived_block(
        &self,
    ) -> Result<(u64, String), ArchiveError> {
        let client = Arc::clone(&self.archive_client);
        client
            .fetch_last_finalized_block()
            .await
            .map_err(|node_err| {
                ArchiveError::QueryFailed(format!(
                    "Archive query failed fetching last finalized block: {}",
                    node_err
                ))
            })
    }

    /// Fetches all archived VM events associated with a specific block hash.
    ///
    /// This retrieves events regardless of whether the block itself is
    /// currently marked as finalized or unfinalized within the archive's
    /// perspective.
    ///
    /// # Arguments
    ///
    /// * `hex_block_hash`: The hex-encoded string representation of the block
    ///   hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ArchivedEvent>)`: A vector containing all archived events for
    ///   the specified block. Returns an empty vector if the block is found but
    ///   has no associated events, or if the block hash is not found in the
    ///   archive.
    /// * `Err(ArchiveError::QueryFailed)`: If the query fails due to database
    ///   issues.
    ///
    /// # Implementation details:
    /// - Calls the asynchronous `node::archive::Archive::fetch_events_by_hash`.
    /// - Directly `.await`s the result.
    /// - Converts `node::archive::ArchivedEvent` to
    ///   `model::archive::ArchivedEvent` using `Into::into`.
    /// - Maps errors to `ArchiveError`.
    async fn get_block_events_by_hash(
        &self,
        hex_block_hash: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        let client = Arc::clone(&self.archive_client);
        let block_hash_str = hex_block_hash.to_string(); // Clone for the closure
        let node_events = client
            .fetch_events_by_hash(&block_hash_str)
            .await // Await the future
            .map_err(|node_err| {
                ArchiveError::QueryFailed(format!(
                    "Archive query failed fetching events by hash: {}",
                    node_err
                ))
            })?;

        Ok(node_events.into_iter().map(Into::into).collect())
    }

    /// Fetches all archived VM events associated with a specific block height.
    ///
    /// Similar to `get_block_events_by_hash`, this retrieves events regardless
    /// of the block's finalization status in the archive.
    ///
    /// # Arguments
    ///
    /// * `block_height`: The block height number.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ArchivedEvent>)`: A vector containing all archived events for
    ///   the specified block height. Returns an empty vector if the block is
    ///   found but has no associated events, or if the block height is not
    ///   found in the archive.
    /// * `Err(ArchiveError::QueryFailed)`: If the query fails due to database
    ///   issues.
    ///
    /// # Implementation details:
    /// - Calls the asynchronous
    ///   `node::archive::Archive::fetch_events_by_height`.
    /// - Casts the input `block_height: u64` to `i64` for the node method.
    /// - Directly `.await`s the result.
    /// - Converts `node::archive::ArchivedEvent` to
    ///   `model::archive::ArchivedEvent` using `Into::into`.
    /// - Maps errors to `ArchiveError`.
    async fn get_block_events_by_height(
        &self,
        block_height: u64,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        let client = Arc::clone(&self.archive_client);
        let node_events = client
            .fetch_events_by_height(block_height as i64)
            .await // Await the future
            .map_err(|node_err| {
                ArchiveError::QueryFailed(format!(
                    "Archive query failed fetching events by height: {}",
                    node_err
                ))
            })?;

        Ok(node_events.into_iter().map(Into::into).collect())
    }

    /// Fetches all archived VM events from the latest block known to the
    /// archive (regardless of finalization status).
    ///
    /// This is useful for getting the most recent events indexed by the
    /// archive, which might include events from blocks not yet marked as
    /// finalized.
    ///
    /// # Implementation Note
    ///
    /// This method is typically implemented by first calling
    /// [`get_last_archived_block`](ArchiveAdapter::get_last_archived_block) to
    /// find the latest height and then calling
    /// [`get_block_events_by_height`](ArchiveAdapter::get_block_events_by_height)
    /// with that height.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ArchivedEvent>)`: A vector containing all archived events for
    ///   the latest block found in the archive. Returns an empty vector if the
    ///   latest block has no events.
    /// * `Err(ArchiveError)`: If fetching the last block height or fetching
    ///   events by height fails.
    ///
    /// # Implementation details:
    /// - Calls `self.get_last_archived_block()` to get the latest height.
    /// - Calls `self.get_block_events_by_height()` with the obtained height.
    /// - Relies on the error handling and conversion of the called methods.
    async fn get_latest_block_events(
        &self,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        // Implementation uses get_last_archived_block +
        // get_block_events_by_height
        let (latest_height, _) = self.get_last_archived_block().await?;
        self.get_block_events_by_height(latest_height).await
    }

    /// Fetches all **finalized** VM events emitted by a specific contract ID.
    ///
    /// This retrieves only events from blocks that are marked as finalized
    /// within the archive.
    ///
    /// # Arguments
    ///
    /// * `contract_id`: The identifier string of the contract (e.g., a
    ///   hex-encoded ID).
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ArchivedEvent>)`: A vector containing all finalized events
    ///   emitted by the specified contract. Returns an empty vector if the
    ///   contract has emitted no finalized events or is not found.
    /// * `Err(ArchiveError::QueryFailed)`: If the query fails due to database
    ///   issues.
    ///
    /// # Implementation details:
    /// - Calls the asynchronous
    ///   `node::archive::Archive::fetch_finalized_events_from_contract`.
    /// - Directly `.await`s the result.
    /// - Converts `node::archive::ArchivedEvent` to
    ///   `model::archive::ArchivedEvent` using `Into::into`.
    /// - Maps errors to `ArchiveError`.
    async fn get_contract_finalized_events(
        &self,
        contract_id: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        let client = Arc::clone(&self.archive_client);
        let contract_id_str = contract_id.to_string(); // Clone for closure
        let node_events = client
            .fetch_finalized_events_from_contract(&contract_id_str)
            .await // Await the future
            .map_err(|node_err| {
                ArchiveError::QueryFailed(format!(
                    "Archive query failed fetching finalized contract events: {}",
                    node_err
                ))
            })?;

        Ok(node_events.into_iter().map(Into::into).collect())
    }

    /// contains at least one Phoenix transaction.
    ///
    /// Phoenix transactions are a specific type within the Dusk ecosystem.
    /// This query helps in navigating the chain based on the presence of these
    /// transactions.
    ///
    /// # Arguments
    ///
    /// * `block_height`: The height *after* which to start searching for a
    ///   block containing a Phoenix transaction.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(u64))`: The height of the next block containing a Phoenix
    ///   transaction.
    /// * `Ok(None)`: If no subsequent block contains a Phoenix transaction.
    /// * `Err(ArchiveError::QueryFailed)`: If the query fails due to database
    ///   issues.
    ///
    /// # Implementation details:
    /// - Calls the asynchronous `node::archive::Archive::next_phoenix`.
    /// - Casts the input `block_height: u64` to `i64` for the node method.
    /// - Directly `.await`s the result.
    /// - Maps errors to `ArchiveError`.
    async fn get_next_block_with_phoenix_transaction(
        &self,
        block_height: u64,
    ) -> Result<Option<u64>, ArchiveError> {
        let client = Arc::clone(&self.archive_client);
        client
            .next_phoenix(block_height as i64)
            .await
            .map_err(|node_err| {
                ArchiveError::QueryFailed(format!(
                    "Archive query failed fetching next phoenix block: {}",
                    node_err
                ))
            })
    }

    /// Fetches the full Moonlight transaction history for a given account,
    /// identified by its public key.
    ///
    /// Allows filtering by block range and specifying the order of results.
    /// Moonlight history includes transactions where the given account was
    /// either the sender or the receiver.
    ///
    /// # Arguments
    ///
    /// * `pk_bs58`: The Base58 encoded public key string of the account.
    /// * `ord`: An optional [`Order`](model::archive::Order) enum specifying
    ///   whether to sort results `Ascending` or `Descending` by block height.
    ///   Defaults typically to descending (newest first) if `None`.
    /// * `from_block`: An optional block height to start the history from
    ///   (inclusive).
    /// * `to_block`: An optional block height to end the history at
    ///   (inclusive).
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Vec<MoonlightEventGroup>))`: If history is found for the
    ///   account within the specified range, returns a vector of event groups,
    ///   sorted according to `ord`.
    /// * `Ok(None)`: If no Moonlight transaction history is found for the
    ///   account in the specified range.
    /// * `Err(ArchiveError::QueryFailed)`: If the input `pk_bs58` is invalid,
    ///   if the query fails due to database issues, or other internal errors.
    ///
    /// # Implementation details:
    /// - Handles Base58 decoding errors.
    /// - Handles public key decoding errors.
    /// - Converts `model::archive::Order` to `NodeOrder`.
    /// - Calls the asynchronous
    ///   `node::archive::Archive::full_moonlight_history`.
    /// - Converts `node::archive::MoonlightEventGroup` to
    ///   `model::archive::MoonlightEventGroup`.
    /// - Maps errors to `ArchiveError`.
    async fn get_moonlight_transaction_history(
        &self,
        pk_bs58: String,
        ord: Option<model::archive::Order>,
        from_block: Option<u64>,
        to_block: Option<u64>,
    ) -> Result<Option<Vec<model::archive::MoonlightEventGroup>>, ArchiveError>
    {
        // Explicitly handle Base58 decoding error
        let pk_bytes = match bs58::decode(&pk_bs58).into_vec() {
            Ok(bytes) => bytes,
            Err(e) => {
                return Err(ArchiveError::QueryFailed(format!(
                    "Invalid Base58 public key: {}",
                    e
                )));
            }
        };

        // Explicitly handle public key decoding error
        let node_pk = match NodePublicKey::from_slice(&pk_bytes) {
            Ok(pk) => pk,
            Err(e) => {
                return Err(ArchiveError::QueryFailed(format!(
                    "Invalid public key bytes: {:?}",
                    e
                )));
            }
        };

        let node_ord: Option<NodeOrder> =
            ord.map(|model_order| match model_order {
                model::archive::Order::Ascending => NodeOrder::Ascending,
                model::archive::Order::Descending => NodeOrder::Descending,
            });
        let client = Arc::clone(&self.archive_client);
        let result = tokio::task::spawn_blocking(move || {
            client
                .full_moonlight_history(node_pk, node_ord, from_block, to_block)
        })
        .await
        .map_err(|e| {
            ArchiveError::InternalError(format!("Task join error: {}", e))
        })?;
        result
            .map(|opt_node_groups| {
                opt_node_groups.map(|node_groups| {
                    node_groups
                        .into_iter()
                        .map(MoonlightEventGroup::from)
                        .collect()
                })
            })
            .map_err(|node_err| {
                ArchiveError::QueryFailed(format!(
                    "Archive query failed fetching moonlight history: {}",
                    node_err
                ))
            })
    }
}
