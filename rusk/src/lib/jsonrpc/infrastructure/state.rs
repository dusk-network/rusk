// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # Application State Management for JSON-RPC Server
//!
//! This module defines the `AppState` struct, which serves as the central
//! container for shared resources and configuration required by JSON-RPC method
//! handlers and potentially other web framework handlers (like `axum`). It
//! ensures that components like configuration, database access
//! (`DatabaseAdapter`), archive access (`ArchiveAdapter`), subscription
//! management, metrics, and rate limiting are accessible in a thread-safe
//! manner throughout the application.
//!
//! ## Design
//!
//! - `AppState` uses `Arc` for shared ownership of immutable or thread-safe
//!   components (like `JsonRpcConfig`, `MetricsCollector`).
//! - It provides direct methods (e.g., `get_block_by_hash`,
//!   `broadcast_transaction`) that delegate calls to internal adapter
//!   implementations (`DatabaseAdapter`, `NetworkAdapter`, etc.). This hides
//!   the internal adapter structure.
//! - `Arc<RwLock>` is used for components requiring mutable access across
//!   threads (like `SubscriptionManager`).
//! - It implements `Clone`, allowing cheap cloning for sharing across tasks or
//!   handlers.
//! - `Send + Sync` are implicitly satisfied due to the use of `Arc`, `RwLock`,
//!   and the required bounds on the underlying adapters.
//!
//! ## Integration with Axum
//!
//! When used with the `axum` web framework, an instance of `AppState`
//! (typically wrapped in an `Arc`) is provided to the `axum::Router` using the
//! `.with_state()` method. Handlers can then access the shared state via the
//! `axum::extract::State` extractor.
//!
//! ```rust
//! // Example: Setting up Axum router with AppState
//! # use axum::{routing::get, Router, extract::State};
//! # use std::sync::Arc;
//! # use parking_lot::RwLock;
//! # use rusk::jsonrpc::infrastructure::state::AppState;
//! # use rusk::jsonrpc::infrastructure::subscription::manager::SubscriptionManager;
//! # use rusk::jsonrpc::config::JsonRpcConfig;
//! # use rusk::jsonrpc::infrastructure::db::DatabaseAdapter;
//! # use rusk::jsonrpc::infrastructure::archive::ArchiveAdapter;
//! # use rusk::jsonrpc::infrastructure::error::{ArchiveError, DbError, NetworkError, VmError};
//! # use rusk::jsonrpc::infrastructure::metrics::MetricsCollector;
//! # use rusk::jsonrpc::infrastructure::manual_limiter::ManualRateLimiters;
//! # use rusk::jsonrpc::model::block::Block;
//! # use rusk::jsonrpc::model::archive::{ArchivedEvent, Order, MoonlightEventGroup};
//! # use dusk_core::abi::ContractId;
//! # use dusk_bytes::{Serializable, DeserializableSlice};
//! # use async_trait::async_trait;
//! # use rusk::jsonrpc::infrastructure::network::NetworkAdapter;
//! # use rusk::jsonrpc::infrastructure::vm::VmAdapter;
//! # use dusk_consensus::user::{provisioners::Provisioners, stake::Stake};
//! # use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
//! # use dusk_core::transfer::moonlight::AccountData;
//! # use dusk_core::stake::{StakeData, StakeKeys};
//! # use std::net::SocketAddr;
//! # // --- Mock Implementations for Example ---
//! # #[derive(Debug, Clone)]
//! # struct MockDbAdapter;
//! # #[async_trait]
//! # impl DatabaseAdapter for MockDbAdapter {
//! #     // --- Required Primitives ---
//! #     async fn get_block_by_hash(&self, _: &str) -> Result<Option<rusk::jsonrpc::model::block::Block>, DbError> { Ok(None) }
//! #     async fn get_block_transactions_by_hash(&self, _: &str) -> Result<Option<Vec<rusk::jsonrpc::model::transaction::TransactionResponse>>, DbError> { Ok(None) }
//! #     async fn get_block_faults_by_hash(&self, _: &str) -> Result<Option<rusk::jsonrpc::model::block::BlockFaults>, DbError> { Ok(None) }
//! #     async fn get_block_hash_by_height(&self, _: u64) -> Result<Option<String>, DbError> { Ok(None) }
//! #     async fn get_block_header_by_hash(&self, _: &str) -> Result<Option<rusk::jsonrpc::model::block::BlockHeader>, DbError> { Ok(None) }
//! #     async fn get_block_label_by_height(&self, _: u64) -> Result<Option<rusk::jsonrpc::model::block::BlockLabel>, DbError> { Ok(None) }
//! #     async fn get_spent_transaction_by_hash(&self, _: &str) -> Result<Option<node_data::ledger::SpentTransaction>, DbError> { Ok(None) }
//! #     async fn ledger_tx_exists(&self, _: &[u8; 32]) -> Result<bool, DbError> { Ok(false) }
//! #     async fn mempool_tx(&self, _: [u8; 32]) -> Result<Option<node_data::ledger::Transaction>, DbError> { Ok(None) }
//! #     async fn mempool_tx_exists(&self, _: [u8; 32]) -> Result<bool, DbError> { Ok(false) }
//! #     async fn mempool_txs_sorted_by_fee(&self) -> Result<Vec<node_data::ledger::Transaction>, DbError> { Ok(vec![]) }
//! #     async fn mempool_txs_count(&self) -> Result<usize, DbError> { Ok(0) }
//! #     async fn mempool_txs_ids_sorted_by_fee(&self) -> Result<Vec<(u64, [u8; 32])>, DbError> { Ok(vec![]) }
//! #     async fn mempool_txs_ids_sorted_by_low_fee(&self) -> Result<Vec<(u64, [u8; 32])>, DbError> { Ok(vec![]) }
//! #     async fn candidate(&self, _: &[u8; 32]) -> Result<Option<node_data::ledger::Block>, DbError> { Ok(None) }
//! #     async fn candidate_by_iteration(&self, _: &node_data::message::ConsensusHeader) -> Result<Option<node_data::ledger::Block>, DbError> { Ok(None) }
//! #     async fn validation_result(&self, _: &node_data::message::ConsensusHeader) -> Result<Option<node_data::message::payload::ValidationResult>, DbError> { Ok(None) }
//! #     async fn metadata_op_read(&self, _: &[u8]) -> Result<Option<Vec<u8>>, DbError> { Ok(None) }
//! #     async fn metadata_op_write(&mut self, _: &[u8], _: &[u8]) -> Result<(), DbError> { Ok(()) }
//! # }
//! # #[derive(Debug, Clone)] struct MockArchiveAdapter;
//! # #[async_trait]
//! # impl ArchiveAdapter for MockArchiveAdapter {
//! #     async fn get_moonlight_txs_by_memo(&self, _memo: Vec<u8>) -> Result<Option<Vec<MoonlightEventGroup>>, ArchiveError> { Ok(Some(vec![])) }
//! #     async fn get_last_archived_block(&self) -> Result<(u64, String), ArchiveError> { Ok((42, "dummy_hash".to_string())) }
//! #     async fn get_block_events_by_hash(&self, _hex_block_hash: &str) -> Result<Vec<ArchivedEvent>, ArchiveError> { unimplemented!() }
//! #     async fn get_block_events_by_height(&self, _block_height: u64) -> Result<Vec<ArchivedEvent>, ArchiveError> { unimplemented!() }
//! #     async fn get_latest_block_events(&self) -> Result<Vec<ArchivedEvent>, ArchiveError> { unimplemented!() } // Can use default if desired
//! #     async fn get_contract_finalized_events(&self, _contract_id: &str) -> Result<Vec<ArchivedEvent>, ArchiveError> { unimplemented!() }
//! #     async fn get_next_block_with_phoenix_transaction(&self, _block_height: u64) -> Result<Option<u64>, ArchiveError> { unimplemented!() }
//! #     async fn get_moonlight_transaction_history(&self, _pk_bs58: String, _ord: Option<Order>, _from_block: Option<u64>, _to_block: Option<u64>) -> Result<Option<Vec<MoonlightEventGroup>>, ArchiveError> { unimplemented!() }
//! #     // Default methods like get_last_archived_block_height are implicitly included
//! # }
//! # #[derive(Debug, Clone)] struct MockNetworkAdapter; #[async_trait] impl NetworkAdapter for MockNetworkAdapter { async fn broadcast_transaction(&self, _tx: Vec<u8>) -> Result<(), NetworkError> { Ok(()) } async fn get_network_info(&self) -> Result<String, NetworkError> { Ok("MockNet".to_string()) } async fn get_public_address(&self) -> Result<std::net::SocketAddr, NetworkError> { Ok(([127,0,0,1], 8080).into()) } async fn get_alive_peers(&self, _max: usize) -> Result<Vec<std::net::SocketAddr>, NetworkError> { Ok(vec![]) } async fn get_alive_peers_count(&self) -> Result<usize, NetworkError> { Ok(0) } async fn flood_request(&self, _inv: node_data::message::payload::Inv, _ttl: Option<u64>, _hops: u16) -> Result<(), NetworkError> { Ok(()) }}
//! # #[derive(Debug, Clone)] struct MockVmAdapter; #[async_trait] impl VmAdapter for MockVmAdapter { async fn simulate_transaction(&self, _tx: Vec<u8>) -> Result<rusk::jsonrpc::model::transaction::SimulationResult, VmError> { unimplemented!() } async fn preverify_transaction(&self, _tx: Vec<u8>) -> Result<node::vm::PreverificationResult, VmError> { Ok(node::vm::PreverificationResult::Valid) } async fn get_provisioners(&self) -> Result<Vec<(StakeKeys, StakeData)>, VmError> { Ok(Vec::new()) } async fn get_stake_info_by_pk(&self, _pk: &BlsPublicKey) -> Result<Option<Stake>, VmError> { Ok(None) } async fn get_state_root(&self) -> Result<[u8; 32], VmError> { Ok([0; 32]) } async fn get_block_gas_limit(&self) -> Result<u64, VmError> { Ok(1000000) } async fn query_contract_raw(&self, _contract_id: dusk_core::abi::ContractId, _method: String, _base_commit: [u8; 32], _args_bytes: Vec<u8>) -> Result<Vec<u8>, VmError> { Ok(vec![]) } async fn get_vm_config(&self) -> Result<rusk::node::RuskVmConfig, VmError> { unimplemented!() } async fn get_chain_id(&self) -> Result<u8, VmError> { Ok(0) } async fn get_account_data(&self, _pk: &BlsPublicKey) -> Result<AccountData, VmError> { Ok(AccountData { balance: 0, nonce: 0 }) } }
//! # // --- End Mock Implementations ---
//! // Initialize components (using mocks for example)
//! let config = JsonRpcConfig::default();
//! let db_adapter: Arc<dyn DatabaseAdapter> = Arc::new(MockDbAdapter);
//! let archive_adapter: Arc<dyn ArchiveAdapter> = Arc::new(MockArchiveAdapter);
//! let network_adapter: Arc<dyn NetworkAdapter> = Arc::new(MockNetworkAdapter);
//! let vm_adapter: Arc<dyn VmAdapter> = Arc::new(MockVmAdapter);
//! let subscription_manager = SubscriptionManager::default();
//! let metrics_collector = MetricsCollector::default();
//! let rate_limit_config = Arc::new(config.rate_limit.clone());
//! let manual_rate_limiters = ManualRateLimiters::new(rate_limit_config)
//!     .expect("Failed to create manual rate limiters");
//!
//! let app_state = AppState::new(
//!     config.clone(),
//!     db_adapter,
//!     archive_adapter,
//!     network_adapter,
//!     vm_adapter,
//!     subscription_manager,
//!     metrics_collector,
//!     manual_rate_limiters,
//! );
//!
//! // Create the Axum router and provide the state
//! let app: Router<AppState> = Router::new()
//!     .route("/health", get(health_handler))
//!     // Add other routes...
//!     .with_state(app_state); // Pass AppState to the router
//!
//! // Example handler accessing the state
//! async fn health_handler(State(state): State<AppState>) -> &'static str {
//!     println!("Current config bind_address: {}", state.config().http.bind_address);
//!     // Access components via direct methods on state
//!     let _block = state.get_latest_block().await; // Example direct call
//!     let _peers = state.get_alive_peers(10).await;
//!     "OK"
//! }
//! // ...
//! ```
//!
//! ## Adapters and Dynamic Dispatch
//!
//! `AppState` holds internal adapter implementations (`NetworkAdapter`,
//! `VmAdapter`, etc.) and exposes their functionality through direct methods.
//! This uses **dynamic dispatch**, offering:
//!
//! 1. **Flexibility & Testability:** Easily swap implementations (e.g., real
//!    vs. mock adapters for testing) without changing `AppState`'s type or
//!    handler signatures.
//! 2. **Avoids Generic Propagation:** Prevents the need to thread generic type
//!    parameters through large parts of the codebase.
//! 3. **Simplified Usage:** Consumers interact with a single, concrete
//!    `AppState` type, calling methods like `state.get_block_by_hash(...)`
//!    directly.
//!
//! While there's a minor runtime overhead compared to static dispatch
//! (generics), the architectural benefits are significant for shared state
//! management.
//!
//! ## Adapters for Network and VM
//!
//! `AppState` holds internal adapter implementations (`NetworkAdapter`,
//! `VmAdapter`, etc.) and exposes their functionality through direct methods.
//!
//! Mocks for the adapters (e.g., `MockNetworkAdapter`, `MockVmAdapter`)
//! are defined in test utilities (`rusk/tests/jsonrpc/utils.rs`)
//! for testing JSON-RPC methods without a full node environment.
//!
//! For real usage, concrete adapter implementations like `RuskNetworkAdapter`
//! and `RuskVmAdapter` are instantiated and passed during `AppState` creation.

