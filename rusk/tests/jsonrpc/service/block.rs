// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::Arc;
use std::time::Duration;

use reqwest::StatusCode;
use serde_json::json;
use tokio::time::sleep;

use rusk::jsonrpc::config::JsonRpcConfig;
use rusk::jsonrpc::infrastructure::{
    manual_limiter::ManualRateLimiters, metrics::MetricsCollector,
    state::AppState, subscription::manager::SubscriptionManager,
};
use rusk::jsonrpc::server::run_server;

use crate::jsonrpc::utils::{
    create_custom_app_state,
    create_mock_model_block_with_optional_transactions, get_ephemeral_port,
    MockArchiveAdapter, MockDbAdapter, MockNetworkAdapter, MockVmAdapter,
};

#[tokio::test]
async fn test_get_block_by_hash_with_optional_param() {
    // --- Test Case 1: include_txs = true ---
    {
        let ephemeral_addr_true = get_ephemeral_port()
            .expect("Failed to get ephemeral port (case true)");
        let mut config_true = JsonRpcConfig::test_config();
        config_true.http.bind_address = ephemeral_addr_true;
        config_true.http.cert = None;
        config_true.http.key = None;

        let mut db_mock_true = MockDbAdapter::default();
        let hash_true =
            "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                .to_string();
        let block_with_txs = create_mock_model_block_with_optional_transactions(
            &hash_true, 123, true,
        ); // true -> transactions: Some(...)
        assert!(block_with_txs.transactions.is_some()); // Sanity check helper
        db_mock_true
            .blocks_by_hash
            .insert(hash_true.clone(), block_with_txs);

        let manual_rate_limiters_true =
            ManualRateLimiters::new(Arc::new(config_true.rate_limit.clone()))
                .unwrap();
        let app_state_true = Arc::new(AppState::new(
            config_true,
            Arc::new(db_mock_true),
            Arc::new(MockArchiveAdapter::default()),
            Arc::new(MockNetworkAdapter::default()),
            Arc::new(MockVmAdapter::default()),
            SubscriptionManager::default(),
            MetricsCollector::default(),
            manual_rate_limiters_true,
        ));
        let addr_true = app_state_true.config().http.bind_address;
        let server_task_true = tokio::spawn(run_server(app_state_true.clone()));
        sleep(Duration::from_millis(500)).await; // Shorter sleep might be ok

        let client_true = reqwest::Client::new();
        let rpc_url_true = format!("http://{}/rpc", addr_true);
        let request_with_param = json!({
            "jsonrpc": "2.0",
            "method": "getBlockByHash",
            "params": [hash_true, true], // Use hash_true, include_txs=true
            "id": "test-block-by-hash-with-param-true"
        });

        let response = client_true
            .post(&rpc_url_true)
            .json(&request_with_param)
            .send()
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Request status mismatch (true case)"
        );

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert!(
            response_json.get("result").is_some(),
            "Expected result (true case)"
        );
        assert_ne!(
            response_json["result"]["transactions"],
            serde_json::Value::Null,
            "Expected transactions not null (true case)"
        );
        assert!(
            response_json["result"]["transactions"].is_array(),
            "Expected transactions array (true case)"
        );

        server_task_true.abort();
        let _ = server_task_true.await;
    }

    // --- Test Case 2: include_txs omitted (defaults to false) ---
    {
        let ephemeral_addr_false = get_ephemeral_port()
            .expect("Failed to get ephemeral port (case false)");
        let mut config_false = JsonRpcConfig::test_config();
        config_false.http.bind_address = ephemeral_addr_false;
        config_false.http.cert = None;
        config_false.http.key = None;

        let mut db_mock_false = MockDbAdapter::default();
        let hash_false =
            "6464646464646464646464646464646464646464646464646464646464646464"
                .to_string();
        let block_without_txs =
            create_mock_model_block_with_optional_transactions(
                &hash_false,
                456,
                false,
            ); // false -> transactions: None
        assert!(block_without_txs.transactions.is_none()); // Sanity check helper
        db_mock_false
            .blocks_by_hash
            .insert(hash_false.clone(), block_without_txs);

        let manual_rate_limiters_false =
            ManualRateLimiters::new(Arc::new(config_false.rate_limit.clone()))
                .unwrap();
        let app_state_false = Arc::new(AppState::new(
            config_false,
            Arc::new(db_mock_false),
            Arc::new(MockArchiveAdapter::default()),
            Arc::new(MockNetworkAdapter::default()),
            Arc::new(MockVmAdapter::default()),
            SubscriptionManager::default(),
            MetricsCollector::default(),
            manual_rate_limiters_false,
        ));
        let addr_false = app_state_false.config().http.bind_address;
        let server_task_false =
            tokio::spawn(run_server(app_state_false.clone()));
        sleep(Duration::from_millis(500)).await; // Shorter sleep might be ok

        let client_false = reqwest::Client::new();
        let rpc_url_false = format!("http://{}/rpc", addr_false);
        let request_without_optional_param = json!({
            "jsonrpc": "2.0",
            "method": "getBlockByHash",
            "params": [hash_false], // Use hash_false, include_txs omitted
            "id": "test-block-by-hash-with-param-false"
        });

        let response = client_false
            .post(&rpc_url_false)
            .json(&request_without_optional_param)
            .send()
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Request status mismatch (false case)"
        );

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert!(
            response_json.get("result").is_some(),
            "Expected result (false case)"
        );
        assert_eq!(
            response_json["result"]["transactions"],
            serde_json::Value::Null,
            "Expected transactions null (false case)"
        );

        server_task_false.abort();
        let _ = server_task_false.await;
    }
}

