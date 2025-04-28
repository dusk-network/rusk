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
//! # use rusk::jsonrpc::model::{block::Block, transaction::MoonlightEventGroup};
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
//! # #[derive(Debug, Clone)] struct MockArchiveAdapter; #[async_trait] impl ArchiveAdapter for MockArchiveAdapter { async fn get_moonlight_txs_by_memo(&self, _m: &str) -> Result<Vec<MoonlightEventGroup>, ArchiveError> { Ok(vec![]) } async fn get_last_archived_block_height(&self) -> Result<u64, ArchiveError> { Ok(42) } }
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

use crate::jsonrpc::infrastructure::archive::ArchiveAdapter;
use crate::jsonrpc::infrastructure::db::DatabaseAdapter;
use crate::jsonrpc::infrastructure::error::Error as RpcInfraError;
use crate::jsonrpc::infrastructure::error::{
    ArchiveError, DbError, NetworkError, VmError,
};
use crate::jsonrpc::infrastructure::manual_limiter::ManualRateLimiters;
use crate::jsonrpc::infrastructure::metrics::MetricsCollector;
use crate::jsonrpc::infrastructure::network::NetworkAdapter;
use crate::jsonrpc::infrastructure::subscription::manager::SubscriptionManager;
use crate::jsonrpc::infrastructure::vm::VmAdapter;
use crate::jsonrpc::model::block::Block;
use crate::jsonrpc::model::provisioner::ProvisionerInfo;
use crate::jsonrpc::model::transaction::MoonlightEventGroup;
use crate::jsonrpc::model::transaction::SimulationResult;
use crate::jsonrpc::{
    config::JsonRpcConfig, model::provisioner::StakeOwnerInfo,
};

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

    // --- Delegated DatabaseAdapter Methods ---

    /// Delegates to [`DatabaseAdapter::get_block_by_hash`].
    pub async fn get_block_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<Block>, DbError> {
        self.db_adapter.get_block_by_hash(block_hash_hex).await
    }

    /// Delegates to [`DatabaseAdapter::get_block_by_height`].
    pub async fn get_block_by_height(
        &self,
        height: u64,
    ) -> Result<Option<Block>, DbError> {
        self.db_adapter.get_block_by_height(height).await
    }

    /// Delegates to [`DatabaseAdapter::get_latest_block`].
    pub async fn get_latest_block(&self) -> Result<Block, DbError> {
        self.db_adapter.get_latest_block().await
    }

    // --- Delegated ArchiveAdapter Methods --- //

    /// Delegates to [`ArchiveAdapter::get_moonlight_txs_by_memo`].
    pub async fn get_moonlight_txs_by_memo(
        &self,
        memo_hex: &str,
    ) -> Result<Vec<MoonlightEventGroup>, ArchiveError> {
        self.archive_adapter
            .get_moonlight_txs_by_memo(memo_hex)
            .await
    }

    /// Delegates to [`ArchiveAdapter::get_last_archived_block_height`].
    pub async fn get_last_archived_block_height(
        &self,
    ) -> Result<u64, ArchiveError> {
        self.archive_adapter.get_last_archived_block_height().await
    }

    // --- Delegated NetworkAdapter Methods --- //

    /// Delegates to [`NetworkAdapter::broadcast_transaction`].
    pub async fn broadcast_transaction(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<(), NetworkError> {
        self.network_adapter.broadcast_transaction(tx_bytes).await
    }

    /// Delegates to [`NetworkAdapter::get_public_address`].
    pub async fn get_public_address(&self) -> Result<SocketAddr, NetworkError> {
        self.network_adapter.get_public_address().await
    }

    /// Delegates to [`NetworkAdapter::get_alive_peers`].
    pub async fn get_alive_peers(
        &self,
        max_peers: usize,
    ) -> Result<Vec<SocketAddr>, NetworkError> {
        self.network_adapter.get_alive_peers(max_peers).await
    }

    /// Delegates to [`NetworkAdapter::get_alive_peers_count`].
    pub async fn get_alive_peers_count(&self) -> Result<usize, NetworkError> {
        self.network_adapter.get_alive_peers_count().await
    }

    /// Delegates to [`NetworkAdapter::get_network_info`].
    pub async fn get_network_info(&self) -> Result<String, NetworkError> {
        self.network_adapter.get_network_info().await
    }

    // --- Delegated VmAdapter Methods --- //

    /// Delegates to [`VmAdapter::get_state_root`].
    /// Used internally but exposed for potential direct use.
    pub async fn get_state_root(&self) -> Result<[u8; 32], VmError> {
        self.vm_adapter.get_state_root().await
    }

    /// Delegates to [`VmAdapter::simulate_transaction`].
    pub async fn simulate_transaction(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<SimulationResult, VmError> {
        self.vm_adapter.simulate_transaction(tx_bytes).await
    }

    /// Delegates to [`VmAdapter::query_contract_raw`].
    pub async fn query_contract_raw(
        &self,
        contract_id: dusk_core::abi::ContractId,
        method: String,
        base_commit: [u8; 32],
        args_bytes: Vec<u8>,
    ) -> Result<Vec<u8>, VmError> {
        self.vm_adapter
            .query_contract_raw(contract_id, method, base_commit, args_bytes)
            .await
    }

    /// Delegates to [`VmAdapter::get_vm_config`].
    pub async fn get_vm_config(
        &self,
    ) -> Result<crate::node::RuskVmConfig, VmError> {
        self.vm_adapter.get_vm_config().await
    }

    /// Retrieves the configured chain ID from the VM adapter.
    ///
    /// This is necessary for providing chain context in RPC responses.
    /// Requires the `chain` feature flag, as the VM adapter depends on it.
    #[cfg(feature = "chain")]
    pub async fn get_chain_id(&self) -> Result<u8, VmError> {
        self.vm_adapter.get_chain_id().await
    }

    // --- New Methods for Provisioner Info --- //

    /// Helper function to map internal `StakeData` and the known account public
    /// key to the JSON-RPC `ProvisionerInfo` model.
    fn map_stake_data_to_provisioner_info(
        account_pk: &BlsPublicKey,
        data: &StakeData,
    ) -> ProvisionerInfo {
        let owner_info = StakeOwnerInfo::Account(
            bs58::encode(Serializable::to_bytes(account_pk)).into_string(),
        );

        // Use default if amount is None (though it typically shouldn't be)
        // StakeAmount structure assumed: { value: u64, locked: u64,
        // eligibility: u64 }
        let amount_data = data.amount.unwrap_or_default();

        ProvisionerInfo {
            public_key: bs58::encode(Serializable::to_bytes(account_pk))
                .into_string(),
            amount: amount_data.value, // Assumed field
            locked_amount: amount_data.locked, // Assumed field
            eligibility: amount_data.eligibility, // Assumed field
            reward: data.reward,       // Assumed field
            faults: data.faults,       // Assumed field
            hard_faults: data.hard_faults, // Assumed field
            owner: owner_info,
        }
    }

    /// Retrieves information for all current provisioners using VmAdapter.
    pub async fn get_all_provisioner_info(
        &self,
    ) -> Result<Vec<ProvisionerInfo>, RpcInfraError> {
        // 1. Get current state root using the delegated method
        let state_root = self.get_state_root().await?;

        // 2. Query stake contract for all stakes using VmAdapter
        let args_bytes = Vec::new(); // Empty args for "stakes" method
        let all_stakes_bytes: Vec<u8> = self
            .vm_adapter
            .query_contract_raw(
                STAKE_CONTRACT,
                "stakes".to_string(),
                state_root,
                args_bytes,
            )
            .await
            .map_err(RpcInfraError::Vm)?;

        // 3. Manually deserialize the Vec<(StakeKeys, StakeData)> from bytes
        let all_stakes_results: Vec<(StakeKeys, StakeData)> =
            rkyv::from_bytes(&all_stakes_bytes).map_err(|e| {
                RpcInfraError::Unknown(format!(
                    "Failed to deserialize stakes result: {}",
                    e
                ))
            })?;

        // 4. Map results using the account key from StakeKeys
        let provisioner_infos = all_stakes_results
            .iter()
            .map(|(keys, data)| {
                // Use the correct helper function
                Self::map_stake_data_to_provisioner_info(&keys.account, data)
            })
            .collect();

        Ok(provisioner_infos)
    }

    /// Retrieves information for a single provisioner by their public key using
    /// VmAdapter.
    pub async fn get_single_provisioner_info(
        &self,
        public_key_bls_hex: &str,
    ) -> Result<Option<ProvisionerInfo>, RpcInfraError> {
        // 1. Decode public key from hex
        let pk_bytes_vec = hex::decode(public_key_bls_hex).map_err(|e| {
            RpcInfraError::Unknown(format!("Invalid hex public key: {}", e))
        })?;

        let pk_bytes_arr: [u8; 96] =
            pk_bytes_vec.try_into().map_err(|v: Vec<u8>| {
                RpcInfraError::Unknown(format!(
                    "Invalid BLS public key length: expected 96, got {}",
                    v.len()
                ))
            })?;

        let bls_pk =
            DeserializableSlice::from_slice(&pk_bytes_arr).map_err(|e| {
                RpcInfraError::Unknown(format!(
                    "Invalid BLS public key bytes: {:?}",
                    e
                ))
            })?;

        // 2. Get current state root using the delegated method
        let state_root = self.get_state_root().await?;

        // 3. Serialize args (BlsPublicKey)
        let args_bytes = Serializable::to_bytes(&bls_pk).to_vec();

        // 4. Query stake contract using VmAdapter
        let stake_data_bytes_result = self
            .vm_adapter
            .query_contract_raw(
                STAKE_CONTRACT,
                "get_stake".to_string(),
                state_root,
                args_bytes,
            )
            .await;

        // 5. Handle the result: Deserialize bytes or map error to None/Error
        match stake_data_bytes_result {
            Ok(bytes) => {
                // Successfully got bytes, try to deserialize StakeData
                let stake_data = rkyv::from_bytes::<StakeData>(&bytes)
                    .map_err(|e| {
                        RpcInfraError::Unknown(format!(
                            "Failed to deserialize StakeData result: {}",
                            e
                        ))
                    })?;
                // Deserialization successful, map to ProvisionerInfo
                Ok(Some(Self::map_stake_data_to_provisioner_info(
                    &bls_pk,
                    &stake_data,
                )))
            }
            Err(VmError::ExecutionFailed(e)) if e.contains("NotFound") => {
                // Contract execution indicated 'Not Found'
                // TODO: Confirm this is the correct error variant/message for
                // NotFound
                Ok(None)
            }
            Err(e) => {
                // Any other VmError should be propagated, wrapped in
                // RpcInfraError
                Err(RpcInfraError::Vm(e))
            }
        }
    }
}
