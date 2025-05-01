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
//! # use std::net::SocketAddr;
//! # use std::fmt::Debug;
//! # use async_trait::async_trait;
//! # use parking_lot::RwLock;
//! # use rusk::jsonrpc::infrastructure::state::AppState;
//! # use rusk::jsonrpc::infrastructure::subscription::manager::SubscriptionManager;
//! # use rusk::jsonrpc::config::JsonRpcConfig;
//! # use rusk::jsonrpc::infrastructure::db::DatabaseAdapter;
//! # use rusk::jsonrpc::infrastructure::archive::ArchiveAdapter;
//! # use rusk::jsonrpc::infrastructure::network::NetworkAdapter;
//! # use rusk::jsonrpc::infrastructure::vm::VmAdapter;
//! # use rusk::jsonrpc::infrastructure::error::{ArchiveError, DbError, NetworkError, VmError};
//! # use rusk::jsonrpc::infrastructure::metrics::MetricsCollector;
//! # use rusk::jsonrpc::infrastructure::manual_limiter::ManualRateLimiters;
//! # use rusk::jsonrpc::model;
//! # use dusk_core::abi::ContractId;
//! # use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
//! # use node_data::message::ConsensusHeader;
//! # use node_data::message::payload::Inv;
//! # // --- Mock Implementations for Example ---
//! # #[derive(Debug, Clone)]
//! # struct MockDbAdapter;
//! # #[async_trait]
//! # impl DatabaseAdapter for MockDbAdapter {
//! #     // --- Required Ledger Primitives ---
//! #     async fn get_block_by_hash(&self, _: &str) -> Result<Option<rusk::jsonrpc::model::block::Block>, DbError> { Ok(None) }
//! #     async fn get_block_transactions_by_hash(&self, _: &str) -> Result<Option<Vec<rusk::jsonrpc::model::transaction::TransactionResponse>>, DbError> { Ok(None) }
//! #     async fn get_block_faults_by_hash(&self, _: &str) -> Result<Option<rusk::jsonrpc::model::block::BlockFaults>, DbError> { Ok(None) }
//! #     async fn get_block_hash_by_height(&self, _: u64) -> Result<Option<String>, DbError> { Ok(None) }
//! #     async fn get_block_header_by_hash(&self, _: &str) -> Result<Option<rusk::jsonrpc::model::block::BlockHeader>, DbError> { Ok(None) }
//! #     async fn get_block_label_by_height(&self, _: u64) -> Result<Option<rusk::jsonrpc::model::block::BlockLabel>, DbError> { Ok(None) }
//! #     async fn get_spent_transaction_by_hash(&self, _: &str) -> Result<Option<rusk::jsonrpc::model::transaction::TransactionInfo>, DbError> { Ok(None) }
//! #     async fn ledger_tx_exists(&self, _: &[u8; 32]) -> Result<bool, DbError> { Ok(false) }
//! #     async fn get_block_finality_status(&self, _: &str) -> Result<rusk::jsonrpc::model::block::BlockFinalityStatus, DbError> { Ok(rusk::jsonrpc::model::block::BlockFinalityStatus::Unknown) }
//! #     // --- Required Mempool Primitives ---
//! #     async fn mempool_tx(&self, _: [u8; 32]) -> Result<Option<rusk::jsonrpc::model::transaction::TransactionResponse>, DbError> { Ok(None) }
//! #     async fn mempool_tx_exists(&self, _: [u8; 32]) -> Result<bool, DbError> { Ok(false) }
//! #     async fn mempool_txs_sorted_by_fee(&self) -> Result<Vec<rusk::jsonrpc::model::transaction::TransactionResponse>, DbError> { Ok(vec![]) }
//! #     async fn mempool_txs_count(&self) -> Result<usize, DbError> { Ok(0) }
//! #     async fn mempool_txs_ids_sorted_by_fee(&self) -> Result<Vec<(u64, [u8; 32])>, DbError> { Ok(vec![]) }
//! #     async fn mempool_txs_ids_sorted_by_low_fee(&self) -> Result<Vec<(u64, [u8; 32])>, DbError> { Ok(vec![]) }
//! #     // --- Required ConsensusStorage Primitives ---
//! #     async fn candidate(&self, _: &[u8; 32]) -> Result<Option<rusk::jsonrpc::model::block::CandidateBlock>, DbError> { Ok(None) }
//! #     async fn candidate_by_iteration(&self, _: &ConsensusHeader) -> Result<Option<rusk::jsonrpc::model::block::CandidateBlock>, DbError> { Ok(None) }
//! #     async fn validation_result(&self, _: &ConsensusHeader) -> Result<Option<rusk::jsonrpc::model::consensus::ValidationResult>, DbError> { Ok(None) }
//! #     // --- Required Metadata Primitives ---
//! #     async fn metadata_op_read(&self, _: &[u8]) -> Result<Option<Vec<u8>>, DbError> { Ok(None) }
//! #     async fn metadata_op_write(&self, _: &[u8], _: &[u8]) -> Result<(), DbError> { Ok(()) }
//! # }
//! # #[derive(Debug, Clone)] struct MockArchiveAdapter;
//! # #[async_trait]
//! # impl ArchiveAdapter for MockArchiveAdapter {
//! #     async fn get_moonlight_txs_by_memo(&self, _memo: Vec<u8>) -> Result<Option<Vec<model::archive::MoonlightEventGroup>>, ArchiveError> { Ok(Some(vec![])) }
//! #     async fn get_last_archived_block(&self) -> Result<(u64, String), ArchiveError> { Ok((42, "dummy_hash".to_string())) }
//! #     async fn get_block_events_by_hash(&self, _hex_block_hash: &str) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> { Ok(vec![]) }
//! #     async fn get_block_events_by_height(&self, _block_height: u64) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> { Ok(vec![]) }
//! #     async fn get_contract_finalized_events(&self, _contract_id: &str) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> { Ok(vec![]) }
//! #     async fn get_next_block_with_phoenix_transaction(&self, _block_height: u64) -> Result<Option<u64>, ArchiveError> { Ok(None) }
//! #     async fn get_moonlight_transaction_history(&self, _pk_bs58: String, _ord: Option<model::archive::Order>, _from_block: Option<u64>, _to_block: Option<u64>) -> Result<Option<Vec<model::archive::MoonlightEventGroup>>, ArchiveError> { Ok(None) }
//! #     async fn get_latest_block_events(&self) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> { Ok(vec![]) }
//! # }
//! # #[derive(Debug, Clone)] struct MockNetworkAdapter;
//! # #[async_trait]
//! # impl NetworkAdapter for MockNetworkAdapter {
//! #     async fn broadcast_transaction(&self, _tx_bytes: Vec<u8>) -> Result<(), NetworkError> { Ok(()) }
//! #     async fn get_network_info(&self) -> Result<String, NetworkError> { Ok("MockNet".to_string()) }
//! #     async fn get_public_address(&self) -> Result<SocketAddr, NetworkError> { Ok(([127,0,0,1], 8080).into()) }
//! #     async fn get_alive_peers(&self, _max_peers: usize) -> Result<Vec<SocketAddr>, NetworkError> { Ok(vec![]) }
//! #     async fn get_alive_peers_count(&self) -> Result<usize, NetworkError> { Ok(0) }
//! #     async fn flood_request(&self, _inv: Inv, _ttl_seconds: Option<u64>, _hops: u16) -> Result<(), NetworkError> { Ok(()) }
//! #     async fn get_network_peers_location(&self) -> Result<Vec<model::network::PeerLocation>, NetworkError> { Ok(vec![]) }
//! # }
//! # #[derive(Debug, Clone)] struct MockVmAdapter;
//! # #[async_trait]
//! # impl VmAdapter for MockVmAdapter {
//! #     async fn simulate_transaction(&self, _tx_bytes: Vec<u8>) -> Result<model::transaction::SimulationResult, VmError> { Ok(model::transaction::SimulationResult{ success: true, gas_estimate: Some(100), error: None }) }
//! #     async fn preverify_transaction(&self, _tx_bytes: Vec<u8>) -> Result<model::vm::VmPreverificationResult, VmError> { Ok(model::vm::VmPreverificationResult::Valid) }
//! #     async fn get_chain_id(&self) -> Result<u8, VmError> { Ok(0) }
//! #     async fn get_account_data(&self, _pk: &BlsPublicKey) -> Result<model::account::AccountInfo, VmError> { Ok(model::account::AccountInfo { balance: 0, nonce: 0 }) }
//! #     async fn get_state_root(&self) -> Result<[u8; 32], VmError> { Ok([0; 32]) }
//! #     async fn get_block_gas_limit(&self) -> Result<u64, VmError> { Ok(1000000) }
//! #     async fn get_provisioners(&self) -> Result<Vec<(model::provisioner::ProvisionerKeys, model::provisioner::ProvisionerStakeData)>, VmError> { Ok(Vec::new()) }
//! #     async fn get_stake_info_by_pk(&self, _pk: &BlsPublicKey) -> Result<Option<model::provisioner::ConsensusStakeInfo>, VmError> { Ok(None) }
//! #     async fn query_contract_raw(&self, _contract_id: ContractId, _method: String, _base_commit: [u8; 32], _args_bytes: Vec<u8>) -> Result<Vec<u8>, VmError> { Ok(vec![]) }
//! #     async fn get_vm_config(&self) -> Result<model::vm::VmConfig, VmError> { unimplemented!() } // Assuming VmConfig has defaults or is simple
//! #     async fn validate_nullifiers(&self, _nullifiers: &[[u8; 32]]) -> Result<Vec<[u8; 32]>, VmError> { Ok(vec![]) }
//! # }
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
//! let app: Router = Router::new()
//!     .route("/health", get(health_handler))
//!     // Add other routes...
//!     .with_state(app_state.clone()); // Pass AppState to the router
//!
//! // Example handler accessing the state
//! async fn health_handler(State(state): State<AppState>) -> &'static str {
//!     println!("Current config bind_address: {}", state.config().http.bind_address);
//!     // Access components via direct methods on state
//!     // Example: Call a method that delegates to the DB adapter
//!     match state.get_block_by_height(100).await {
//!         Ok(Some(block)) => println!("Found block: {}", block.header.hash),
//!         Ok(None) => println!("Block 100 not found."),
//!         Err(e) => println!("Error getting block: {}", e),
//!     };
//!     // Example: Call a method that delegates to the Network adapter
//!     let _peers = state.get_alive_peers(10).await;
//!     "OK"
//! }
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

