// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Integration tests for the JSON-RPC server startup and basic functionality.

use crate::jsonrpc::utils::get_ephemeral_port;

// Use available helpers from utils.rs
use super::utils::{
    create_test_app_state, create_test_app_state_with_addr, MockArchiveAdapter,
    MockDbAdapter, MockNetworkAdapter, MockVmAdapter,
};
use assert_matches::assert_matches;
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
use reqwest::StatusCode;
use rusk::jsonrpc::config::{
    ConfigError, HttpServerConfig, JsonRpcConfig, RateLimit,
};
use rusk::jsonrpc::error::Error;
use rusk::jsonrpc::infrastructure::manual_limiter::ManualRateLimiters;
use rusk::jsonrpc::infrastructure::metrics::MetricsCollector;
use rusk::jsonrpc::infrastructure::state::AppState;
use rusk::jsonrpc::infrastructure::subscription::manager::SubscriptionManager;
use rusk::jsonrpc::server::run_server;
use rustls::crypto::ring;
use serde_json::json;
use std::fs;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::Duration;
use tempfile::{tempdir, TempDir};
use tokio::time::sleep;

// generate_tls_certs remains mostly the same, but returns HttpServerConfig
fn generate_tls_certs(
) -> Result<(TempDir, HttpServerConfig), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let cert_path = dir.path().join("cert.pem");
    let key_path = dir.path().join("key.pem");

    let mut params = CertificateParams::new(vec!["localhost".to_string()])?;
    params.distinguished_name = DistinguishedName::new();
    params
        .distinguished_name
        .push(DnType::CommonName, "Rusk Test Cert");
    params
        .subject_alt_names
        .push(SanType::IpAddress(IpAddr::V4(Ipv4Addr::LOCALHOST)));

    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    // Use correct rcgen 0.13 methods with 'pem' feature
    let cert_pem = cert.pem(); // Get cert PEM string
    let key_pem = key_pair.serialize_pem(); // Serialize keypair to PEM string

    fs::write(&cert_path, cert_pem)?;
    fs::write(&key_path, key_pem)?;

    let http_config = HttpServerConfig {
        // Use a fixed, likely available port for testing instead of 0
        // If this port is taken, the test will fail, indicating need for a
        // different approach
        bind_address: "127.0.0.1:39989".parse()?,
        cert: Some(cert_path),
        key: Some(key_path),
        ..Default::default()
    };

    Ok((dir, http_config))
}