#[tokio::test]
async fn test_get_block_by_height_with_optional_param() {
    // --- Test Case 1: include_txs = true ---
    {
        let ephemeral_addr_true = get_ephemeral_port()
            .expect("Failed to get ephemeral port (case true)");
        let mut config_true = JsonRpcConfig::test_config();
        config_true.http.bind_address = ephemeral_addr_true;
        config_true.http.cert = None;
        config_true.http.key = None;

        let mut db_mock_true = MockDbAdapter::default();
        let height_true: u64 = 123;
        let hash_true = hex::encode([height_true as u8; 32]); // Create a valid hex hash
        let block_with_txs = create_mock_model_block_with_optional_transactions(
            &hash_true,
            height_true,
            true,
        ); // true -> transactions: Some(...)
        assert!(block_with_txs.transactions.is_some()); // Sanity check helper
        db_mock_true
            .blocks_by_height
            .insert(height_true, block_with_txs);

        let manual_rate_limiters_true =
            ManualRateLimiters::new(Arc::new(config_true.rate_limit.clone()))
                .unwrap();
        let app_state_true = Arc::new(AppState::new(
            config_true,
            Arc::new(db_mock_true),
            Arc::new(MockArchiveAdapter::default()),
            Arc::new(MockNetworkAdapter::default()),
            Arc::new(MockVmAdapter::default()),
            SubscriptionManager::default(),
            MetricsCollector::default(),
            manual_rate_limiters_true,
        ));
        let addr_true = app_state_true.config().http.bind_address;
        let server_task_true = tokio::spawn(run_server(app_state_true.clone()));
        sleep(Duration::from_millis(500)).await;

        let client_true = reqwest::Client::new();
        let rpc_url_true = format!("http://{}/rpc", addr_true);
        let request_with_param = json!({
            "jsonrpc": "2.0",
            "method": "getBlockByHeight",
            "params": [height_true, true], // Use height_true, include_txs=true
            "id": "test-block-by-height-with-param-true"
        });

        let response = client_true
            .post(&rpc_url_true)
            .json(&request_with_param)
            .send()
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Request status mismatch (true case)"
        );

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert!(
            response_json.get("result").is_some(),
            "Expected result (height true case)"
        );
        // getBlockByHeight returns Option<Block>, so result is the block or
        // null
        assert!(
            response_json["result"].is_object(),
            "Expected result to be a block object (height true case)"
        );
        assert_ne!(
            response_json["result"]["transactions"],
            serde_json::Value::Null,
            "Expected transactions not null (height true case)"
        );
        assert!(
            response_json["result"]["transactions"].is_array(),
            "Expected transactions array (height true case)"
        );

        server_task_true.abort();
        let _ = server_task_true.await;
    }

    // --- Test Case 2: include_txs omitted (defaults to false) ---
    {
        let ephemeral_addr_false = get_ephemeral_port()
            .expect("Failed to get ephemeral port (case false)");
        let mut config_false = JsonRpcConfig::test_config();
        config_false.http.bind_address = ephemeral_addr_false;
        config_false.http.cert = None;
        config_false.http.key = None;

        let mut db_mock_false = MockDbAdapter::default();
        let height_false: u64 = 456;
        let hash_false = hex::encode([height_false as u8; 32]); // Create a valid hex hash
        let block_without_txs =
            create_mock_model_block_with_optional_transactions(
                &hash_false,
                height_false,
                false,
            ); // false -> transactions: None
        assert!(block_without_txs.transactions.is_none()); // Sanity check helper
        db_mock_false
            .blocks_by_height
            .insert(height_false, block_without_txs);

        let manual_rate_limiters_false =
            ManualRateLimiters::new(Arc::new(config_false.rate_limit.clone()))
                .unwrap();
        let app_state_false = Arc::new(AppState::new(
            config_false,
            Arc::new(db_mock_false),
            Arc::new(MockArchiveAdapter::default()),
            Arc::new(MockNetworkAdapter::default()),
            Arc::new(MockVmAdapter::default()),
            SubscriptionManager::default(),
            MetricsCollector::default(),
            manual_rate_limiters_false,
        ));
        let addr_false = app_state_false.config().http.bind_address;
        let server_task_false =
            tokio::spawn(run_server(app_state_false.clone()));
        sleep(Duration::from_millis(500)).await;

        let client_false = reqwest::Client::new();
        let rpc_url_false = format!("http://{}/rpc", addr_false);
        let request_without_optional_param = json!({
            "jsonrpc": "2.0",
            "method": "getBlockByHeight",
            "params": [height_false], // Use height_false, include_txs omitted
            "id": "test-block-by-height-with-param-false"
        });

        let response = client_false
            .post(&rpc_url_false)
            .json(&request_without_optional_param)
            .send()
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Request status mismatch (false case)"
        );

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert!(
            response_json.get("result").is_some(),
            "Expected result (height false case)"
        );
        assert!(
            response_json["result"].is_object(),
            "Expected result to be a block object (height false case)"
        );
        assert_eq!(
            response_json["result"]["transactions"],
            serde_json::Value::Null,
            "Expected transactions null (height false case)"
        );

        server_task_false.abort();
        let _ = server_task_false.await;
    }
}