use std::{net::SocketAddr, sync::Arc};

use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_core::stake::StakeData;
use dusk_core::stake::STAKE_CONTRACT;
use dusk_core::{signatures::bls::PublicKey as BlsPublicKey, stake::StakeKeys};

use parking_lot::RwLock;

use crate::jsonrpc::config::JsonRpcConfig;
use crate::jsonrpc::error::Error as JsonRpcError;
use crate::jsonrpc::infrastructure::archive::ArchiveAdapter;
use crate::jsonrpc::infrastructure::db::DatabaseAdapter;
use crate::jsonrpc::infrastructure::error::{
    ArchiveError, DbError, Error as RpcInfraError, NetworkError, VmError,
};
use crate::jsonrpc::infrastructure::manual_limiter::ManualRateLimiters;
use crate::jsonrpc::infrastructure::metrics::MetricsCollector;
use crate::jsonrpc::infrastructure::network::NetworkAdapter;
use crate::jsonrpc::infrastructure::subscription::manager::SubscriptionManager;
use crate::jsonrpc::infrastructure::vm::VmAdapter;
use crate::jsonrpc::model::archive::{
    ArchivedEvent, MoonlightEventGroup, Order,
};
use crate::jsonrpc::model::block::Block;
use crate::jsonrpc::model::block::BlockFaults;
use crate::jsonrpc::model::provisioner::{ProvisionerInfo, StakeOwnerInfo};
use crate::jsonrpc::model::transaction::SimulationResult;
use crate::jsonrpc::model::transaction::TransactionResponse;