// Function to manually create AppState with custom JsonRpcConfig
fn create_custom_app_state(config: JsonRpcConfig) -> AppState {
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

#[tokio::test]
async fn test_server_starts_http() {
    // 1. Setup: Get an ephemeral port and configure AppState
    let ephemeral_addr =
        get_ephemeral_port().expect("Failed to get an ephemeral port");
    println!("Using ephemeral port: {}", ephemeral_addr.port());

    let mut config = JsonRpcConfig::test_config(); // Start with test defaults
                                                   // Ensure HTTP config uses the ephemeral address
    config.http.bind_address = ephemeral_addr;
    // Disable TLS for this HTTP test
    config.http.cert = None;
    config.http.key = None;

    let app_state = Arc::new(create_custom_app_state(config));
    // Verify address in config matches the one we obtained
    let addr = app_state.config().http.bind_address;
    assert_eq!(addr, ephemeral_addr, "Config address mismatch");

    // 2. Action: Spawn server
    let mut server_task = tokio::spawn(run_server(app_state.clone()));

    // Give server a small amount of time to bind or fail early
    sleep(Duration::from_millis(200)).await;

    // 3. Assert: Try connecting while monitoring the server task
    let client = reqwest::Client::new();
    let health_url = format!("http://{}/health", addr); // addr is now ephemeral

    // Define the client request future separately
    let client_request_future = async {
        // Allow more time for the server to become fully responsive before
        // sending
        sleep(Duration::from_millis(1300)).await; // Total wait 1500ms
        client.get(&health_url).send().await
    };

    tokio::select! {
        // Biased select ensures we check server_task first if both are ready
        // (though unlikely here)
        biased;

        // Case 1: Server task finishes (or crashes) first
        server_result = &mut server_task => {
            panic!("Server task exited unexpectedly before client could connect: {:?}", server_result);
        }

        // Case 2: Client request completes first
        client_response_result = client_request_future => {
            match client_response_result {
                Ok(response) => {
                    // Client succeeded, check response
                    assert_eq!(response.status(), StatusCode::OK, "Health check status mismatch");
                    assert_eq!(response.text().await.unwrap(), "OK", "Health check body mismatch");
                    println!("Health check successful.");
                    // Now we know the server is up and running, proceed to cleanup
                }
                Err(e) => {
                    // Client failed. The server might still be running or might have crashed
                    // after the initial 200ms sleep but before the client sent the request.
                    // Await the server task to see its fate.
                    match server_task.await {
                        Ok(Ok(())) => panic!(
                            "Client request failed ({:?}), but server task completed successfully?", e
                        ),
                        Ok(Err(server_err)) => panic!(
                            "Client request failed ({:?}). Server task failed: {:?}", e, server_err
                        ),
                        Err(join_err) => panic!(
                            "Client request failed ({:?}). Server task panicked: {:?}", e, join_err
                        ),
                    }
                }
            }
        }

        // Case 3: Timeout (shouldn't happen with reasonable sleep times)
        _ = sleep(Duration::from_secs(10)) => {
            panic!("Test timed out waiting for client request or server exit.");
        }
    }

    // 4. Cleanup (only reached if client request succeeded in the select!
    //    block)
    println!("Aborting server task after successful health check.");
    server_task.abort();
    // Optionally await the aborted handle to ensure cleanup and check for
    // errors during shutdown
    match server_task.await {
        Ok(Ok(())) => println!("Server task finished ok after abort."), /* Expected if shutdown is graceful */
        Ok(Err(e)) => {
            eprintln!("Server task finished with error after abort: {}", e)
        }
        Err(e) if e.is_cancelled() => {
            println!("Server task cancelled successfully.")
        } // Expected
        Err(e) => {
            eprintln!("Server task panicked during abort/shutdown: {:?}", e)
        }
    }
}

#[tokio::test]
async fn test_server_starts_https() {
    // Install default crypto provider for rustls (needed for tests)
    let _ = ring::default_provider().install_default();

    // 1. Setup: Generate certs & Manually create AppState with TLS config
    let (_dir_guard, mut http_config) =
        generate_tls_certs().expect("Failed to generate certs");
    // Explicitly set a known port for the HTTPS test
    let https_test_port = 39989;
    http_config.bind_address =
        format!("127.0.0.1:{}", https_test_port).parse().unwrap();

    let config = JsonRpcConfig {
        // Overwrite HTTP config with TLS settings
        http: http_config,
        ..JsonRpcConfig::default()
    };
    let app_state = Arc::new(create_custom_app_state(config)); // Wrap in Arc

    let addr = app_state.config().http.bind_address; // Get the fixed TLS port
    assert_eq!(
        addr.port(),
        https_test_port,
        "Test assumes fixed HTTPS port"
    );

    // 2. Action: Spawn server
    let server_handle = tokio::spawn(run_server(app_state.clone()));
    sleep(Duration::from_millis(1500)).await;

    // 3. Assert: Check /health via HTTPS
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true) // ONLY FOR TESTING!
        .build()
        .unwrap();
    let health_url = format!("https://{}/health", addr); // Use https and fixed port
    let response = client.get(&health_url).send().await;

    assert!(response.is_ok(), "Request failed: {:?}", response.err());
    if let Ok(resp) = response {
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.text().await.unwrap(), "OK");
    } // Only assert on Ok response

    // 4. Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_tls_config_invalid_path() {
    // 1. Setup: Manually create AppState with invalid paths
    let mut config = JsonRpcConfig::test_config();
    config.http.cert = Some("/tmp/nonexistent/cert.pem".into());
    config.http.key = Some("/tmp/nonexistent/key.pem".into());
    let app_state = Arc::new(create_custom_app_state(config));

    // 2. Action & Assert: Run server directly and check error
    let result = run_server(app_state).await;
    assert_matches!(result, Err(Error::Config(ConfigError::FileRead(_))));
}

