// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Utility functions for JSON-RPC integration tests.

use async_trait::async_trait;
use rusk::jsonrpc::config::{ConfigError, JsonRpcConfig};
use rusk::jsonrpc::infrastructure::db::{BlockData, DatabaseAdapter};
use rusk::jsonrpc::infrastructure::error::DbError;
use rusk::jsonrpc::infrastructure::manual_limiter::ManualRateLimiters;
use rusk::jsonrpc::infrastructure::metrics::MetricsCollector;
use rusk::jsonrpc::infrastructure::state::{AppState, SubscriptionManager};
use std::fmt::Debug;
use std::sync::Arc;

#[allow(dead_code)]
pub(crate) fn assert_security_error<T>(
    result: &Result<T, ConfigError>,
    expected_substring: &str,
) {
    if let Err(e) = result {
        let error_string_lower = e.to_string().to_lowercase();
        let expected_substring_lower = expected_substring.to_lowercase();
        assert!(
            error_string_lower.contains(&expected_substring_lower),
            "Expected error message to contain (case-insensitive) '{}', but got: {}",
            expected_substring,
            e
        );
    } else {
        panic!(
            "Expected an error containing '{}', but got Ok",
            expected_substring
        );
    }
}

#[allow(dead_code)]
pub(crate) fn create_environment_config(
    _vars: &[(&str, &str)],
) -> JsonRpcConfig {
    JsonRpcConfig::default()
}

// --- Mock Database Adapter ---

/// A simple mock database adapter for testing purposes.
#[derive(Debug, Clone)]
pub(crate) struct MockDbAdapter;

#[async_trait]
impl DatabaseAdapter for MockDbAdapter {
    // Implement required methods with dummy logic
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
    // Add other methods as required by the DatabaseAdapter trait definition
}

// --- Test AppState Creator ---

/// Creates a default AppState instance for use in tests, including
/// ManualRateLimiters.
///
/// Panics if `ManualRateLimiters` cannot be created from the config's rate
/// limits.
#[allow(dead_code)]
pub(crate) fn create_test_app_state() -> AppState {
    // Assume JsonRpcConfig::test_config() provides a valid default test config
    // and includes a valid `rate_limit` field of type RateLimitConfig.
    let config = JsonRpcConfig::test_config();
    let db_adapter = MockDbAdapter;
    let sub_manager = SubscriptionManager::default();
    let metrics = MetricsCollector::default();

    // Create ManualRateLimiters using the RateLimitConfig from JsonRpcConfig
    let rate_limit_config = Arc::new(config.rate_limit.clone());
    let manual_rate_limiters = ManualRateLimiters::new(rate_limit_config)
        .expect("Failed to create manual limiters from test config in helper");

    AppState::new(
        config,
        db_adapter,
        sub_manager,
        metrics,
        manual_rate_limiters,
    )
}