#[tokio::test]
async fn test_get_latest_block_with_optional_param() {
    let latest_height: u64 = 999;
    let latest_hash = hex::encode([latest_height as u8; 32]); // Create a valid hex hash

    // --- Test Case 1: include_txs = true ---
    {
        let ephemeral_addr_true = get_ephemeral_port()
            .expect("Failed to get ephemeral port (latest true)");
        let mut config_true = JsonRpcConfig::test_config();
        config_true.http.bind_address = ephemeral_addr_true;
        config_true.http.cert = None;
        config_true.http.key = None;

        let mut db_mock_true = MockDbAdapter::default();
        // Set latest height in mock and add the corresponding block with
        // transactions
        db_mock_true.latest_height = latest_height;
        let block_with_txs = create_mock_model_block_with_optional_transactions(
            &latest_hash,
            latest_height,
            true,
        ); // true -> transactions: Some(...)
        assert!(block_with_txs.transactions.is_some());
        db_mock_true
            .blocks_by_height
            .insert(latest_height, block_with_txs);
        // Ensure get_latest_block in AppState uses db_adapter.latest_height and
        // db_adapter.get_block_by_height

        let manual_rate_limiters_true =
            ManualRateLimiters::new(Arc::new(config_true.rate_limit.clone()))
                .unwrap();
        let app_state_true = Arc::new(AppState::new(
            config_true,
            Arc::new(db_mock_true),
            Arc::new(MockArchiveAdapter::default()),
            Arc::new(MockNetworkAdapter::default()),
            Arc::new(MockVmAdapter::default()),
            SubscriptionManager::default(),
            MetricsCollector::default(),
            manual_rate_limiters_true,
        ));
        let addr_true = app_state_true.config().http.bind_address;
        let server_task_true = tokio::spawn(run_server(app_state_true.clone()));
        sleep(Duration::from_millis(500)).await;

        let client_true = reqwest::Client::new();
        let rpc_url_true = format!("http://{}/rpc", addr_true);
        let request_with_param = json!({
            "jsonrpc": "2.0",
            "method": "getLatestBlock",
            "params": [true], // include_txs=true
            "id": "test-latest-block-with-param-true"
        });

        let response = client_true
            .post(&rpc_url_true)
            .json(&request_with_param)
            .send()
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Request status mismatch (latest true case)"
        );

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert!(
            response_json.get("result").is_some(),
            "Expected result (latest true case)"
        );
        assert_ne!(
            response_json["result"]["transactions"],
            serde_json::Value::Null,
            "Expected transactions not null (latest true case)"
        );
        assert!(
            response_json["result"]["transactions"].is_array(),
            "Expected transactions array (latest true case)"
        );
        assert_eq!(
            response_json["result"]["header"]["height"],
            latest_height.to_string(),
            "Height mismatch (latest true)"
        );

        server_task_true.abort();
        let _ = server_task_true.await;
    }

    // --- Test Case 2: include_txs omitted (defaults to false) ---
    {
        let ephemeral_addr_false = get_ephemeral_port()
            .expect("Failed to get ephemeral port (latest false)");
        let mut config_false = JsonRpcConfig::test_config();
        config_false.http.bind_address = ephemeral_addr_false;
        config_false.http.cert = None;
        config_false.http.key = None;

        let mut db_mock_false = MockDbAdapter::default();
        // Set latest height and add corresponding block *without* transactions
        db_mock_false.latest_height = latest_height;
        let block_without_txs =
            create_mock_model_block_with_optional_transactions(
                &latest_hash,
                latest_height,
                false,
            ); // false -> transactions: None
        assert!(block_without_txs.transactions.is_none());
        db_mock_false
            .blocks_by_height
            .insert(latest_height, block_without_txs);

        let manual_rate_limiters_false =
            ManualRateLimiters::new(Arc::new(config_false.rate_limit.clone()))
                .unwrap();
        let app_state_false = Arc::new(AppState::new(
            config_false,
            Arc::new(db_mock_false),
            Arc::new(MockArchiveAdapter::default()),
            Arc::new(MockNetworkAdapter::default()),
            Arc::new(MockVmAdapter::default()),
            SubscriptionManager::default(),
            MetricsCollector::default(),
            manual_rate_limiters_false,
        ));
        let addr_false = app_state_false.config().http.bind_address;
        let server_task_false =
            tokio::spawn(run_server(app_state_false.clone()));
        sleep(Duration::from_millis(500)).await;

        let client_false = reqwest::Client::new();
        let rpc_url_false = format!("http://{}/rpc", addr_false);
        let request_without_optional_param = json!({
            "jsonrpc": "2.0",
            "method": "getLatestBlock",
            "params": [], // include_txs omitted
            "id": "test-latest-block-with-param-false"
        });

        let response = client_false
            .post(&rpc_url_false)
            .json(&request_without_optional_param)
            .send()
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Request status mismatch (latest false case)"
        );

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert!(
            response_json.get("result").is_some(),
            "Expected result (latest false case)"
        );
        assert_eq!(
            response_json["result"]["transactions"],
            serde_json::Value::Null,
            "Expected transactions null (latest false case)"
        );
        assert_eq!(
            response_json["result"]["header"]["height"],
            latest_height.to_string(),
            "Height mismatch (latest false)"
        );

        server_task_false.abort();
        let _ = server_task_false.await;
    }
}