#[tokio::test]
async fn test_tls_config_invalid_cert_format() {
    // 1. Setup: Manually create AppState with invalid cert
    let dir = tempdir().unwrap();
    let cert_path = dir.path().join("invalid_cert.pem");
    let key_path = dir.path().join("valid_key.pem");

    let key_pair_params =
        CertificateParams::new(vec!["localhost".to_string()]).unwrap();
    let key_pair = KeyPair::generate().unwrap();
    let _cert_for_key = key_pair_params.self_signed(&key_pair).unwrap();
    let key_pem = key_pair.serialize_pem(); // Serialize keypair
    fs::write(&key_path, key_pem).unwrap();
    fs::write(&cert_path, "this is not a valid pem certificate").unwrap();

    let mut config = JsonRpcConfig::test_config();
    config.http.cert = Some(cert_path);
    config.http.key = Some(key_path);
    let app_state = Arc::new(create_custom_app_state(config));

    // 2. Action & Assert: Run server directly and check error
    let result = run_server(app_state).await;
    assert_matches!(
        result,
        Err(Error::Config(ConfigError::Validation(_))),
        "Expected Validation error for invalid cert format, got {:?}",
        result
    );
}

#[tokio::test]
async fn test_tls_config_invalid_key_format() {
    // 1. Setup: Manually create AppState with invalid key
    let dir = tempdir().unwrap();
    let cert_path = dir.path().join("valid_cert.pem");
    let key_path = dir.path().join("invalid_key.pem");

    let params = CertificateParams::new(vec!["localhost".to_string()]).unwrap();
    let key_pair = KeyPair::generate().unwrap();
    let cert_data = params.self_signed(&key_pair).unwrap();
    let cert_pem = cert_data.pem();
    fs::write(&cert_path, cert_pem).unwrap();
    fs::write(&key_path, "this is not a valid pem private key").unwrap();

    let mut config = JsonRpcConfig::test_config();
    config.http.cert = Some(cert_path);
    config.http.key = Some(key_path);
    let app_state = Arc::new(create_custom_app_state(config));

    // 2. Action & Assert: Run server directly and check error
    let result = run_server(app_state).await;
    assert_matches!(
        result,
        Err(Error::Config(ConfigError::Validation(_))),
        "Expected Validation error for invalid key format, got {:?}",
        result
    );
}

#[tokio::test]
async fn test_tls_config_partial_config_cert_only() {
    // 1. Setup: Manually create AppState with only cert path
    let dir = tempdir().unwrap();
    let cert_path = dir.path().join("cert.pem");
    fs::write(&cert_path, "dummy cert content").unwrap();

    let mut config = JsonRpcConfig::test_config();
    config.http.cert = Some(cert_path);
    config.http.key = None; // Key is None
    let app_state = Arc::new(create_custom_app_state(config));

    // 2. Action & Assert: Run server directly and check validation error
    let result = run_server(app_state).await;
    assert_matches!(result, Err(Error::Config(ConfigError::Validation(msg))) if msg.contains("key path is missing"));
}

#[tokio::test]
async fn test_tls_config_partial_config_key_only() {
    // 1. Setup: Manually create AppState with only key path
    let dir = tempdir().unwrap();
    let key_path = dir.path().join("key.pem");
    fs::write(&key_path, "dummy key content").unwrap();

    let mut config = JsonRpcConfig::test_config();
    config.http.cert = None; // Cert is None
    config.http.key = Some(key_path);
    let app_state = Arc::new(create_custom_app_state(config));

    // 2. Action & Assert: Run server directly and check validation error
    let result = run_server(app_state).await;
    assert_matches!(result, Err(Error::Config(ConfigError::Validation(msg))) if msg.contains("certificate path is missing"));
}