use std::sync::Arc;

use parking_lot::RwLock;
use std::collections::HashSet;

use dusk_core::abi::ContractId;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use hex;
use node_data::message::payload::Inv;

use crate::jsonrpc::config::JsonRpcConfig;
use crate::jsonrpc::error::Error as JsonRpcError;
use crate::jsonrpc::infrastructure::error::{Error as RpcInfraError, VmError};
use crate::jsonrpc::infrastructure::{
    archive, db, manual_limiter, metrics, network, subscription, vm,
};
use crate::jsonrpc::model;

#[derive(Debug, Clone)]
pub struct AppState {
    /// Shared JSON-RPC server configuration.
    config: Arc<JsonRpcConfig>,

    /// Shared database adapter instance.
    /// Provides access to the live blockchain state via [`DatabaseAdapter`].
    db_adapter: Arc<dyn db::DatabaseAdapter>,

    /// Shared archive adapter instance.
    /// Provides access to historical/indexed data via [`ArchiveAdapter`].
    archive_adapter: Arc<dyn archive::ArchiveAdapter>,

    /// Shared network adapter instance.
    /// Provides access to network operations (broadcast, peers) via
    /// [`NetworkAdapter`].
    network_adapter: Arc<dyn network::NetworkAdapter>,

    /// Shared VM adapter instance.
    /// Provides access to high-level VM operations (simulation, state queries)
    /// via [`VmAdapter`].
    vm_adapter: Arc<dyn vm::VmAdapter>,

    /// Shared subscription manager for WebSocket event handling.
    /// Needs `RwLock` for managing mutable subscription state.
    subscription_manager:
        Arc<RwLock<subscription::manager::SubscriptionManager>>,

    /// Shared metrics collector instance.
    metrics_collector: Arc<metrics::MetricsCollector>,

    /// Shared manual rate limiters for WebSockets and specific methods.
    manual_rate_limiters: Arc<manual_limiter::ManualRateLimiters>,
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
        db_adapter: Arc<dyn db::DatabaseAdapter>,
        archive_adapter: Arc<dyn archive::ArchiveAdapter>,
        network_adapter: Arc<dyn network::NetworkAdapter>,
        vm_adapter: Arc<dyn vm::VmAdapter>,
        subscription_manager: subscription::manager::SubscriptionManager,
        metrics_collector: metrics::MetricsCollector,
        manual_rate_limiters: manual_limiter::ManualRateLimiters,
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
    /// The manager is wrapped in
    /// `Arc<RwLock<infrastructure::subscription::manager::SubscriptionManager>>`,
    /// allowing thread-safe read/write access to subscription state.
    pub fn subscription_manager(
        &self,
    ) -> &Arc<RwLock<subscription::manager::SubscriptionManager>> {
        &self.subscription_manager
    }

    /// Returns a reference to the shared metrics collector.
    ///
    /// The collector is wrapped in an `Arc`.
    pub fn metrics_collector(&self) -> &Arc<metrics::MetricsCollector> {
        &self.metrics_collector
    }