#[derive(Debug, Clone)]
pub struct AppState {
    /// Shared JSON-RPC server configuration.
    config: Arc<JsonRpcConfig>,

    /// Shared database adapter instance.
    /// Provides access to the live blockchain state via [`DatabaseAdapter`].
    db_adapter: Arc<dyn DatabaseAdapter>,

    /// Shared archive adapter instance.
    /// Provides access to historical/indexed data via [`ArchiveAdapter`].
    archive_adapter: Arc<dyn ArchiveAdapter>,

    /// Shared network adapter instance.
    /// Provides access to network operations (broadcast, peers) via
    /// [`NetworkAdapter`].
    network_adapter: Arc<dyn NetworkAdapter>,

    /// Shared VM adapter instance.
    /// Provides access to high-level VM operations (simulation, state queries)
    /// via [`VmAdapter`].
    vm_adapter: Arc<dyn VmAdapter>,

    /// Shared subscription manager for WebSocket event handling.
    /// Needs `RwLock` for managing mutable subscription state.
    subscription_manager: Arc<RwLock<SubscriptionManager>>,

    /// Shared metrics collector instance.
    metrics_collector: Arc<MetricsCollector>,

    /// Shared manual rate limiters for WebSockets and specific methods.
    manual_rate_limiters: Arc<ManualRateLimiters>,
}