#[tokio::test]
async fn test_cors_headers() {
    // 1. Setup: Use a unique port for this test
    let cors_test_port = 39997;
    let cors_addr: std::net::SocketAddr =
        format!("127.0.0.1:{}", cors_test_port).parse().unwrap();
    let app_state = Arc::new(create_test_app_state_with_addr(Some(cors_addr)));

    // Verify the port is set correctly in the config
    assert_eq!(
        app_state.config().http.bind_address,
        cors_addr,
        "Test CORS port mismatch"
    );

    // 2. Action: Spawn server
    let mut server_task = tokio::spawn(run_server(app_state.clone()));

    // Give server a small amount of time to bind or fail early
    sleep(Duration::from_millis(200)).await;

    // 3. Assert: Try connecting while monitoring the server task
    let client = reqwest::Client::new();
    // Use the specific cors_addr for the request URL
    let health_url = format!("http://{}/health", cors_addr);

    let client_request_future = async {
        sleep(Duration::from_millis(1300)).await; // Total wait 1500ms
        client
            .get(&health_url)
            .header("Origin", "http://example.com")
            .send()
            .await
    };

    tokio::select! {
        biased;

        server_result = &mut server_task => {
            panic!("CORS Test: Server task exited unexpectedly: {:?}", server_result);
        }

        client_response_result = client_request_future => {
            match client_response_result {
                Ok(response) => {
                    assert_eq!(response.status(), StatusCode::OK, "CORS Test: Health check status mismatch");
                    // Check for permissive CORS headers
                    assert_eq!(
                        response
                            .headers()
                            .get("access-control-allow-origin")
                            .expect("Missing access-control-allow-origin header"),
                        "*"
                    );
                     println!("CORS Test: Health check and headers successful.");
                }
                Err(e) => {
                    match server_task.await {
                        Ok(Ok(())) => panic!(
                            "CORS Test: Client request failed ({:?}), but server task completed successfully?", e
                        ),
                        Ok(Err(server_err)) => panic!(
                            "CORS Test: Client request failed ({:?}). Server task failed: {:?}", e, server_err
                        ),
                        Err(join_err) => panic!(
                            "CORS Test: Client request failed ({:?}). Server task panicked: {:?}", e, join_err
                        ),
                    }
                }
            }
        }

        _ = sleep(Duration::from_secs(10)) => {
            panic!("CORS Test: Timed out waiting for client request or server exit.");
        }
    }

    // 4. Cleanup
    println!("CORS Test: Aborting server task.");
    server_task.abort();
    match server_task.await {
        Ok(Ok(())) => {
            println!("CORS Test: Server task finished ok after abort.")
        }
        Ok(Err(e)) => eprintln!(
            "CORS Test: Server task finished with error after abort: {}",
            e
        ),
        Err(e) if e.is_cancelled() => {
            println!("CORS Test: Server task cancelled successfully.")
        }
        Err(e) => eprintln!(
            "CORS Test: Server task panicked during abort/shutdown: {:?}",
            e
        ),
    }
}

#[tokio::test]
async fn test_rate_limiting() {
    // 1. Setup: Configure a strict rate limit and a unique port
    let rate_limit_test_port = 39998; // Keep existing unique port
    let rate_limit_addr: std::net::SocketAddr =
        format!("127.0.0.1:{}", rate_limit_test_port)
            .parse()
            .unwrap();

    let mut config = JsonRpcConfig::default(); // Start with defaults
    config.rate_limit.enabled = true;
    config.rate_limit.default_limit = RateLimit {
        requests: 2,                        // Allow only 2 requests...
        window: Duration::from_millis(500), // ...within a 500ms window
    };
    config.http.bind_address = rate_limit_addr; // Set the unique address

    let app_state = Arc::new(create_custom_app_state(config));
    assert_eq!(
        app_state.config().http.bind_address,
        rate_limit_addr,
        "Test rate limit port mismatch"
    );

    // 2. Action: Spawn server
    let server_task = tokio::spawn(run_server(app_state.clone()));

    // Give server time to start or fail
    sleep(Duration::from_millis(1500)).await;

    // Check if server task failed early
    if server_task.is_finished() {
        match server_task.await {
            Ok(Err(e)) => {
                panic!("Rate Limit Test: Server task failed early: {:?}", e)
            }
            Err(e) => {
                panic!("Rate Limit Test: Server task panicked early: {:?}", e)
            }
            _ => panic!("Rate Limit Test: Server exited early unexpectedly"),
        }
    }

    // 3. Assert: Perform rate limit checks
    let client = reqwest::Client::new();
    let health_url = format!("http://{}/health", rate_limit_addr);

    // Send 2 requests - should succeed
    let resp1 = client.get(&health_url).send().await.expect("Req 1 failed");
    assert_eq!(resp1.status(), StatusCode::OK, "Req 1 status mismatch");
    let resp2 = client.get(&health_url).send().await.expect("Req 2 failed");
    assert_eq!(resp2.status(), StatusCode::OK, "Req 2 status mismatch");

    // Send 3rd request - should be rate limited (429)
    let resp3 = client.get(&health_url).send().await.expect("Req 3 failed");
    assert_eq!(
        resp3.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "Req 3 should be rate limited"
    );

    // Wait for the window to reset
    sleep(Duration::from_millis(600)).await; // Wait longer than window

    // Send 4th request - should succeed again
    let resp4 = client.get(&health_url).send().await.expect("Req 4 failed");
    assert_eq!(resp4.status(), StatusCode::OK, "Req 4 status mismatch");

    // 4. Cleanup
    println!("Rate Limit Test: Aborting server task.");
    server_task.abort();
    match server_task.await {
        Ok(Ok(())) => {
            println!("Rate Limit Test: Server task finished ok after abort.")
        }
        Ok(Err(e)) => eprintln!(
            "Rate Limit Test: Server task finished with error after abort: {}",
            e
        ),
        Err(e) if e.is_cancelled() => {
            println!("Rate Limit Test: Server task cancelled successfully.")
        }
        Err(e) => eprintln!(
            "Rate Limit Test: Server task panicked during abort/shutdown: {:?}",
            e
        ),
    }
}