    /// Returns a reference to the shared manual rate limiters.
    ///
    /// The limiters are wrapped in an `Arc`.
    pub fn manual_rate_limiters(
        &self,
    ) -> &Arc<manual_limiter::ManualRateLimiters> {
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
    /// * `Ok(Some(Vec<model::archive::MoonlightEventGroup>))`: If transactions
    ///   with the given memo are found, returns a vector of corresponding event
    ///   groups.
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: A vector containing all
    ///   archived events for the specified block. Returns an empty vector if
    ///   the block is found but has no associated events, or if the block hash
    ///   is not found in the archive.
    /// * `Err(JsonRpcError::Infrastructure)`: If the query fails due to
    ///   database issues.
    pub async fn get_block_events_by_hash(
        &self,
        hex_block_hash: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: A vector containing all
    ///   archived events for the specified block height. Returns an empty
    ///   vector if the block is found but has no associated events, or if the
    ///   block height is not found in the archive.
    /// * `Err(JsonRpcError::Infrastructure)`: If the query fails due to
    ///   database issues.
    pub async fn get_block_events_by_height(
        &self,
        block_height: u64,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: A vector containing all
    ///   archived events for the latest block found in the archive. Returns an
    ///   empty vector if the latest block has no events.
    /// * `Err(JsonRpcError::Infrastructure)`: If fetching the last block height
    ///   or fetching events by height fails in the underlying adapter.
    pub async fn get_latest_block_events(
        &self,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: A vector containing all
    ///   finalized events emitted by the specified contract. Returns an empty
    ///   vector if the contract has emitted no finalized events or is not
    ///   found.
    /// * `Err(JsonRpcError::Infrastructure)`: If the query fails due to
    ///   database issues.
    pub async fn get_contract_finalized_events(
        &self,
        contract_id: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Some(Vec<model::archive::MoonlightEventGroup>))`: If history is
    ///   found for the account within the specified range, returns a vector of
    ///   event groups, sorted according to `ord`.
    /// * `Ok(None)`: If no Moonlight transaction history is found for the
    ///   account in the specified range.
    /// * `Err(JsonRpcError::Infrastructure)`: If the input `pk_bs58` is
    ///   invalid, if the query fails due to database issues, or other internal
    ///   errors.
    pub async fn get_moonlight_transaction_history(
        &self,
        pk_bs58: String,
        ord: Option<model::archive::Order>,
        from_block: Option<u64>,
        to_block: Option<u64>,
    ) -> Result<Option<Vec<model::archive::MoonlightEventGroup>>, JsonRpcError>
    {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: A vector containing
    ///   finalized events from the contract that match the specified topic.
    ///   Returns an empty vector if no matching events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: If the underlying call to
    ///   `get_contract_finalized_events` fails.
    pub async fn get_contract_events_by_topic(
        &self,
        contract_id: &str,
        topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: A vector containing all
    ///   finalized events emitted by the specified contract. Returns an empty
    ///   vector if the contract has emitted no finalized events or is not
    ///   found.
    /// * `Err(JsonRpcError::Infrastructure)`: If the underlying call to
    ///   `get_contract_finalized_events` fails.
    ///
    /// See [`get_contract_finalized_events`](AppState::get_contract_finalized_events).
    pub async fn get_contract_events(
        &self,
        contract_id: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: A vector containing events
    ///   from the specified block height whose source matches the
    ///   `contract_id`. Returns an empty vector if no matching events are
    ///   found.
    /// * `Err(JsonRpcError::Infrastructure)`: If the underlying call to
    ///   `get_block_events_by_height` fails.
    pub async fn get_contract_events_by_block_height(
        &self,
        block_height: u64,
        contract_id: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: A vector containing events
    ///   from the specified block hash whose source matches the `contract_id`.
    ///   Returns an empty vector if no matching events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: If the underlying call to
    ///   `get_block_events_by_hash` fails.
    pub async fn get_contract_events_by_block_hash(
        &self,
        hex_block_hash: &str,
        contract_id: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: See
    ///   [`get_contract_events`](AppState::get_contract_events).
    /// * `Err(JsonRpcError::Infrastructure)`: If the underlying call fails.
    pub async fn get_contract_transactions(
        &self,
        contract_id: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: See
    ///   [`get_contract_events_by_block_height`](AppState::get_contract_events_by_block_height).
    /// * `Err(JsonRpcError::Infrastructure)`: If the underlying call fails.
    pub async fn get_contract_transactions_by_block_height(
        &self,
        block_height: u64,
        contract_id: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: See
    ///   [`get_contract_events_by_block_hash`](AppState::get_contract_events_by_block_hash).
    /// * `Err(JsonRpcError::Infrastructure)`: If the underlying call fails.
    pub async fn get_contract_transactions_by_block_hash(
        &self,
        hex_block_hash: &str,
        contract_id: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_item_added_events(
        &self,
        contract_id: &str,
        item_added_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_item_removed_events(
        &self,
        contract_id: &str,
        item_removed_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_item_modified_events(
        &self,
        contract_id: &str,
        item_modified_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_stake_events(
        &self,
        contract_id: &str,
        stake_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_transfer_events(
        &self,
        contract_id: &str,
        transfer_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_unstake_events(
        &self,
        contract_id: &str,
        unstake_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_slash_events(
        &self,
        contract_id: &str,
        slash_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_deposit_events(
        &self,
        contract_id: &str,
        deposit_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_withdraw_events(
        &self,
        contract_id: &str,
        withdraw_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_convert_events(
        &self,
        contract_id: &str,
        convert_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_provisioner_changes(
        &self,
        contract_id: &str,
        provisioner_changes_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Vec<model::archive::ArchivedEvent>)`: if the events are found.
    /// * `Err(JsonRpcError::Infrastructure)`: if an error occurs in the
    ///   underlying adapter.
    ///
    /// See [`get_contract_events_by_topic`](AppState::get_contract_events_by_topic).
    pub async fn get_hard_slash_events(
        &self,
        contract_id: &str,
        hard_slash_topic: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, JsonRpcError> {
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
    /// * `Ok(Option<Vec<model::transaction::TransactionResponse>>)`: if the
    ///   transactions are found for the block.
    /// * `Err(JsonRpcError::Infrastructure)`: if a database error occurs.
    pub async fn get_block_transactions_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<
        Option<Vec<model::transaction::TransactionResponse>>,
        JsonRpcError,
    > {
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
    /// * `Ok(Option<model::block::BlockFaults>)`: if the faults are found for
    ///   the block.
    /// * `Err(JsonRpcError::Infrastructure)`: if a database error occurs.
    pub async fn get_block_faults_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<model::block::BlockFaults>, JsonRpcError> {
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
    /// * `height`: Height of the block.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::block::BlockHeader>)` if the header is found for the
    ///   given height.
    /// * `Err(jsonrpc::error::Error)` if a database error occurs during hash or
    ///   header lookup.
    pub async fn get_block_header_by_height(
        &self,
        height: u64,
    ) -> Result<Option<model::block::BlockHeader>, JsonRpcError> {
        self.db_adapter // Corrected: Use field access
            .get_block_header_by_height(height)
            .await
            .map_err(RpcInfraError::Database) // Reverted: Explicit chain
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

    /// Retrieves a spent transaction record by its hash, returning detailed
    /// info.
    ///
    /// # Arguments
    ///
    /// * `tx_hash_hex`: 64-char hex string of the transaction hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::transaction::TransactionInfo>)`: if the transaction
    ///   is found.
    /// * `Err(JsonRpcError::Infrastructure)`: if a database error occurs.
    pub async fn get_spent_transaction_by_hash(
        &self,
        tx_hash_hex: &str,
    ) -> Result<Option<model::transaction::TransactionInfo>, JsonRpcError> {
        self.db_adapter
            .get_spent_transaction_by_hash(tx_hash_hex)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Checks if a transaction exists in the confirmed ledger.
    ///
    /// # Arguments
    ///
    /// * `tx_id`: 32-byte transaction hash.
    ///
    /// # Returns
    ///
    /// * `Ok(bool)`: true if the transaction exists, false otherwise.
    /// * `Err(JsonRpcError::Infrastructure)`: if a database error occurs.
    pub async fn ledger_tx_exists(
        &self,
        tx_id: &[u8; 32],
    ) -> Result<bool, JsonRpcError> {
        self.db_adapter
            .ledger_tx_exists(tx_id)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves a transaction from the mempool by its hash.
    ///
    /// # Arguments
    ///
    /// * `tx_id`: 32-byte transaction hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::transaction::TransactionResponse>)`: if the
    ///   transaction is found.
    /// * `Err(JsonRpcError::Infrastructure)`: if a database error occurs.
    pub async fn mempool_tx(
        &self,
        tx_id: [u8; 32],
    ) -> Result<Option<model::transaction::TransactionResponse>, JsonRpcError>
    {
        self.db_adapter
            .mempool_tx(tx_id)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Checks if a transaction exists in the mempool.
    ///
    /// # Arguments
    ///
    /// * `tx_id`: 32-byte transaction hash.
    ///
    /// # Returns
    ///
    /// * `Ok(bool)`: true if the transaction exists, false otherwise.
    /// * `Err(JsonRpcError::Infrastructure)`: if a database error occurs.
    pub async fn mempool_tx_exists(
        &self,
        tx_id: [u8; 32],
    ) -> Result<bool, JsonRpcError> {
        self.db_adapter
            .mempool_tx_exists(tx_id)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Gets transactions from the mempool, sorted by fee (highest first).
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<model::transaction::TransactionResponse>)`: the sorted mempool
    ///   transactions.
    /// * `Err(JsonRpcError::Infrastructure)`: if a database error occurs.
    pub async fn mempool_txs_sorted_by_fee(
        &self,
    ) -> Result<Vec<model::transaction::TransactionResponse>, JsonRpcError>
    {
        self.db_adapter
            .mempool_txs_sorted_by_fee()
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Gets the current count of transactions in the mempool.
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` if the count is found.
    /// * `Err(JsonRpcError::Infrastructure)`: if a database error occurs.
    pub async fn mempool_txs_count(&self) -> Result<usize, JsonRpcError> {
        self.db_adapter
            .mempool_txs_count()
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Gets an iterator over mempool (fee, tx_id) pairs, sorted by
    /// fee (highest first).
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<(u64, [u8; 32])>)`: if the iterator is found.
    /// * `Err(JsonRpcError::Infrastructure)`: if a database error occurs.
    pub async fn mempool_txs_ids_sorted_by_fee(
        &self,
    ) -> Result<Vec<(u64, [u8; 32])>, JsonRpcError> {
        self.db_adapter
            .mempool_txs_ids_sorted_by_fee()
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Gets an iterator over mempool (fee, tx_id) pairs, sorted by
    /// fee (lowest first).
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<(u64, [u8; 32])>)`: if the iterator is found.
    /// * `Err(JsonRpcError::Infrastructure)`: if a database error occurs.
    pub async fn mempool_txs_ids_sorted_by_low_fee(
        &self,
    ) -> Result<Vec<(u64, [u8; 32])>, JsonRpcError> {
        self.db_adapter
            .mempool_txs_ids_sorted_by_low_fee()
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    // --- Delegated ConsensusStorage Primitives ---

    /// Retrieves a candidate block by its hash.
    ///
    /// # Arguments
    /// * `hash`: 32-byte candidate block hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::block::CandidateBlock>)`: if found.
    /// * `Err(JsonRpcError::Infrastructure)`: if the identifier is invalid or a
    ///   database error occurs.
    pub async fn candidate(
        &self,
        hash: &[u8; 32],
    ) -> Result<Option<model::block::CandidateBlock>, JsonRpcError> {
        self.db_adapter
            .candidate(hash)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves a candidate block by its consensus header.
    ///
    /// # Arguments
    /// * `header`: Consensus header.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::block::CandidateBlock>)` if found.
    /// * `Err(JsonRpcError::Infrastructure)` if the identifier is invalid or a
    ///   database error occurs.
    pub async fn candidate_by_iteration(
        &self,
        header: &node_data::message::ConsensusHeader,
    ) -> Result<Option<model::block::CandidateBlock>, JsonRpcError> {
        self.db_adapter
            .candidate_by_iteration(header)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves a validation result by its consensus header.
    ///
    /// # Arguments
    ///
    /// * `header`: Consensus header.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::consensus::ValidationResult>)` if found.
    /// * `Err(JsonRpcError::Infrastructure)` if the identifier is invalid or a
    ///   database error occurs.
    pub async fn validation_result(
        &self,
        header: &node_data::message::ConsensusHeader,
    ) -> Result<Option<model::consensus::ValidationResult>, JsonRpcError> {
        self.db_adapter
            .validation_result(header)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    // --- Delegated Metadata Primitives ---

    /// Reads a value from the metadata storage by key.
    ///
    /// Corresponds to `DatabaseAdapter::metadata_op_read`.
    ///
    /// # Arguments
    ///
    /// * `key`: Key to read.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<Vec<u8>>)` if the key is found.
    /// * `Err(JsonRpcError::Infrastructure)` if a database error occurs.
    pub async fn metadata_op_read(
        &self,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, JsonRpcError> {
        self.db_adapter
            .metadata_op_read(key)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Writes a value to the metadata storage by key.
    ///
    /// # Arguments
    ///
    /// * `key`: Key to write.
    /// * `value`: Value to write.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the value is written.
    /// * `Err(JsonRpcError::Infrastructure)` if a database error occurs.
    pub async fn metadata_op_write(
        &self,
        key: &[u8],
        value: &[u8],
    ) -> Result<(), JsonRpcError> {
        self.db_adapter
            .metadata_op_write(key, value)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves the height of the current chain tip.
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` if the height is found.
    /// * `Err(JsonRpcError::Infrastructure)` if the tip hash is not found, the
    ///   block header is not found, or a database error occurs.
    pub async fn get_block_height(&self) -> Result<u64, JsonRpcError> {
        self.db_adapter
            .get_block_height()
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves a candidate block by its hash, converting to the JSON-RPC
    /// model.
    ///
    /// # Arguments
    ///
    /// * `block_hash_hex`: Hex string of the block hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::block::CandidateBlock>)`: if the candidate block is
    ///   found.
    /// * `Err(JsonRpcError::Infrastructure)`: if the identifier is invalid or a
    ///   database error occurs.
    pub async fn get_candidate_block_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<model::block::CandidateBlock>, JsonRpcError> {
        self.db_adapter
            .get_candidate_block_by_hash(block_hash_hex)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves the latest candidate block proposed during consensus.
    ///
    /// # Returns
    ///
    /// * `Ok(model::block::CandidateBlock)`: if a latest candidate block is
    ///   found.
    /// * `Err(JsonRpcError::Infrastructure)`: if the identifier is invalid, the
    ///   block is not found, or a database error occurs.
    pub async fn get_latest_candidate_block(
        &self,
    ) -> Result<model::block::CandidateBlock, JsonRpcError> {
        self.db_adapter
            .get_latest_candidate_block()
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves a consensus validation result by its identifier, converting to
    /// the JSON-RPC model.
    ///
    /// # Arguments
    ///
    /// * `prev_block_hash_hex`: Hex string of the previous block hash for the
    ///   consensus round.
    /// * `round`: The consensus round number (block height).
    /// * `iteration`: The consensus iteration number.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(model::consensus::ValidationResult))`: if a result is found
    ///   for the identifier.
    /// * `Ok(None)`: if no validation result matches the identifier.
    /// * `Err(JsonRpcError::Infrastructure)`: if the identifier is invalid or a
    ///   database error occurs.
    pub async fn get_validation_result(
        &self,
        prev_block_hash_hex: &str,
        round: u64,
        iteration: u8,
    ) -> Result<Option<model::consensus::ValidationResult>, JsonRpcError> {
        self.db_adapter
            .get_validation_result(prev_block_hash_hex, round, iteration)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves the latest consensus validation result.
    ///
    /// # Returns
    ///
    /// * `Ok(model::consensus::ValidationResult)`: if the latest result is
    ///   found.
    /// * `Err(JsonRpcError::Infrastructure)`: if the identifier is invalid, the
    ///   result is not found, or a database error occurs.
    pub async fn get_latest_validation_result(
        &self,
    ) -> Result<model::consensus::ValidationResult, JsonRpcError> {
        self.db_adapter
            .get_latest_validation_result()
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves the status (Confirmed, Pending, NotFound) of a transaction by
    /// its hash.
    ///
    /// # Arguments
    ///
    /// * `tx_hash_hex`: Hex string of the transaction hash.
    ///
    /// # Returns
    ///
    /// * `Ok(model::transaction::TransactionStatus)`: describing the status.
    /// * `Err(JsonRpcError::Infrastructure)`: if the hash format is invalid,
    ///   the transaction is not found (neither confirmed nor pending), or a
    ///   database error occurs.
    pub async fn get_transaction_status(
        &self,
        tx_hash_hex: &str,
    ) -> Result<model::transaction::TransactionStatus, JsonRpcError> {
        self.db_adapter
            .get_transaction_status(tx_hash_hex)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves a list of transactions currently in the mempool, sorted by fee
    /// (highest first).
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<model::transaction::TransactionResponse>)`: a vector of
    ///   mempool transactions.
    /// * `Err(JsonRpcError::Infrastructure)`: if a database error occurs.
    pub async fn get_mempool_transactions(
        &self,
    ) -> Result<Vec<model::transaction::TransactionResponse>, JsonRpcError>
    {
        self.db_adapter
            .get_mempool_transactions()
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves a specific transaction from the mempool by hash.
    ///
    /// # Arguments
    ///
    /// * `tx_hash_hex`: Hex string of the transaction hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::transaction::TransactionResponse>)`: if found.
    /// * `Err(JsonRpcError::Infrastructure)`: if the hash format is invalid or
    ///   a database error occurs.
    pub async fn get_mempool_transaction_by_hash(
        &self,
        tx_hash_hex: &str,
    ) -> Result<Option<model::transaction::TransactionResponse>, JsonRpcError>
    {
        self.db_adapter
            .get_mempool_transaction_by_hash(tx_hash_hex)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves statistics about the mempool (count, fee range).
    ///
    /// # Returns
    ///
    /// * `Ok(model::mempool::MempoolInfo)`: containing mempool statistics.
    /// * `Err(JsonRpcError::Infrastructure)`: if a database error occurs.
    pub async fn get_mempool_info(
        &self,
    ) -> Result<model::mempool::MempoolInfo, JsonRpcError> {
        self.db_adapter
            .get_mempool_info()
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves overall chain statistics.
    ///
    /// # Returns
    ///
    /// * `Ok(model::chain::ChainStats)` if found.
    /// * `Err(JsonRpcError::Infrastructure)` if a database error occurs.
    pub async fn get_chain_stats(
        &self,
    ) -> Result<model::chain::ChainStats, JsonRpcError> {
        self.db_adapter
            .get_chain_stats()
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Calculates gas price statistics based on mempool fees.
    ///
    /// # Arguments
    ///
    /// * `max_transactions`: Maximum number of transactions to consider.
    ///
    /// # Returns
    ///
    /// * `Ok(model::gas::GasPriceStats)` if found.
    /// * `Err(JsonRpcError::Infrastructure)` if a database error occurs.
    pub async fn get_gas_price(
        &self,
        max_transactions: Option<usize>,
    ) -> Result<model::gas::GasPriceStats, JsonRpcError> {
        self.db_adapter
            .get_gas_price(max_transactions)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Suggests gas price statistics based on mempool fees.
    ///
    /// # Arguments
    ///
    /// * `max_transactions`: Maximum number of transactions to consider.
    ///
    /// # Returns
    ///
    /// * `Ok(model::gas::GasPriceStats)` if found.
    /// * `Err(JsonRpcError::Infrastructure)` if a database error occurs.
    pub async fn suggest_transaction_fee(
        &self,
        max_transactions: Option<usize>,
    ) -> Result<model::gas::GasPriceStats, JsonRpcError> {
        self.db_adapter
            .suggest_transaction_fee(max_transactions)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves block summary by height.
    ///
    /// # Arguments
    ///
    /// * `height`: Height of the block.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::block::Block>)` if found.
    /// * `Err(JsonRpcError::Infrastructure)` if a database error occurs.
    pub async fn get_block_by_height(
        &self,
        height: u64,
    ) -> Result<Option<model::block::Block>, JsonRpcError> {
        self.db_adapter
            .get_block_by_height(height)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves the latest block summary.
    ///
    /// # Returns
    ///
    /// * `Ok(model::block::Block)` if found.
    /// * `Err(JsonRpcError::Infrastructure)` if a database error occurs.
    pub async fn get_latest_block(
        &self,
    ) -> Result<model::block::Block, JsonRpcError> {
        self.db_adapter
            .get_latest_block()
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves a range of block summaries concurrently.
    ///
    /// # Arguments
    ///
    /// * `height_start`: Start height of the range.
    /// * `height_end`: End height of the range.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<model::block::Block>)` containing summaries for found blocks
    ///   in the range. Note: If individual block lookups within the range fail
    ///   (e.g., height not found), they are skipped.
    /// * `Err(JsonRpcError::Infrastructure)` if `height_start > height_end` or
    ///   a database error occurs.
    pub async fn get_blocks_range(
        &self,
        height_start: u64,
        height_end: u64,
    ) -> Result<Vec<model::block::Block>, JsonRpcError> {
        self.db_adapter
            .get_blocks_range(height_start, height_end)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves multiple block summaries concurrently.
    ///
    /// # Arguments
    ///
    /// * `hashes_hex`: Array of block hashes.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Option<model::block::Block>>)` containing an option for each
    ///   requested hash.
    /// * `Err(JsonRpcError::Infrastructure)` if a database error occurs.
    pub async fn get_blocks_by_hashes(
        &self,
        hashes_hex: &[String],
    ) -> Result<Vec<Option<model::block::Block>>, JsonRpcError> {
        self.db_adapter
            .get_blocks_by_hashes(hashes_hex)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves the latest block header.
    ///
    /// # Returns
    ///
    /// * `Ok(model::block::BlockHeader)` if found.
    /// * `Err(JsonRpcError::Infrastructure)` if a database error occurs.
    pub async fn get_latest_block_header(
        &self,
    ) -> Result<model::block::BlockHeader, JsonRpcError> {
        self.db_adapter
            .get_latest_block_header()
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves a range of block headers concurrently.
    ///
    /// # Arguments
    ///
    /// * `height_start`: Start height of the range.
    /// * `height_end`: End height of the range.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<model::block::BlockHeader>)` containing headers for found
    ///   blocks in the range. Note: If individual header lookups within the
    ///   range fail (e.g., height not found), they are skipped.
    /// * `Err(jsonrpc::error::Error::InternalError)` if `height_start >
    ///   height_end`.
    pub async fn get_block_headers_range(
        &self,
        height_start: u64,
        height_end: u64,
    ) -> Result<Vec<model::block::BlockHeader>, JsonRpcError> {
        self.db_adapter
            .get_block_headers_range(height_start, height_end)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves multiple block headers concurrently.
    ///
    /// # Arguments
    ///
    /// * `hashes_hex`: Array of block hashes.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Option<model::block::BlockHeader>>)` if found.
    /// * `Err(JsonRpcError::Infrastructure)` if a database error occurs.
    pub async fn get_block_headers_by_hashes(
        &self,
        hashes_hex: &[String],
    ) -> Result<Vec<Option<model::block::BlockHeader>>, JsonRpcError> {
        self.db_adapter
            .get_block_headers_by_hashes(hashes_hex)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves block timestamp by hash.
    ///
    /// # Arguments
    ///
    /// * `block_hash_hex`: Block hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<u64>)` if found.
    /// * `Err(JsonRpcError::Infrastructure)` if a database error occurs.
    pub async fn get_block_timestamp_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<u64>, JsonRpcError> {
        self.db_adapter
            .get_block_timestamp_by_hash(block_hash_hex)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves block timestamp by height.
    ///
    /// # Arguments
    ///
    /// * `height`: Height of the block.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<u64>)` if found.
    /// * `Err(JsonRpcError::Infrastructure)` if a database error occurs.
    pub async fn get_block_timestamp_by_height(
        &self,
        height: u64,
    ) -> Result<Option<u64>, JsonRpcError> {
        self.db_adapter
            .get_block_timestamp_by_height(height)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves transactions for a block by height.
    ///
    /// # Arguments
    ///
    /// * `height`: Height of the block.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<Vec<model::transaction::TransactionResponse>>)` if found.
    /// * `Err(JsonRpcError::Infrastructure)` if a database error occurs.
    pub async fn get_block_transactions_by_height(
        &self,
        height: u64,
    ) -> Result<
        Option<Vec<model::transaction::TransactionResponse>>,
        JsonRpcError,
    > {
        self.db_adapter
            .get_block_transactions_by_height(height)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves faults for a block by height.
    ///
    /// # Arguments
    ///
    /// * `height`: Height of the block.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::block::BlockFaults>)` if found.
    /// * `Err(JsonRpcError::Infrastructure)` if a database error occurs.
    pub async fn get_block_faults_by_height(
        &self,
        height: u64,
    ) -> Result<Option<model::block::BlockFaults>, JsonRpcError> {
        self.db_adapter
            .get_block_faults_by_height(height)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves the consensus label for the latest block.
    ///
    /// # Returns
    ///
    /// * `Ok(model::block::BlockLabel)` if found.
    /// * `Err(JsonRpcError::Infrastructure)` if a database error occurs.
    pub async fn get_latest_block_label(
        &self,
    ) -> Result<model::block::BlockLabel, JsonRpcError> {
        self.db_adapter
            .get_latest_block_label()
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves detailed transaction info by hash.
    ///
    /// # Arguments
    ///
    /// * `tx_hash_hex`: Transaction hash.
    /// * `include_tx_index`: Whether to include the transaction index in the
    ///   returned [`TransactionInfo`].
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::transaction::TransactionInfo>)` if found.
    /// * `Err(JsonRpcError::Infrastructure)` if a database error occurs.
    pub async fn get_transaction_by_hash(
        &self,
        tx_hash_hex: &str,
        include_tx_index: bool,
    ) -> Result<Option<model::transaction::TransactionInfo>, JsonRpcError> {
        self.db_adapter
            .get_transaction_by_hash(tx_hash_hex, include_tx_index)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves multiple transactions concurrently.
    ///
    /// # Arguments
    ///
    /// * `tx_hashes_hex`: Array of transaction hashes.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Option<model::transaction::TransactionInfo>>)` if found.
    /// * `Err(JsonRpcError::Infrastructure)` if a database error occurs.
    pub async fn get_transactions_batch(
        &self,
        tx_hashes_hex: &[String],
    ) -> Result<Vec<Option<model::transaction::TransactionInfo>>, JsonRpcError>
    {
        self.db_adapter
            .get_transactions_batch(tx_hashes_hex)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves the count of transactions currently in the mempool.
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` if found.
    /// * `Err(JsonRpcError::Infrastructure)` if a database error occurs.
    pub async fn get_mempool_transactions_count(
        &self,
    ) -> Result<u64, JsonRpcError> {
        self.db_adapter
            .get_mempool_transactions_count()
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves the `count` most recent block summaries.
    ///
    /// # Arguments
    ///
    /// * `count`: The number of latest blocks to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<model::block::Block>)` containing the block summaries.
    /// * `Err(JsonRpcError::Infrastructure)` if fetching the latest block
    ///   height or the block range fails.
    pub async fn get_latest_blocks(
        &self,
        count: u64,
    ) -> Result<Vec<model::block::Block>, JsonRpcError> {
        self.db_adapter
            .get_latest_blocks(count)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves the total number of blocks in the chain.
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` containing the block count (latest height + 1).
    /// * `Err(JsonRpcError::Infrastructure)` if fetching the latest block
    ///   height fails.
    pub async fn get_blocks_count(&self) -> Result<u64, JsonRpcError> {
        self.db_adapter
            .get_blocks_count()
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves a pair of consecutive block summaries by the height of the
    /// first block.
    ///
    /// # Arguments
    ///
    /// * `height`: The height of the first block in the pair.
    ///
    /// # Returns
    ///
    /// * `Ok(Some((model::block::Block, model::block::Block)))` if both blocks
    ///   at `height` and `height + 1` are found.
    /// * `Ok(None)` if either block in the pair is not found.
    /// * `Err(JsonRpcError::Infrastructure)` if a database error occurs.
    pub async fn get_block_pair(
        &self,
        height: u64,
    ) -> Result<Option<(model::block::Block, model::block::Block)>, JsonRpcError>
    {
        self.db_adapter
            .get_block_pair(height)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves a specific range of transactions from a block identified by
    /// its hash.
    ///
    /// # Arguments
    ///
    /// * `block_hash_hex`: The hex-encoded hash of the block.
    /// * `start_index`: The starting index (0-based) of the transaction range.
    /// * `count`: The maximum number of transactions to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<Vec<model::transaction::TransactionResponse>>)`: Contains
    ///   the transactions in the specified range if the block and range are
    ///   valid. Returns `None` if the block itself is not found.
    /// * `Err(JsonRpcError::Infrastructure)`: If a database error occurs.
    pub async fn get_block_transaction_range_by_hash(
        &self,
        block_hash_hex: &str,
        start_index: usize,
        count: usize,
    ) -> Result<
        Option<Vec<model::transaction::TransactionResponse>>,
        JsonRpcError,
    > {
        self.db_adapter
            .get_block_transaction_range_by_hash(
                block_hash_hex,
                start_index,
                count,
            )
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves the last `count` transactions from a block identified by its
    /// height.
    ///
    /// # Arguments
    ///
    /// * `height`: The height of the block.
    /// * `count`: The maximum number of last transactions to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<Vec<model::transaction::TransactionResponse>>)`: Contains
    ///   the last `count` transactions if the block is found. Returns `None` if
    ///   the block itself is not found.
    /// * `Err(JsonRpcError::Infrastructure)`: If a database error occurs.
    pub async fn get_last_block_transactions_by_height(
        &self,
        height: u64,
        count: usize,
    ) -> Result<
        Option<Vec<model::transaction::TransactionResponse>>,
        JsonRpcError,
    > {
        self.db_adapter
            .get_last_block_transactions_by_height(height, count)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves a specific range of transactions from a block identified by
    /// its height.
    ///
    /// # Arguments
    ///
    /// * `height`: The height of the block.
    /// * `start_index`: The starting index (0-based) of the transaction range.
    /// * `count`: The maximum number of transactions to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<Vec<model::transaction::TransactionResponse>>)`: Contains
    ///   the transactions in the specified range if the block and range are
    ///   valid. Returns `None` if the block itself is not found.
    /// * `Err(JsonRpcError::Infrastructure)`: If a database error occurs.
    pub async fn get_block_transaction_range_by_height(
        &self,
        height: u64,
        start_index: usize,
        count: usize,
    ) -> Result<
        Option<Vec<model::transaction::TransactionResponse>>,
        JsonRpcError,
    > {
        self.db_adapter
            .get_block_transaction_range_by_height(height, start_index, count)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Simulates the execution of a transaction without applying state changes.
    ///
    /// This is useful for estimating gas costs or predicting the outcome of a
    /// transaction before broadcasting it.
    ///
    /// # Arguments
    ///
    /// * `tx_bytes` - The serialized transaction bytes to be simulated.
    ///
    /// # Returns
    ///
    /// * `Ok(model::transaction::SimulationResult)` - Contains details about
    ///   the simulation outcome (e.g., gas used, return value, logs).
    /// * `Err(JsonRpcError)` - If the simulation failed (e.g., invalid
    ///   transaction, execution error, internal VM error).
    pub async fn simulate_transaction(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<model::transaction::SimulationResult, JsonRpcError> {
        self.vm_adapter
            .simulate_transaction(tx_bytes)
            .await
            .map_err(|err| RpcInfraError::Vm(err).into())
    }

    // -- VmAdapter Methods --

    /// Performs preverification checks on a transaction.
    ///
    /// Checks performed may include signature validation, nonce checks, and
    /// basic structural validity without full execution.
    ///
    /// # Arguments
    ///
    /// * `tx_bytes` - The serialized transaction bytes to preverify.
    ///
    /// # Returns
    ///
    /// * `Ok(model::vm::VmPreverificationResult)` - Indicates whether the
    ///   preverification checks passed or failed, potentially with details.
    /// * `Err(JsonRpcError)` - If the preverification process encountered an
    ///   internal error.
    pub async fn preverify_transaction(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<model::vm::VmPreverificationResult, JsonRpcError> {
        self.vm_adapter
            .preverify_transaction(tx_bytes)
            .await
            .map_err(|err| RpcInfraError::Vm(err).into())
    }

    /// Retrieves the current chain ID from the VM.
    ///
    /// # Returns
    ///
    /// * `Ok(u8)` - The chain ID.
    /// * `Err(JsonRpcError)` - If retrieving the chain ID failed.
    pub async fn get_chain_id(&self) -> Result<u8, JsonRpcError> {
        self.vm_adapter
            .get_chain_id()
            .await
            .map_err(|err| RpcInfraError::Vm(err).into())
    }

    /// Retrieves account data (balance and nonce) for a given public key.
    ///
    /// # Arguments
    ///
    /// * `pk` - The BLS public key of the account to query.
    ///
    /// # Returns
    ///
    /// * `Ok(model::account::AccountInfo)` - The account's balance and nonce.
    /// * `Err(JsonRpcError)` - If the account query failed (e.g., account not
    ///   found, internal error).
    pub async fn get_account_data(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<model::account::AccountInfo, JsonRpcError> {
        self.vm_adapter
            .get_account_data(pk)
            .await
            .map_err(|err| RpcInfraError::Vm(err).into())
    }

    /// Retrieves the balance for a given account public key.
    ///
    /// # Arguments
    ///
    /// * `pk` - The BLS public key of the account to query.
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` - The account's balance.
    /// * `Err(JsonRpcError)` - If the underlying query failed.
    pub async fn get_account_balance(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<u64, JsonRpcError> {
        self.vm_adapter
            .get_account_balance(pk)
            .await
            .map_err(|err| RpcInfraError::Vm(err).into())
    }

    /// Retrieves the nonce for a given account public key.
    ///
    /// # Arguments
    ///
    /// * `pk` - The BLS public key of the account to query.
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` - The account's nonce.
    /// * `Err(JsonRpcError)` - If the underlying query failed.
    pub async fn get_account_nonce(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<u64, JsonRpcError> {
        self.vm_adapter
            .get_account_nonce(pk)
            .await
            .map_err(|err| RpcInfraError::Vm(err).into())
    }

    /// Retrieves the current state root hash from the VM.
    ///
    /// # Returns
    ///
    /// * `Ok([u8; 32])` - The 32-byte state root hash.
    /// * `Err(JsonRpcError)` - If retrieving the state root failed.
    pub async fn get_state_root(&self) -> Result<[u8; 32], JsonRpcError> {
        self.vm_adapter
            .get_state_root()
            .await
            .map_err(|err| RpcInfraError::Vm(err).into())
    }

    /// Retrieves the gas limit for a block from the VM.
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` - The block gas limit.
    /// * `Err(JsonRpcError)` - If retrieving the gas limit failed.
    pub async fn get_block_gas_limit(&self) -> Result<u64, JsonRpcError> {
        self.vm_adapter
            .get_block_gas_limit()
            .await
            .map_err(|err| RpcInfraError::Vm(err).into())
    }

    /// Retrieves the full details (ProvisionerKeys, ProvisionerStakeData) for
    /// all current provisioners from the VM state.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<(model::provisioner::ProvisionerKeys,
    ///   model::provisioner::ProvisionerStakeData)>)`
    ///   - A vector containing tuples of provisioner keys and stake data for
    ///     each provisioner.
    /// * `Err(JsonRpcError)` - If retrieving the provisioners failed.
    pub async fn get_provisioners(
        &self,
    ) -> Result<
        Vec<(
            model::provisioner::ProvisionerKeys,
            model::provisioner::ProvisionerStakeData,
        )>,
        JsonRpcError,
    > {
        self.vm_adapter
            .get_provisioners()
            .await
            .map_err(|err| RpcInfraError::Vm(err).into())
    }

    /// Retrieves stake information for a single provisioner by their BLS public
    /// key.
    ///
    /// # Arguments
    ///
    /// * `pk` - The BLS public key of the provisioner.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::provisioner::ConsensusStakeInfo>)` - The simplified
    ///   stake information if the provisioner exists, otherwise `None`.
    /// * `Err(JsonRpcError)` - If the query failed.
    pub async fn get_stake_info_by_pk(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<Option<model::provisioner::ConsensusStakeInfo>, JsonRpcError>
    {
        self.vm_adapter
            .get_stake_info_by_pk(pk)
            .await
            .map_err(|err| RpcInfraError::Vm(err).into())
    }

    /// Retrieves a list of all provisioners and their corresponding simplified
    /// stake data (`ConsensusStakeInfo`).
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<(model::key::AccountPublicKey,
    ///   model::provisioner::ConsensusStakeInfo)>)`
    ///   - A vector containing tuples of BLS public keys (wrapped in
    ///     `model::key::AccountPublicKey`) and their simplified stake
    ///     information.
    /// * `Err(JsonRpcError)` - If retrieving the provisioners failed.
    pub async fn get_all_stake_data(
        &self,
    ) -> Result<
        Vec<(
            model::key::AccountPublicKey,
            model::provisioner::ConsensusStakeInfo,
        )>,
        JsonRpcError,
    > {
        self.vm_adapter
            .get_all_stake_data()
            .await
            .map_err(|err| RpcInfraError::Vm(err).into())
    }

    /// Executes a read-only query on a contract at a specific state commit.
    ///
    /// # Arguments
    /// * `contract_id` - The ID of the contract to query.
    /// * `method` - The name of the contract method to call.
    /// * `base_commit` - The state commit hash to execute the query against.
    /// * `args_bytes` - The serialized arguments for the contract method.
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - The serialized result bytes from the contract query.
    /// * `Err(JsonRpcError)` - If the query failed.
    pub async fn query_contract_raw(
        &self,
        contract_id: ContractId,
        method: String,
        base_commit: [u8; 32],
        args_bytes: Vec<u8>,
    ) -> Result<Vec<u8>, JsonRpcError> {
        self.vm_adapter
            .query_contract_raw(contract_id, method, base_commit, args_bytes)
            .await
            .map_err(|err| RpcInfraError::Vm(err).into())
    }

    /// Retrieves the VM configuration settings.
    ///
    /// # Returns
    /// * `Ok(model::vm::VmConfig)` - The VM configuration settings.
    /// * `Err(JsonRpcError)` - If retrieving the configuration failed.
    pub async fn get_vm_config(
        &self,
    ) -> Result<model::vm::VmConfig, JsonRpcError> {
        self.vm_adapter
            .get_vm_config()
            .await
            .map_err(|err| RpcInfraError::Vm(err).into())
    }

    /// Retrieves detailed information about a single provisioner by public key.
    ///
    /// # Arguments
    /// * `pk` - The BLS public key of the provisioner.
    ///
    /// # Returns
    /// * `Ok(model::provisioner::ProvisionerInfo)` - Detailed information if
    ///   the provisioner is found.
    /// * `Err(JsonRpcError)` - If the provisioner is not found or the query
    ///   failed.
    pub async fn get_provisioner_info_by_pk(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<model::provisioner::ProvisionerInfo, JsonRpcError> {
        self.vm_adapter
            .get_provisioner_info_by_pk(pk)
            .await
            .map_err(RpcInfraError::Vm)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Checks if a list of nullifiers (provided as hex strings) have already
    /// been spent.
    ///
    /// This method translates between the JSON-RPC interface (hex strings,
    /// specific response format) and the underlying `VmAdapter` (byte arrays,
    /// returning only existing nullifiers).
    ///
    /// # Arguments
    ///
    /// * `nullifiers_hex`: A vector of 64-character hex strings representing
    ///   the nullifiers to check.
    ///
    /// # Returns
    ///
    /// * `Ok(NullifiersValidationResult)`: An object containing two lists:
    ///   `existing` (nullifiers that are already spent) and `non_existent`
    ///   (nullifiers that are not spent), both as hex strings.
    /// * `Err(JsonRpcError::Infrastructure)`: If hex decoding fails or the
    ///   underlying VM query fails.
    pub async fn validate_nullifiers(
        &self,
        nullifiers_hex: Vec<String>,
    ) -> Result<model::vm::NullifiersValidationResult, JsonRpcError> {
        // 1. Decode hex strings to byte arrays
        let mut decoded_nullifiers = Vec::with_capacity(nullifiers_hex.len());
        let mut invalid_hex = None;
        for hex_str in &nullifiers_hex {
            match hex::decode(hex_str) {
                Ok(bytes) => {
                    if bytes.len() == 32 {
                        // Correct length, attempt conversion to [u8; 32]
                        match bytes.try_into() {
                            Ok(arr) => decoded_nullifiers.push(arr),
                            Err(_) => {
                                // Should be unreachable if len == 32
                                invalid_hex = Some(format!(
                                    "Internal error converting vec to array for: {}",
                                    hex_str
                                ));
                                break;
                            }
                        }
                    } else {
                        invalid_hex = Some(format!(
                            "Invalid hex string length ({} != 32) for: {}",
                            bytes.len(),
                            hex_str
                        ));
                        break;
                    }
                }
                Err(e) => {
                    invalid_hex = Some(format!(
                        "Invalid hex string format for {}: {}",
                        hex_str, e
                    ));
                    break;
                }
            }
        }

        if let Some(err_msg) = invalid_hex {
            return Err(JsonRpcError::Infrastructure(RpcInfraError::Vm(
                VmError::InternalError(err_msg),
            )));
        }

        // 2. Call the VmAdapter
        let existing_bytes = self
            .vm_adapter
            .validate_nullifiers(&decoded_nullifiers)
            .await
            .map_err(RpcInfraError::Vm)
            .map_err(JsonRpcError::Infrastructure)?;

        // 3. Determine non-existent nullifiers and format results
        let existing_set: HashSet<[u8; 32]> =
            existing_bytes.into_iter().collect();
        let mut existing_hex = Vec::new();
        let mut non_existent_hex = Vec::new();

        for (i, original_bytes) in decoded_nullifiers.iter().enumerate() {
            if existing_set.contains(original_bytes) {
                existing_hex.push(nullifiers_hex[i].clone()); // Reuse original
                                                              // hex
            } else {
                non_existent_hex.push(nullifiers_hex[i].clone()); // Reuse original hex
            }
        }

        Ok(model::vm::NullifiersValidationResult {
            existing: existing_hex,
            non_existent: non_existent_hex,
        })
    }

    // ---- NetworkAdapter Methods ----

    /// Broadcasts a transaction to the network.
    ///
    /// # Arguments
    ///
    /// * `tx_bytes` - The serialized transaction bytes to be broadcast.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the broadcast request was successfully initiated.
    /// * `Err(JsonRpcError::Infrastructure)` - If the broadcast failed (e.g.,
    ///   network unavailable, serialization issues, internal errors).
    pub async fn broadcast_transaction(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<(), JsonRpcError> {
        self.network_adapter
            .broadcast_transaction(tx_bytes)
            .await
            .map_err(RpcInfraError::Network)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves general information about the network state.
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - A string containing network information (format
    ///   determined by the underlying implementation).
    /// * `Err(JsonRpcError::Infrastructure)` - If querying the network
    ///   information failed.
    pub async fn get_network_info(&self) -> Result<String, JsonRpcError> {
        self.network_adapter
            .get_network_info()
            .await
            .map_err(RpcInfraError::Network)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves the public network address of this node.
    ///
    /// # Returns
    ///
    /// * `Ok(std::net::SocketAddr)` - The public socket address of the node.
    /// * `Err(JsonRpcError::Infrastructure)` - If the public address could not
    ///   be determined.
    pub async fn get_public_address(
        &self,
    ) -> Result<std::net::SocketAddr, JsonRpcError> {
        self.network_adapter
            .get_public_address()
            .await
            .map_err(RpcInfraError::Network)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves a list of currently alive peers known to the node.
    ///
    /// # Arguments
    ///
    /// * `max_peers` - The maximum number of peer addresses to return.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<std::net::SocketAddr>)` - A vector containing the socket
    ///   addresses of alive peers, up to `max_peers`.
    /// * `Err(JsonRpcError::Infrastructure)` - If retrieving the peer list
    ///   failed.
    pub async fn get_alive_peers(
        &self,
        max_peers: usize,
    ) -> Result<Vec<std::net::SocketAddr>, JsonRpcError> {
        self.network_adapter
            .get_alive_peers(max_peers)
            .await
            .map_err(RpcInfraError::Network)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves the count of currently alive peers known to the node.
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The number of alive peers.
    /// * `Err(JsonRpcError::Infrastructure)` - If counting the peers failed.
    pub async fn get_alive_peers_count(&self) -> Result<usize, JsonRpcError> {
        self.network_adapter
            .get_alive_peers_count()
            .await
            .map_err(RpcInfraError::Network)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Floods an inventory message (`Inv`) across the network.
    ///
    /// # Arguments
    ///
    /// * `inv` - The inventory message to flood.
    /// * `ttl_seconds` - Optional time-to-live for the flood request in
    ///   seconds.
    /// * `hops` - The number of hops the message should propagate.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the flood request was successfully initiated.
    /// * `Err(JsonRpcError::Infrastructure)` - If initiating the flood request
    ///   failed.
    pub async fn flood_request(
        &self,
        inv: Inv,
        ttl_seconds: Option<u64>,
        hops: u16,
    ) -> Result<(), JsonRpcError> {
        self.network_adapter
            .flood_request(inv, ttl_seconds, hops)
            .await
            .map_err(RpcInfraError::Network)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves metrics about the node's connected peers.
    ///
    /// # Returns
    ///
    /// * `Ok(model::network::PeersMetrics)` - Metrics containing the peer
    ///   count.
    /// * `Err(JsonRpcError::Infrastructure)` - If retrieving the peer count
    ///   failed.
    pub async fn get_peers_metrics(
        &self,
    ) -> Result<model::network::PeersMetrics, JsonRpcError> {
        self.network_adapter
            .get_peers_metrics()
            .await
            .map_err(RpcInfraError::Network)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves the geographical location information for connected peers.
    ///
    /// Delegates to the underlying `NetworkAdapter`, which typically queries
    /// an external GeoIP service and caches the results.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<model::network::PeerLocation>)` - A list of location data for
    ///   peers.
    /// * `Err(JsonRpcError::Infrastructure)` - If retrieving peer IPs or
    ///   querying the geolocation service fails.
    pub async fn get_network_peers_location(
        &self,
    ) -> Result<Vec<model::network::PeerLocation>, JsonRpcError> {
        self.network_adapter
            .get_network_peers_location()
            .await
            .map_err(RpcInfraError::Network)
            .map_err(JsonRpcError::Infrastructure)
    }

    /// Retrieves the finality status of a block by its hash.
    ///
    /// This method determines if a block is finalized, accepted into the
    /// canonical chain but not yet final, or unknown.
    /// It delegates to the underlying `DatabaseAdapter`, which is expected to:
    /// 1. Check if the block header exists for the given hash.
    /// 2. If it exists, retrieve the block's height.
    /// 3. Look up the consensus label and associated hash for that height in
    ///    the canonical chain.
    /// 4. Compare the retrieved hash with the input hash.
    /// 5. Map the label (`Final`, `Accepted`, `Confirmed`, `Attested`) to the
    ///    corresponding `BlockFinalityStatus` (`Final` or `Accepted`) if the
    ///    hashes match.
    /// 6. Return `Unknown` if the header is not found or the hashes do not
    ///    match.
    ///
    /// # Arguments
    ///
    /// * `block_hash_hex`: 64-char hex string of the block hash.
    ///
    /// # Returns
    ///
    /// * `Ok(model::block::BlockFinalityStatus)`: indicating if the block is
    ///   `Final`, `Accepted`, or `Unknown`.
    /// * `Err(JsonRpcError::Infrastructure)`: if a database error occurs or the
    ///   hash format is invalid.
    pub async fn get_block_finality_status(
        &self,
        block_hash_hex: &str,
    ) -> Result<model::block::BlockFinalityStatus, JsonRpcError> {
        self.db_adapter
            .get_block_finality_status(block_hash_hex)
            .await
            .map_err(RpcInfraError::Database)
            .map_err(JsonRpcError::Infrastructure)
    }
}
