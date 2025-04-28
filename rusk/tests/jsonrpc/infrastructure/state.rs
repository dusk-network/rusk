// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Tests for the `AppState` struct and its functionalities.

use crate::jsonrpc::utils::{
    create_test_app_state, setup_mock_app_state, MockArchiveAdapter,
    MockDbAdapter, MockNetworkAdapter, MockVmAdapter,
};

use node::database::rocksdb::MD_HASH_KEY;
use rusk::jsonrpc::infrastructure::manual_limiter::ManualRateLimiters;
use rusk::jsonrpc::infrastructure::metrics::MetricsCollector;
use rusk::jsonrpc::infrastructure::state::AppState;
use rusk::jsonrpc::infrastructure::subscription::manager::SubscriptionManager;
use rusk::jsonrpc::model::block::{Block, BlockHeader, BlockStatus};
use rusk::jsonrpc::{
    config::JsonRpcConfig,
    infrastructure::{
        archive::ArchiveAdapter, db::DatabaseAdapter, network::NetworkAdapter,
        vm::VmAdapter,
    },
};

use std::sync::Arc;

use parking_lot::RwLock;

#[test]
fn test_app_state_creation() {
    let app_state = create_test_app_state();

    // Verify basic properties using accessors
    assert!(
        app_state.config().rate_limit.enabled,
        "Rate limiting should be enabled by default"
    );

    // Check the default HTTP address provided by create_test_app_state() which
    // uses JsonRpcConfig::default()
    let expected_addr: std::net::SocketAddr = "127.0.0.1:8546".parse().unwrap(); // Default port is 8546
    assert_eq!(
        app_state.config().http.bind_address,
        expected_addr,
        "Default HTTP bind address mismatch"
    );

    // Verify config Arc points to the same instance
    assert!(Arc::ptr_eq(app_state.config(), app_state.config()));
}

#[test]
fn test_app_state_clone() {
    let app_state1 = create_test_app_state();
    let app_state2 = app_state1.clone();

    // Verify that cloned state shares the same Arc pointers using accessors
    // Only check accessors that still exist
    assert!(Arc::ptr_eq(app_state1.config(), app_state2.config()));
    assert!(Arc::ptr_eq(
        app_state1.subscription_manager(),
        app_state2.subscription_manager()
    ));
    assert!(Arc::ptr_eq(
        app_state1.metrics_collector(),
        app_state2.metrics_collector()
    ));
    assert!(Arc::ptr_eq(
        app_state1.manual_rate_limiters(),
        app_state2.manual_rate_limiters()
    ));
}

// Test AppState accessor methods return expected types
#[test]
fn test_app_state_accessors() {
    let app_state = create_test_app_state();

    // Just call the remaining accessors to ensure they work and return the
    // correct types. The specific types (&Arc<...>) are checked at compile
    // time.
    let _config: &Arc<JsonRpcConfig> = app_state.config();
    // Removed checks for db_adapter, archive_adapter getters
    let _subs: &Arc<RwLock<SubscriptionManager>> =
        app_state.subscription_manager();
    let _metrics: &Arc<MetricsCollector> = app_state.metrics_collector();
    let _limiters: &Arc<ManualRateLimiters> = app_state.manual_rate_limiters();
}

// Test Send + Sync bounds (compile-time check)
fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn test_app_state_send_sync() {
    assert_send_sync::<AppState>();
}

