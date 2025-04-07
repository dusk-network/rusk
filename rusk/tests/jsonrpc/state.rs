// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Tests for the `AppState` struct and its functionality.

use super::utils::create_test_app_state;
use async_trait::async_trait;
use parking_lot::RwLock;
use rusk::jsonrpc::config::JsonRpcConfig;
use rusk::jsonrpc::infrastructure::db::BlockData;
use rusk::jsonrpc::infrastructure::db::DatabaseAdapter;
use rusk::jsonrpc::infrastructure::error::DbError;
use rusk::jsonrpc::infrastructure::metrics::MetricsCollector;
use rusk::jsonrpc::infrastructure::state::SubscriptionManager;
use std::sync::Arc;

// --- Mock Implementations for Testing ---

/// Mock implementation of the DatabaseAdapter trait for testing.
#[derive(Debug, Clone)]
struct MockDbAdapter;

// Implement the new DatabaseAdapter trait for the mock
#[async_trait]
impl DatabaseAdapter for MockDbAdapter {
    async fn get_block_by_height(
        &self,
        height: u64,
    ) -> Result<Option<BlockData>, DbError> {
        // Simple mock behavior: return None for height 0, otherwise a dummy
        // block
        if height == 0 {
            Ok(None)
        } else {
            Ok(Some(BlockData {
                height,
                hash: format!("mock_hash_{}", height),
            }))
        }
    }
    // TODO: Implement other methods with mock behavior as they are added to the
    // trait
}

// --- Test Functions ---

#[test]
fn test_app_state_new() {
    let state = create_test_app_state();

    // Basic check: ensure config is accessible and has expected test value
    assert!(!state.config().rate_limit.enabled); // test_config disables rate
                                                 // limiting
}

#[test]
fn test_app_state_clone_send_sync() {
    // Verify AppState is Clone
    let state1 = create_test_app_state();
    let state2 = state1.clone();

    // Check if a field is accessible on the clone
    assert_eq!(
        state1.config().http.bind_address,
        state2.config().http.bind_address
    );

    // Verify AppState is Send + Sync (compilation check)
    // This function requires its argument to be Send + Sync.
    // If AppState doesn't satisfy these, this will fail to compile.
    fn assert_send_sync<T: Send + Sync>(_: T) {}
    assert_send_sync(state1);
    assert_send_sync(state2);

    // Test in a separate thread
    let state_for_thread = create_test_app_state();
    let handle = std::thread::spawn(move || {
        // Access state within the thread
        let _ = state_for_thread.config().http.max_connections;
    });
    handle.join().expect("Thread should complete successfully");
}

#[test]
fn test_app_state_getters() {
    let state = create_test_app_state();

    // Test config getter
    let config_ref: &Arc<JsonRpcConfig> = state.config();
    assert!(!config_ref.rate_limit.enabled); // Check a value from test_config

    // Test db_adapter getter (type check)
    let _db_ref: &Arc<dyn DatabaseAdapter> = state.db_adapter();

    // Test subscription_manager getter (type check and lock access)
    let sub_manager_ref: &Arc<RwLock<SubscriptionManager>> =
        state.subscription_manager();
    // Try acquiring a read lock (should succeed)
    let _read_guard = sub_manager_ref.read(); // Check lock acquisition by getting guard

    // Test metrics_collector getter (type check)
    let _metrics_collector_ref: &Arc<MetricsCollector> =
        state.metrics_collector();

    // Test accessing a feature toggle directly via config
    // Default test_config has enable_websocket = true
    assert!(state.config().features.enable_websocket);

    // Check other feature toggles from test_config
    assert!(state.config().features.method_timing);
    assert!(state.config().features.detailed_errors);
    assert!(!state.config().features.strict_version_checking);
    assert_eq!(state.config().features.max_batch_size, 20);
    assert_eq!(state.config().features.max_block_range, 1000);
}
