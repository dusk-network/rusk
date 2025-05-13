// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk::jsonrpc::config::JsonRpcConfig;
use rusk::jsonrpc::infrastructure::manual_limiter::ManualRateLimiters;
use rusk::jsonrpc::infrastructure::metrics::MetricsCollector;
use rusk::jsonrpc::infrastructure::state::AppState;
use rusk::jsonrpc::infrastructure::subscription::manager::SubscriptionManager;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use super::{
    create_basic_mock_block, MockArchiveAdapter, MockDbAdapter,
    MockNetworkAdapter, MockVmAdapter,
};

/// Creates a default AppState instance for use in tests, including
/// ManualRateLimiters.
///
/// Panics if `ManualRateLimiters` cannot be created from the config's rate
/// limits.
///
/// Allows specifying a custom bind address for the HTTP server, overriding
/// defaults and environment variables for test isolation.
pub fn create_test_app_state_with_addr(
    http_addr: Option<SocketAddr>,
) -> AppState {
    let mut config = JsonRpcConfig::default();
    if let Some(addr) = http_addr {
        config.http.bind_address = addr;
    }
    // Ensure the port is what we expect if None was passed (using default)
    else {
        assert_eq!(
            config.http.bind_address.port(),
            8546,
            "Default port assumption failed in create_test_app_state_with_addr"
        );
    }

    let mut blocks_by_hash = HashMap::new();
    let block = create_basic_mock_block(
        100,
        "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    );
    blocks_by_hash.insert(block.header.hash.clone(), block);
    let db_mock = MockDbAdapter {
        blocks_by_hash,
        ..Default::default()
    };
    let archive_mock = MockArchiveAdapter::default();
    let network_mock = MockNetworkAdapter::default();
    let vm_mock = MockVmAdapter::default();
    let sub_manager = SubscriptionManager::default();
    let metrics = MetricsCollector::default();
    let rate_limit_config = Arc::new(config.rate_limit.clone());
    let manual_rate_limiters = ManualRateLimiters::new(rate_limit_config)
        .expect("Failed to create manual rate limiters");

    // Create AppState using the potentially modified config
    AppState::new(
        config, // Use the (potentially modified) config
        Arc::new(db_mock),
        Arc::new(archive_mock),
        Arc::new(network_mock),
        Arc::new(vm_mock),
        sub_manager,
        metrics,
        manual_rate_limiters,
    )
}

// Keep the old helper for compatibility if needed, but point it to the new one
pub fn create_test_app_state() -> AppState {
    create_test_app_state_with_addr(None)
}

/// Helper to setup a basic `AppState` with mock adapters for testing.
pub fn setup_mock_app_state() -> (
    AppState,
    MockDbAdapter,
    MockArchiveAdapter,
    MockNetworkAdapter,
    MockVmAdapter,
) {
    let config = JsonRpcConfig::test_config();
    let db_mock = MockDbAdapter::default();
    let archive_mock = MockArchiveAdapter::default();
    let network_mock = MockNetworkAdapter::default();
    let vm_mock = MockVmAdapter::default();
    let sub_manager = SubscriptionManager::default();
    let metrics = MetricsCollector::default();
    let rate_limit_config = Arc::new(config.rate_limit.clone());
    let manual_rate_limiters = ManualRateLimiters::new(rate_limit_config)
        .expect("Failed to create manual rate limiters");

    let app_state = AppState::new(
        config,
        Arc::new(db_mock.clone()),
        Arc::new(archive_mock.clone()),
        Arc::new(network_mock.clone()),
        Arc::new(vm_mock.clone()),
        sub_manager,
        metrics,
        manual_rate_limiters,
    );

    (app_state, db_mock, archive_mock, network_mock, vm_mock)
}

// Function to manually create AppState with custom JsonRpcConfig
pub fn create_custom_app_state(config: JsonRpcConfig) -> AppState {
    let db_mock = MockDbAdapter::default();
    let archive_mock = MockArchiveAdapter::default();
    let network_mock = MockNetworkAdapter::default();
    let vm_mock = MockVmAdapter::default();
    let sub_manager = SubscriptionManager::default();
    let metrics = MetricsCollector::default();
    let rate_limit_config = Arc::new(config.rate_limit.clone());
    let manual_rate_limiters = ManualRateLimiters::new(rate_limit_config)
        .expect("Failed to create manual rate limiters for custom config");

    AppState::new(
        config, // Use the provided config
        Arc::new(db_mock),
        Arc::new(archive_mock),
        Arc::new(network_mock),
        Arc::new(vm_mock),
        sub_manager,
        metrics,
        manual_rate_limiters,
    )
}