// Example of how to use the adapters from AppState in a mock test function
#[tokio::test]
async fn test_using_adapters_from_state() {
    // Manually create mocks using Default
    let mut db_mock = MockDbAdapter::default();
    let mut archive_mock = MockArchiveAdapter::default();
    let network_mock = MockNetworkAdapter::default();
    let vm_mock = MockVmAdapter::default();

    // Configure mocks *before* creating AppState
    let mock_hash_hex = hex::encode([100u8; 32]); // Use valid hex
    let mock_header = BlockHeader {
        version: 0,
        height: 100,
        timestamp: 1678886400000,
        hash: mock_hash_hex.clone(), // Use hex hash
        previous_hash: hex::encode([99u8; 32]),
        transactions_root: "mock_tx_root".to_string(),
        state_hash: "mock_state_hash".to_string(),
        validator: "mock_validator_pubkey".to_string(),
        gas_limit: 1000000,
        seed: "mock_seed".to_string(),
        sequence: 1,
    };
    let mock_block = Block {
        header: mock_header.clone(),
        status: Some(BlockStatus::Final),
        transactions_count: 0,
        block_reward: Some(1000),
        total_gas_limit: Some(50000),
        transactions: None,
    };

    // Access fields on the concrete mock types BEFORE Arc::new()
    db_mock.blocks_by_height.insert(100, mock_block.clone());
    db_mock
        .blocks_by_hash
        .insert(mock_header.hash.clone(), mock_block.clone()); // Key is hex hash
    db_mock
        .headers_by_hash
        .insert(mock_header.hash.clone(), mock_header.clone()); // Key is hex hash
    db_mock.latest_height = 100;
    db_mock.metadata.insert(
        MD_HASH_KEY.to_vec(),
        hex::decode(&mock_header.hash).expect("Hex decode should succeed"), // Decode hex hash
    );

    // Set the mock value for the last archived block (height, hash)
    archive_mock.last_archived_block =
        Some((99, "mock_archived_hash".to_string()));

    // Create Arcs for the configured mocks
    let db_adapter_arc: Arc<dyn DatabaseAdapter> = Arc::new(db_mock);
    let archive_adapter_arc: Arc<dyn ArchiveAdapter> = Arc::new(archive_mock);
    let network_adapter_arc: Arc<dyn NetworkAdapter> = Arc::new(network_mock);
    let vm_adapter_arc: Arc<dyn VmAdapter> = Arc::new(vm_mock);

    // Now create AppState using the configured mock Arcs
    let config = JsonRpcConfig::test_config(); // Use a test config
    let subscription_manager = SubscriptionManager::default();
    let metrics_collector = MetricsCollector::default();
    let rate_limit_config = Arc::new(config.rate_limit.clone());
    let manual_rate_limiters = ManualRateLimiters::new(rate_limit_config)
        .expect("Failed to create manual rate limiters");

    let app_state = AppState::new(
        config,
        db_adapter_arc, // Pass the Arc
        archive_adapter_arc,
        network_adapter_arc,
        vm_adapter_arc,
        subscription_manager,
        metrics_collector,
        manual_rate_limiters,
    );

    // Call delegated methods directly on app_state
    let block_opt = app_state.get_block_by_height(100).await;
    assert!(block_opt.is_ok(), "get_block_by_height failed");
    assert!(block_opt.unwrap().is_some(), "Block 100 not found");

    let latest_block = app_state.get_latest_block().await;
    assert!(latest_block.is_ok(), "get_latest_block failed");
    assert_eq!(
        latest_block.unwrap().header.height,
        100,
        "Latest block height mismatch"
    );

    let last_archived = app_state.get_last_archived_block_height().await;
    assert!(
        last_archived.is_ok(),
        "get_last_archived_block_height failed"
    );
    assert_eq!(last_archived.unwrap(), 99, "Last archived height mismatch");
}

#[test]
fn app_state_creation_and_accessors() {
    let (app_state, _, _, _, _) = setup_mock_app_state();

    // Verify remaining accessors return the correct types and values
    let expected_test_addr: std::net::SocketAddr =
        "127.0.0.1:0".parse().unwrap();
    assert_eq!(app_state.config().http.bind_address, expected_test_addr);
    assert_eq!(app_state.config().ws.bind_address, expected_test_addr);
    // Removed checks for db_adapter, archive_adapter, network_adapter,
    // vm_adapter getters

    // Verify SubscriptionManager (RwLock access)
    let sub_manager_lock = app_state.subscription_manager().read();
    // Can perform checks on the sub_manager if needed.
    // TODO: Re-add subscriber count check when SubscriptionManager is
    // implemented.
    drop(sub_manager_lock); // Release lock

    // Verify MetricsCollector
    let metrics_collector = app_state.metrics_collector();
    assert!(Arc::strong_count(metrics_collector) >= 1);

    // Verify ManualRateLimiters
    let limiters = app_state.manual_rate_limiters();
    assert!(Arc::strong_count(limiters) >= 1);

    // Test cloning
    let state_clone = app_state.clone();
    assert_eq!(
        state_clone.config().http.bind_address,
        app_state.config().http.bind_address
    );
    // Removed Arc::ptr_eq checks for adapters
    assert!(Arc::ptr_eq(
        app_state.subscription_manager(),
        state_clone.subscription_manager()
    ));
    assert!(Arc::ptr_eq(
        app_state.metrics_collector(),
        state_clone.metrics_collector()
    ));
    assert!(Arc::ptr_eq(
        app_state.manual_rate_limiters(),
        state_clone.manual_rate_limiters()
    ));
}

#[test]
fn app_state_debug_impl() {
    // Use the helper to setup a mock state
    let (app_state, _, _, _, _) = setup_mock_app_state(); // Corrected: Use imported helper

    // Check that the Debug output does not panic and contains expected field
    // names Note: The exact output might change if AppState's Debug impl is
    // customized
    let debug_output = format!("{:?}", app_state);
    assert!(debug_output.starts_with("AppState {"));
    assert!(debug_output.contains("config:"));
    // Removed checks for db_adapter, archive_adapter, etc. in debug output
    assert!(debug_output.contains("subscription_manager:"));
    assert!(debug_output.contains("metrics_collector:"));
    assert!(debug_output.contains("manual_rate_limiters:"));
    assert!(debug_output.ends_with('}'));
}

// Test that AppState is Send + Sync (compile-time check)
#[test]
fn app_state_is_send_sync() {
    assert_send_sync::<AppState>();
}