#[tokio::test]
async fn test_rpc_call_get_node_info() {
    // 1. Setup: Use helper to create AppState with default config (HTTP)
    let app_state = Arc::new(create_test_app_state());
    let addr = app_state.config().http.bind_address;
    assert_eq!(addr.port(), 8546, "Test assumes default HTTP port 8546");

    // 2. Action: Spawn server
    let mut server_task = tokio::spawn(run_server(app_state.clone()));

    // Allow time for server to start
    sleep(Duration::from_millis(1500)).await;

    // 3. Assert: Send RPC request and verify response
    let client = reqwest::Client::new();
    let rpc_url = format!("http://{}/rpc", addr);

    let request_body = json!({
        "jsonrpc": "2.0",
        "method": "getNodeInfo", // Use the correct namespaced method name
        "params": [],
        "id": 1
    });

    let client_request_future =
        client.post(&rpc_url).json(&request_body).send();

    tokio::select! {
        biased;
        // Case 1: Server task finishes (or crashes) first
        server_result = &mut server_task => {
            panic!("Server task exited unexpectedly before RPC client could connect: {:?}", server_result);
        }
        // Case 2: Client request completes first
        client_response_result = client_request_future => {
            match client_response_result {
                Ok(response) => {
                    assert_eq!(response.status(), StatusCode::OK, "RPC request status mismatch");
                    let body_text = response.text().await.unwrap_or_else(|_| "Failed to get body text".to_string());
                    println!("RPC Response Body: {}", body_text);
                    let body_json: serde_json::Value = serde_json::from_str(&body_text).expect("Failed to parse RPC response as JSON");

                    assert!(body_json.get("error").is_none(), "RPC response contained an error: {:#?}", body_json["error"]);
                    assert!(body_json.get("result").is_some(), "RPC response missing 'result' field");
                    assert_eq!(body_json["id"], 1, "RPC response ID mismatch");

                    // Verify structure of the 'result' object (NodeInfo)
                    let result = &body_json["result"];
                    assert!(result["version"].is_string(), "Result 'version' field is not a string");
                    assert!(result["version_build"].is_string(), "Result 'version_build' field is not a string");
                    assert!(result["network_id"].is_number(), "Result 'network_id' field is not a number");
                    assert!(result["public_address"].is_string(), "Result 'public_address' field is not a string");
                    assert!(result["bootstrap_nodes"].is_array(), "Result 'bootstrap_nodes' field is not an array");
                    // Check specific values if necessary (e.g., default network_id)
                    // Note: This assumes the test runs without the 'chain' feature enabled, or that the mock VmAdapter returns 0.
                    assert_eq!(result["network_id"].as_u64().unwrap_or(999), 0, "Expected default network_id 0"); // Use unwrap_or for robustness
                    assert_eq!(result["public_address"].as_str().unwrap(), "127.0.0.1:9000".to_string(), "Public address mismatch with the value from the MockNetworkAdapter");


                    let vm_config = &result["vm_config"];
                    assert!(vm_config.is_object());
                    assert!(vm_config["block_gas_limit"].is_string());
                    assert_eq!(
                        vm_config["block_gas_limit"].as_str().unwrap().to_string(),
                        app_state.get_vm_config().await.unwrap().block_gas_limit.to_string(),
                        "Block gas limit mismatch"
                    );
                    assert!(vm_config["gas_per_deploy_byte"].is_string());
                    assert_eq!(
                        vm_config["gas_per_deploy_byte"].as_str().unwrap().to_string(),
                        app_state.get_vm_config().await.unwrap().gas_per_deploy_byte.to_string(),
                        "Gas per deploy byte mismatch"
                    );
                    assert!(vm_config["min_deploy_points"].is_string());
                    assert_eq!(
                        vm_config["min_deploy_points"].as_str().unwrap().to_string(),
                        app_state.get_vm_config().await.unwrap().min_deploy_points.to_string(),
                        "Minimum deploy points mismatch"
                    );
                    assert!(vm_config["min_deployment_gas_price"].is_string());
                    assert_eq!(
                        vm_config["min_deployment_gas_price"].as_str().unwrap().to_string(),
                        app_state.get_vm_config().await.unwrap().min_deployment_gas_price.to_string(),
                        "Minimum deployment gas price mismatch"
                    );
                    assert!(vm_config["generation_timeout"].is_string());

                    let mut generation_timeout = app_state
                        .get_vm_config()
                        .await
                        .unwrap()
                        .generation_timeout
                        .unwrap()
                        .as_secs()
                        .to_string();
                    generation_timeout.push('s');
                    assert_eq!(
                        vm_config["generation_timeout"].as_str().unwrap().to_string(),
                        generation_timeout,
                        "Generation timeout mismatch"
                    );

                    println!("RPC call rusk_getNodeInfo successful.");
                }
                Err(e) => {
                    // Client failed. Await the server task to see its fate.
                     match server_task.await {
                        Ok(Ok(())) => panic!(
                            "RPC Client request failed ({:?}), but server task completed successfully?", e
                        ),
                        Ok(Err(server_err)) => panic!(
                            "RPC Client request failed ({:?}). Server task failed: {:?}", e, server_err
                        ),
                        Err(join_err) => panic!(
                            "RPC Client request failed ({:?}). Server task panicked: {:?}", e, join_err
                        ),
                    }
                }
            }
        }
        // Case 3: Timeout
         _ = sleep(Duration::from_secs(10)) => {
            panic!("Test timed out waiting for RPC client request or server exit.");
        }
    }

    // 4. Cleanup (only reached if client request succeeded)
    println!("Aborting server task after successful RPC call.");
    server_task.abort();
    match server_task.await {
        Ok(Ok(())) => println!("Server task finished ok after abort."),
        Ok(Err(e)) => {
            eprintln!("Server task finished with error after abort: {}", e)
        }
        Err(e) if e.is_cancelled() => {
            println!("Server task cancelled successfully.")
        }
        Err(e) => {
            eprintln!("Server task panicked during abort/shutdown: {:?}", e)
        }
    }
}