impl AppState {
    /// Creates a new `AppState` instance.
    ///
    /// Initializes the shared state container with the provided configuration
    /// and infrastructure components. Components are wrapped in `Arc` or
    /// `Arc<RwLock>` to enable safe sharing across threads.
    ///
    /// # Arguments
    ///
    /// * `config` - The JSON-RPC server configuration.
    /// * `db_adapter` - An implementation of the [`DatabaseAdapter`] trait.
    /// * `archive_adapter` - An implementation of the [`ArchiveAdapter`] trait.
    /// * `network_adapter` - The network adapter implementation.
    /// * `vm_adapter` - The VM adapter implementation (`VMExecution`).
    /// * `subscription_manager` - The manager for WebSocket subscriptions.
    /// * `metrics_collector` - The collector for server metrics.
    /// * `manual_rate_limiters` - The manager for manual rate limiting.
    ///
    /// # Returns
    ///
    /// A new `AppState` instance ready to be shared.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: JsonRpcConfig,
        db_adapter: Arc<dyn DatabaseAdapter>,
        archive_adapter: Arc<dyn ArchiveAdapter>,
        network_adapter: Arc<dyn NetworkAdapter>,
        vm_adapter: Arc<dyn VmAdapter>,
        subscription_manager: SubscriptionManager,
        metrics_collector: MetricsCollector,
        manual_rate_limiters: ManualRateLimiters,
    ) -> Self {
        Self {
            config: Arc::new(config),
            db_adapter,
            archive_adapter,
            network_adapter,
            vm_adapter,
            subscription_manager: Arc::new(RwLock::new(subscription_manager)),
            metrics_collector: Arc::new(metrics_collector),
            manual_rate_limiters: Arc::new(manual_rate_limiters),
        }
    }

    /// Returns a reference to the shared JSON-RPC configuration.
    ///
    /// The configuration is wrapped in an `Arc`, allowing cheap cloning if
    /// needed.
    pub fn config(&self) -> &Arc<JsonRpcConfig> {
        &self.config
    }

    /// Returns a reference to the shared subscription manager.
    ///
    /// The manager is wrapped in `Arc<RwLock<SubscriptionManager>>`, allowing
    /// thread-safe read/write access to subscription state.
    pub fn subscription_manager(&self) -> &Arc<RwLock<SubscriptionManager>> {
        &self.subscription_manager
    }

    /// Returns a reference to the shared metrics collector.
    ///
    /// The collector is wrapped in an `Arc`.
    pub fn metrics_collector(&self) -> &Arc<MetricsCollector> {
        &self.metrics_collector
    }

    /// Returns a reference to the shared manual rate limiters.
    ///
    /// The limiters are wrapped in an `Arc`.
    pub fn manual_rate_limiters(&self) -> &Arc<ManualRateLimiters> {
        &self.manual_rate_limiters
    }

    // --- Delegated ArchiveAdapter Methods ---

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
    /// * `Err(JsonRpcError::Infrastructure)`: If the query fails due to
    ///   database issues or other internal errors.
    pub async fn get_moonlight_txs_by_memo(
        &self,
        memo: Vec<u8>,
    ) -> Result<Option<Vec<model::archive::MoonlightEventGroup>>, JsonRpcError>
    {
        self.archive_adapter
            .get_moonlight_txs_by_memo(memo)
            .await
            .map_err(RpcInfraError::Archive) // Map adapter error to RpcInfraError
            .map_err(JsonRpcError::Infrastructure) // Map RpcInfraError to
                                                   // JsonRpcError::Infrastructure
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
    /// * `Err(JsonRpcError::Infrastructure)`: If no finalized blocks are found
    ///   in the archive (e.g., during initial sync), or if the query fails due
    ///   to database issues.
    pub async fn get_last_archived_block(
        &self,
    ) -> Result<(u64, String), JsonRpcError> {
        self.archive_adapter
            .get_last_archived_block()
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
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
    /// * `Err(JsonRpcError::Infrastructure)`: If the query fails due to
    ///   database issues.
    pub async fn get_block_events_by_hash(
        &self,
        hex_block_hash: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_block_events_by_hash(hex_block_hash)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
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
    /// * `Err(JsonRpcError::Infrastructure)`: If the query fails due to
    ///   database issues.
    pub async fn get_block_events_by_height(
        &self,
        block_height: u64,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_block_events_by_height(block_height)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
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
    /// This method delegates to the underlying `ArchiveAdapter` which might
    /// first call `get_last_archived_block` to find the latest height and
    /// then call `get_block_events_by_height` with that height.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ArchivedEvent>)`: A vector containing all archived events for
    ///   the latest block found in the archive. Returns an empty vector if the
    ///   latest block has no events.
    /// * `Err(JsonRpcError::Infrastructure)`: If fetching the last block height
    ///   or fetching events by height fails in the underlying adapter.
    pub async fn get_latest_block_events(
        &self,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_latest_block_events()
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
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
    /// * `Err(JsonRpcError::Infrastructure)`: If the query fails due to
    ///   database issues.
    pub async fn get_contract_finalized_events(
        &self,
        contract_id: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_contract_finalized_events(contract_id)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

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
    /// * `Err(JsonRpcError::Infrastructure)`: If the query fails due to
    ///   database issues.
    pub async fn get_next_block_with_phoenix_transaction(
        &self,
        block_height: u64,
    ) -> Result<Option<u64>, JsonRpcError> {
        self.archive_adapter
            .get_next_block_with_phoenix_transaction(block_height)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
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
    /// * `Err(JsonRpcError::Infrastructure)`: If the input `pk_bs58` is
    ///   invalid, if the query fails due to database issues, or other internal
    ///   errors.
    pub async fn get_moonlight_transaction_history(
        &self,
        pk_bs58: String,
        ord: Option<Order>,
        from_block: Option<u64>,
        to_block: Option<u64>,
    ) -> Result<Option<Vec<MoonlightEventGroup>>, JsonRpcError> {
        self.archive_adapter
            .get_moonlight_transaction_history(
                pk_bs58, ord, from_block, to_block,
            )
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Fetches finalized events from a specific contract, filtered by event
    /// topic.
    ///
    /// This is a convenience method that delegates to the underlying adapter,
    /// which first calls `get_contract_finalized_events` and then filters the
    /// results based on the provided `topic` string.
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
    /// * `Err(JsonRpcError::Infrastructure)`: If the underlying call to
    ///   `get_contract_finalized_events` fails.
    pub async fn get_contract_events_by_topic(
        &self,
        contract_id: &str,
        topic: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_contract_events_by_topic(contract_id, topic)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Fetches the height of the last block finalized in the archive.
    ///
    /// A convenience method that delegates to the underlying adapter, which
    /// calls `get_last_archived_block` and extracts only the height
    /// component.
    ///
    /// # Returns
    ///
    /// * `Ok(u64)`: The height of the last finalized block in the archive.
    /// * `Err(JsonRpcError::Infrastructure)`: If the underlying call to
    ///   `get_last_archived_block` fails.
    pub async fn get_last_archived_block_height(
        &self,
    ) -> Result<u64, JsonRpcError> {
        self.archive_adapter
            .get_last_archived_block_height()
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Fetches all finalized events emitted by a specific contract.
    ///
    /// This is an alias for `get_contract_finalized_events` provided by the
    /// underlying adapter. It provides a potentially more intuitive name
    /// depending on the calling context.
    ///
    /// # Arguments
    ///
    /// * `contract_id`: The identifier string of the contract.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ArchivedEvent>)`: A vector containing all finalized events
    ///   emitted by the specified contract. Returns an empty vector if the
    ///   contract has emitted no finalized events or is not found.
    /// * `Err(JsonRpcError::Infrastructure)`: If the underlying call to
    ///   `get_contract_finalized_events` fails.
    ///
    /// See [`get_contract_finalized_events`](AppState::get_contract_finalized_events).
    pub async fn get_contract_events(
        &self,
        contract_id: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_contract_events(contract_id)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Fetches events from a specific block height, filtered by source contract
    /// ID.
    ///
    /// Delegates to the underlying adapter which calls
    /// `get_block_events_by_height` and filters the results, keeping only
    /// events where the `source` field matches the provided `contract_id`.
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
    /// * `Err(JsonRpcError::Infrastructure)`: If the underlying call to
    ///   `get_block_events_by_height` fails.
    pub async fn get_contract_events_by_block_height(
        &self,
        block_height: u64,
        contract_id: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_contract_events_by_block_height(block_height, contract_id)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Fetches events from a specific block hash, filtered by source contract
    /// ID.
    ///
    /// Delegates to the underlying adapter which calls
    /// `get_block_events_by_hash` and filters the results, keeping only events
    /// where the `source` field matches the provided `contract_id`.
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
    /// * `Err(JsonRpcError::Infrastructure)`: If the underlying call to
    ///   `get_block_events_by_hash` fails.
    pub async fn get_contract_events_by_block_hash(
        &self,
        hex_block_hash: &str,
        contract_id: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_contract_events_by_block_hash(hex_block_hash, contract_id)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Fetches finalized contract events considered as 'transactions' (alias
    /// for [`get_contract_events`](AppState::get_contract_events)).
    ///
    /// Provides an alternative naming convention where general contract events
    /// are referred to as transactions. Delegates to the underlying adapter.
    ///
    /// # Arguments
    ///
    /// * `contract_id`: The identifier string of the contract.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ArchivedEvent>)`: See
    ///   [`get_contract_events`](AppState::get_contract_events).
    /// * `Err(JsonRpcError::Infrastructure)`: If the underlying call fails.
    pub async fn get_contract_transactions(
        &self,
        contract_id: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_contract_transactions(contract_id)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Fetches finalized contract events from a specific block height
    /// considered as 'transactions' (alias for
    /// [`get_contract_events_by_block_height`](AppState::get_contract_events_by_block_height)).
    ///
    /// Provides an alternative naming convention. Delegates to the underlying
    /// adapter.
    ///
    /// # Arguments
    ///
    /// * `block_height`: The block height number.
    /// * `contract_id`: The identifier string of the source contract.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ArchivedEvent>)`: See
    ///   [`get_contract_events_by_block_height`](AppState::get_contract_events_by_block_height).
    /// * `Err(JsonRpcError::Infrastructure)`: If the underlying call fails.
    pub async fn get_contract_transactions_by_block_height(
        &self,
        block_height: u64,
        contract_id: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_contract_transactions_by_block_height(
                block_height,
                contract_id,
            )
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Fetches finalized contract events from a specific block hash considered
    /// as 'transactions' (alias for
    /// [`get_contract_events_by_block_hash`](AppState::get_contract_events_by_block_hash)).
    ///
    /// Provides an alternative naming convention. Delegates to the underlying
    /// adapter.
    ///
    /// # Arguments
    ///
    /// * `hex_block_hash`: The hex-encoded string of the block hash.
    /// * `contract_id`: The identifier string of the source contract.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ArchivedEvent>)`: See
    ///   [`get_contract_events_by_block_hash`](AppState::get_contract_events_by_block_hash).
    /// * `Err(JsonRpcError::Infrastructure)`: If the underlying call fails.
    pub async fn get_contract_transactions_by_block_hash(
        &self,
        hex_block_hash: &str,
        contract_id: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_contract_transactions_by_block_hash(
                hex_block_hash,
                contract_id,
            )
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    // --- Topic-Specific Event Getters ---

    /// Fetches finalized 'item added' events from a specific contract.
    ///
    /// This is a convenience method that delegates to the underlying adapter,
    /// which calls
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
    /// * `Ok(Vec<ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_item_added_events(
        &self,
        contract_id: &str,
        item_added_topic: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_item_added_events(contract_id, item_added_topic)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Fetches finalized 'item removed' events from a specific contract.
    ///
    /// This is a convenience method that delegates to the underlying adapter,
    /// which calls
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
    /// * `Ok(Vec<ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_item_removed_events(
        &self,
        contract_id: &str,
        item_removed_topic: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_item_removed_events(contract_id, item_removed_topic)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Fetches finalized 'item modified' events from a specific contract.
    ///
    /// This is a convenience method that delegates to the underlying adapter,
    /// which calls
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
    /// * `Ok(Vec<ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_item_modified_events(
        &self,
        contract_id: &str,
        item_modified_topic: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_item_modified_events(contract_id, item_modified_topic)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Fetches finalized 'stake' events from a specific contract.
    ///
    /// This is a convenience method that delegates to the underlying adapter,
    /// which calls
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
    /// * `Ok(Vec<ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_stake_events(
        &self,
        contract_id: &str,
        stake_topic: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_stake_events(contract_id, stake_topic)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Fetches finalized 'transfer' events from a specific contract (e.g.,
    /// "moonlight").
    ///
    /// This is a convenience method that delegates to the underlying adapter,
    /// which calls
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
    /// * `Ok(Vec<ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_transfer_events(
        &self,
        contract_id: &str,
        transfer_topic: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_transfer_events(contract_id, transfer_topic)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Fetches finalized 'unstake' events from a specific contract.
    ///
    /// This is a convenience method that delegates to the underlying adapter,
    /// which calls
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
    /// * `Ok(Vec<ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_unstake_events(
        &self,
        contract_id: &str,
        unstake_topic: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_unstake_events(contract_id, unstake_topic)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Fetches finalized 'slash' events from a specific contract.
    ///
    /// This is a convenience method that delegates to the underlying adapter,
    /// which calls
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
    /// * `Ok(Vec<ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_slash_events(
        &self,
        contract_id: &str,
        slash_topic: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_slash_events(contract_id, slash_topic)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Fetches finalized 'deposit' events from a specific contract.
    ///
    /// This is a convenience method that delegates to the underlying adapter,
    /// which calls
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
    /// * `Ok(Vec<ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_deposit_events(
        &self,
        contract_id: &str,
        deposit_topic: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_deposit_events(contract_id, deposit_topic)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Fetches finalized 'withdraw' events from a specific contract.
    ///
    /// This is a convenience method that delegates to the underlying adapter,
    /// which calls
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
    /// * `Ok(Vec<ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_withdraw_events(
        &self,
        contract_id: &str,
        withdraw_topic: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_withdraw_events(contract_id, withdraw_topic)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Fetches finalized 'convert' events from a specific contract.
    ///
    /// This is a convenience method that delegates to the underlying adapter,
    /// which calls
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
    /// * `Ok(Vec<ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_convert_events(
        &self,
        contract_id: &str,
        convert_topic: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_convert_events(contract_id, convert_topic)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Fetches finalized 'provisioner changes' events from a specific contract.
    ///
    /// This is a convenience method that delegates to the underlying adapter,
    /// which calls
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
    /// * `Ok(Vec<ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_provisioner_changes(
        &self,
        contract_id: &str,
        provisioner_changes_topic: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_provisioner_changes(contract_id, provisioner_changes_topic)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Fetches finalized 'hard slash' events from a specific contract.
    ///
    /// This is a convenience method that delegates to the underlying adapter,
    /// which calls
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
    /// * `Ok(Vec<ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_hard_slash_events(
        &self,
        contract_id: &str,
        hard_slash_topic: &str,
    ) -> Result<Vec<ArchivedEvent>, JsonRpcError> {
        self.archive_adapter
            .get_hard_slash_events(contract_id, hard_slash_topic)
            .await
            .map_err(RpcInfraError::Archive)
            .map_err(JsonRpcError::Infrastructure)
    }

    // --- Delegated DatabaseAdapter Methods ---

    /// Retrieves a block summary by its 32-byte hash.
    ///
    /// # Arguments
    /// * `block_hash_hex`: 64-char hex string of the block hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::block::Block>)`: if the block is found.
    /// * `Err(JsonRpcError::Infrastructure)`: if a database error occurs.
    pub async fn get_block_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<model::block::Block>, JsonRpcError> {
        self.db_adapter
            .get_block_by_hash(block_hash_hex)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves the list of full transactions for a block by hash.
    ///
    /// # Arguments
    /// * `block_hash_hex`: 64-char hex string of the block hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<Vec<TransactionResponse>>)`: if the transactions are found
    ///   for the block.
    /// * `Err(JsonRpcError::Infrastructure)`: if a database error occurs.
    pub async fn get_block_transactions_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<Vec<TransactionResponse>>, JsonRpcError> {
        self.db_adapter
            .get_block_transactions_by_hash(block_hash_hex)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves consensus faults for a block by hash.
    ///
    /// # Arguments
    /// * `block_hash_hex`: 64-char hex string of the block hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<BlockFaults>)`: if the faults are found for the block.
    /// * `Err(JsonRpcError::Infrastructure)`: if a database error occurs.
    pub async fn get_block_faults_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<BlockFaults>, JsonRpcError> {
        self.db_adapter
            .get_block_faults_by_hash(block_hash_hex)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves a block hash by its height.
    ///
    /// # Arguments
    ///
    /// * `height`: The block height.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<String>)`: The hex-encoded block hash if found.
    /// * `Err(JsonRpcError::Infrastructure)`: if a database error occurs.
    pub async fn get_block_hash_by_height(
        &self,
        height: u64,
    ) -> Result<Option<String>, JsonRpcError> {
        self.db_adapter
            .get_block_hash_by_height(height)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves a block header by its 32-byte hash.
    ///
    /// # Arguments
    /// * `block_hash_hex`: 64-char hex string of the block hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::block::BlockHeader>)`: if the header is found.
    /// * `Err(JsonRpcError::Infrastructure)`: if a database error occurs.
    pub async fn get_block_header_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<model::block::BlockHeader>, JsonRpcError> {
        self.db_adapter
            .get_block_header_by_hash(block_hash_hex)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves a block header by its height.
    ///
    /// # Arguments
    ///
    /// * `height`: The block height.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::block::BlockHeader>)`: if the header is found.
    /// * `Err(JsonRpcError::Infrastructure)`: if a database error occurs.
    pub async fn get_block_header_by_height(
        &self,
        height: u64,
    ) -> Result<Option<model::block::BlockHeader>, JsonRpcError> {
        self.db_adapter
            .get_block_header_by_height(height)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves the consensus label for a block by height.
    ///
    /// # Arguments
    ///
    /// * `height`: Height of the block.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::block::BlockLabel>)`: if the label is found.
    /// * `Err(JsonRpcError::Infrastructure)`: if a database error occurs.
    pub async fn get_block_label_by_height(
        &self,
        height: u64,
    ) -> Result<Option<model::block::BlockLabel>, JsonRpcError> {
        self.db_adapter
            .get_block_label_by_height(height)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }
}