#[tokio::test]
async fn test_get_block_by_hash_invalid_hash() {
    // 1. Setup: Start server on an ephemeral port
    let ephemeral_addr =
        get_ephemeral_port().expect("Failed to get ephemeral port");

    let mut config = JsonRpcConfig::test_config();
    config.http.bind_address = ephemeral_addr;
    config.http.cert = None;
    config.http.key = None;

    let app_state = Arc::new(create_custom_app_state(config));
    let addr = app_state.config().http.bind_address;

    // 2. Action: Spawn server
    let server_task = tokio::spawn(run_server(app_state.clone()));
    sleep(Duration::from_millis(1500)).await;

    // 3. Assert: Send RPC request with invalid hash
    let client = reqwest::Client::new();
    let rpc_url = format!("http://{}/rpc", addr);

    let request_invalid_hash = json!({
        "jsonrpc": "2.0",
        "method": "getBlockByHash",
        "params": ["invalid-hash", false],
        "id": "test-block-by-hash-invalid"
    });

    let response = client
        .post(&rpc_url)
        .json(&request_invalid_hash)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response_json: serde_json::Value = response.json().await.unwrap();
    assert!(
        response_json.get("error").is_some(),
        "Expected error response for invalid hash"
    );
    assert_eq!(
        response_json["error"]["code"], -32602,
        "Expected invalid params error code"
    );

    // 4. Cleanup
    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn test_get_block_by_height_invalid_height() {
    // 1. Setup: Start server on an ephemeral port
    let ephemeral_addr =
        get_ephemeral_port().expect("Failed to get ephemeral port");

    let mut config = JsonRpcConfig::test_config();
    config.http.bind_address = ephemeral_addr;
    config.http.cert = None;
    config.http.key = None;

    let app_state = Arc::new(create_custom_app_state(config));
    let addr = app_state.config().http.bind_address;

    // 2. Action: Spawn server
    let server_task = tokio::spawn(run_server(app_state.clone()));
    sleep(Duration::from_millis(1500)).await;

    // 3. Assert: Send RPC request with invalid height (0 is invalid)
    let client = reqwest::Client::new();
    let rpc_url = format!("http://{}/rpc", addr);

    let request_invalid_height = json!({
        "jsonrpc": "2.0",
        "method": "getBlockByHeight",
        "params": [0, false],
        "id": "test-block-by-height-invalid"
    });

    let response = client
        .post(&rpc_url)
        .json(&request_invalid_height)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response_json: serde_json::Value = response.json().await.unwrap();
    assert!(
        response_json.get("error").is_some(),
        "Expected error response for invalid height"
    );
    assert_eq!(
        response_json["error"]["code"], -32602,
        "Expected invalid params error code"
    );

    // 4. Cleanup
    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn test_get_block_by_hash_transactions_none() {
    // 1. Setup: Start server on an ephemeral port
    let ephemeral_addr =
        get_ephemeral_port().expect("Failed to get ephemeral port");

    let mut config = JsonRpcConfig::test_config();
    config.http.bind_address = ephemeral_addr;
    config.http.cert = None;
    config.http.key = None;

    // Setup mock DB for this specific test
    let mut db_mock = MockDbAdapter::default();
    let hash_to_find =
        "6464646464646464646464646464646464646464646464646464646464646464"
            .to_string();

    let block_model_from_node_without_txs =
        create_mock_model_block_with_optional_transactions(
            &hash_to_find,
            456,
            false,
        ); // false -> empty tx list in NodeBlock

    // Sanity check the created model block before inserting
    assert!(block_model_from_node_without_txs.transactions.is_none());
    assert_eq!(block_model_from_node_without_txs.transactions_count, 0);

    db_mock
        .blocks_by_hash
        .insert(hash_to_find.clone(), block_model_from_node_without_txs);

    // Manually create AppState with the configured mock DB
    let manual_rate_limiters =
        ManualRateLimiters::new(Arc::new(config.rate_limit.clone()))
            .expect("Failed to create manual rate limiters for txs_none test");
    let app_state = Arc::new(AppState::new(
        config,            // Pass config directly
        Arc::new(db_mock), // Use the configured mock
        Arc::new(MockArchiveAdapter::default()),
        Arc::new(MockNetworkAdapter::default()),
        Arc::new(MockVmAdapter::default()),
        SubscriptionManager::default(),
        MetricsCollector::default(),
        manual_rate_limiters,
    ));
    let addr = app_state.config().http.bind_address; // Get address from AppState's config

    // 2. Action: Spawn server
    let server_task = tokio::spawn(run_server(app_state.clone()));
    sleep(Duration::from_millis(1500)).await;

    // 3. Assert: Send RPC request without include_txs parameter
    let client = reqwest::Client::new();
    let rpc_url = format!("http://{}/rpc", addr);

    // Request uses the specific hash we inserted into the mock db
    let request_without_include_txs = json!({
        "jsonrpc": "2.0",
        "method": "getBlockByHash",
        "params": [hash_to_find], // Use the hash variable
        "id": "test-block-by-hash-transactions-none"
    });

    let response = client
        .post(&rpc_url)
        .json(&request_without_include_txs)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response_json: serde_json::Value = response.json().await.unwrap();
    assert!(
        response_json.get("result").is_some(),
        "Expected result in response for txs_none"
    );
    // Because the RPC layer defaults include_txs to false, and the model block
    // When include_txs is false (default), the transactions field should be
    // null in the JSON response, evenxs (or an empty list), the final JSON
    // should have 'transactions: null'.
    assert_eq!(
        response_json["result"]["transactions"],
        serde_json::Value::Null,
        "Expected transactions to be null when include_txs is omitted"
    );

    // 4. Cleanup
    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn test_get_next_block_with_phoenix_transaction() {
    // --- Test Case 1: Phoenix transaction found ---
    {
        let ephemeral_addr = get_ephemeral_port()
            .expect("Failed to get ephemeral port (case found)");
        let mut config = JsonRpcConfig::test_config();
        config.http.bind_address = ephemeral_addr;
        config.http.cert = None;
        config.http.key = None;

        // Setup mock data
        let mut archive_mock = MockArchiveAdapter::default();
        let from_height = 1000;
        let phoenix_block_height = 1002;
        archive_mock
            .next_phoenix_height
            .insert(from_height, Some(phoenix_block_height));

        let db_mock = MockDbAdapter::default();
        let network_mock = MockNetworkAdapter::default();
        let vm_mock = MockVmAdapter::default();
        let sub_manager = SubscriptionManager::default();
        let metrics = MetricsCollector::default();
        let rate_limiters =
            ManualRateLimiters::new(Arc::new(config.rate_limit.clone()))
                .expect("Failed to create rate limiters");

        let app_state = Arc::new(AppState::new(
            config.clone(),
            Arc::new(db_mock),
            Arc::new(archive_mock),
            Arc::new(network_mock),
            Arc::new(vm_mock),
            sub_manager,
            metrics,
            rate_limiters,
        ));

        // Start the JSON-RPC server
        let server_handle = tokio::spawn(run_server(app_state));
        sleep(Duration::from_millis(500)).await;

        // Send a request to get_next_block_with_phoenix_transaction
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("http://{}/rpc", ephemeral_addr))
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getNextBlockWithPhoenixTransaction",
                "params": [1000]
            }))
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(resp.status(), StatusCode::OK);
        let resp_json = resp
            .json::<serde_json::Value>()
            .await
            .expect("Failed to parse response as JSON");

        // Verify the response format when a Phoenix transaction is found
        assert_eq!(resp_json["jsonrpc"], "2.0");
        assert_eq!(resp_json["id"], 1);
        assert_eq!(resp_json["result"], "1002");

        // Cleanup
        server_handle.abort();
    }

    // --- Test Case 2: Phoenix transaction not found ---
    {
        let ephemeral_addr = get_ephemeral_port()
            .expect("Failed to get ephemeral port (case not found)");
        let mut config = JsonRpcConfig::test_config();
        config.http.bind_address = ephemeral_addr;
        config.http.cert = None;
        config.http.key = None;

        // Setup mock data - Return None for the
        // get_next_block_with_phoenix_transaction
        let mut archive_mock = MockArchiveAdapter::default();
        let from_height = 1000;
        archive_mock.next_phoenix_height.insert(from_height, None);

        let db_mock = MockDbAdapter::default();
        let network_mock = MockNetworkAdapter::default();
        let vm_mock = MockVmAdapter::default();
        let sub_manager = SubscriptionManager::default();
        let metrics = MetricsCollector::default();
        let rate_limiters =
            ManualRateLimiters::new(Arc::new(config.rate_limit.clone()))
                .expect("Failed to create rate limiters");

        let app_state = Arc::new(AppState::new(
            config.clone(),
            Arc::new(db_mock),
            Arc::new(archive_mock),
            Arc::new(network_mock),
            Arc::new(vm_mock),
            sub_manager,
            metrics,
            rate_limiters,
        ));

        // Start the JSON-RPC server
        let server_handle = tokio::spawn(run_server(app_state));
        sleep(Duration::from_millis(500)).await;

        // Send a request to get_next_block_with_phoenix_transaction
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("http://{}/rpc", ephemeral_addr))
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getNextBlockWithPhoenixTransaction",
                "params": [1000]
            }))
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(resp.status(), StatusCode::OK);
        let resp_json = resp
            .json::<serde_json::Value>()
            .await
            .expect("Failed to parse response as JSON");

        // Verify the response format when no Phoenix transaction is found
        assert_eq!(resp_json["jsonrpc"], "2.0");
        assert_eq!(resp_json["id"], 1);
        assert_eq!(resp_json["result"], serde_json::Value::Null);

        // Cleanup
        server_handle.abort();
    }
}