#[tokio::test]
async fn test_rpc_method_via_server() {
    // 1. Setup: Start server on an ephemeral port
    let ephemeral_addr =
        get_ephemeral_port().expect("Failed to get ephemeral port");
    println!(
        "Integration test using ephemeral port: {}",
        ephemeral_addr.port()
    );

    let mut config = JsonRpcConfig::test_config();
    config.http.bind_address = ephemeral_addr;
    config.http.cert = None; // Ensure HTTP for this test
    config.http.key = None;

    let app_state = Arc::new(create_custom_app_state(config));
    let addr = app_state.config().http.bind_address;

    // 2. Action: Spawn server
    let mut server_task = tokio::spawn(run_server(app_state.clone()));

    // Allow time for server to start
    sleep(Duration::from_millis(1500)).await;

    // 3. Assert: Send RPC request via HTTP and verify response
    let client = reqwest::Client::new();
    let rpc_url = format!("http://{}/rpc", addr);

    let request_body = json!({
        "jsonrpc": "2.0",
        "method": "getNodeInfo",
        "params": [],
        "id": "test-integration-rpc-1"
    });

    let client_request_future =
        client.post(&rpc_url).json(&request_body).send();

    tokio::select! {
        biased;
        server_result = &mut server_task => {
            panic!("Server task exited unexpectedly (RPC method test): {:?}", server_result);
        }
        client_response_result = client_request_future => {
            match client_response_result {
                Ok(response) => {
                    assert_eq!(response.status(), StatusCode::OK, "RPC method request status mismatch");
                    let body_text = response.text().await.unwrap_or_else(|_| "Failed to get body text".to_string());
                    println!("RPC Method Response Body: {}", body_text);
                    let body_json: serde_json::Value = serde_json::from_str(&body_text).expect("Failed to parse RPC method response as JSON");

                    assert!(body_json.get("error").is_none(), "RPC method response contained an error: {:#?}", body_json.get("error"));
                    assert!(body_json.get("result").is_some(), "RPC method response missing 'result' field");
                    assert_eq!(body_json["id"], "test-integration-rpc-1", "RPC method response ID mismatch");

                    let result = &body_json["result"];
                    assert!(result["version"].is_string());
                    assert!(result["version_build"].is_string());
                    assert!(result["network_id"].is_number(), "Result 'network_id' field is not a number");
                    assert_eq!(result["network_id"].as_u64().unwrap_or(999), 0, "Expected default network_id 0");
                    assert_eq!(result["public_address"].as_str().unwrap(), "127.0.0.1:9000".to_string(), "Public address mismatch with the value from the MockNetworkAdapter");

                    let vm_config = &result["vm_config"];
                    assert!(vm_config.is_object());
                    assert!(vm_config["block_gas_limit"].is_string());
                    assert_eq!(
                        vm_config["block_gas_limit"].as_str().unwrap().to_string(),
                        app_state.get_vm_config().await.unwrap().block_gas_limit.to_string(),
                        "Block gas limit mismatch"
                    );
                    assert!(vm_config["gas_per_deploy_byte"].is_string());
                    assert_eq!(
                        vm_config["gas_per_deploy_byte"].as_str().unwrap().to_string(),
                        app_state.get_vm_config().await.unwrap().gas_per_deploy_byte.to_string(),
                        "Gas per deploy byte mismatch"
                    );
                    assert!(vm_config["min_deploy_points"].is_string());
                    assert_eq!(
                        vm_config["min_deploy_points"].as_str().unwrap().to_string(),
                        app_state.get_vm_config().await.unwrap().min_deploy_points.to_string(),
                        "Minimum deploy points mismatch"
                    );
                    assert!(vm_config["min_deployment_gas_price"].is_string());
                    assert_eq!(
                        vm_config["min_deployment_gas_price"].as_str().unwrap().to_string(),
                        app_state.get_vm_config().await.unwrap().min_deployment_gas_price.to_string(),
                        "Minimum deployment gas price mismatch"
                    );
                    assert!(vm_config["generation_timeout"].is_string());

                    let mut generation_timeout = app_state
                        .get_vm_config()
                        .await
                        .unwrap()
                        .generation_timeout
                        .unwrap()
                        .as_secs()
                        .to_string();
                    generation_timeout.push('s');
                    assert_eq!(
                        vm_config["generation_timeout"].as_str().unwrap().to_string(),
                        generation_timeout,
                        "Generation timeout mismatch"
                    );

                    println!("RPC method call rusk_getNodeInfo via server successful.");
                }
                Err(e) => {
                     match server_task.await {
                        Ok(Ok(())) => panic!("RPC Method Client request failed ({:?}), but server task OK?", e),
                        Ok(Err(server_err)) => panic!("RPC Method Client request failed ({:?}). Server failed: {:?}", e, server_err),
                        Err(join_err) => panic!("RPC Method Client request failed ({:?}). Server panicked: {:?}", e, join_err),
                    }
                }
            }
        }
         _ = sleep(Duration::from_secs(10)) => {
            panic!("Test timed out waiting for RPC method client request or server exit.");
        }
    }

    // 4. Cleanup
    println!("Aborting server task after successful RPC method call.");
    server_task.abort();
    // Suppress shutdown errors/cancellations in test output for brevity
    let _ = server_task.await;
}
