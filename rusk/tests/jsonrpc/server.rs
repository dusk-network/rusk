// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Integration tests for the JSON-RPC server startup and basic functionality.

// Use available helpers from utils.rs
use super::utils::{
    create_test_app_state, // Use this helper
    MockArchiveAdapter,
    MockDbAdapter,
    MockNetworkAdapter,
    MockVmAdapter,
};
use assert_matches::assert_matches;
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
use reqwest::StatusCode;
use rusk::jsonrpc::config::{ConfigError, HttpServerConfig, JsonRpcConfig};
use rusk::jsonrpc::error::Error;
use rusk::jsonrpc::infrastructure::manual_limiter::ManualRateLimiters;
use rusk::jsonrpc::infrastructure::metrics::MetricsCollector;
use rusk::jsonrpc::infrastructure::state::AppState;
use rusk::jsonrpc::infrastructure::subscription::manager::SubscriptionManager;
use rusk::jsonrpc::server::run_server;
use rustls::crypto::ring;
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
    // 1. Setup: Use default test AppState, wrapped in Arc
    let app_state = Arc::new(create_test_app_state()); // Wrap in Arc::new()
    let addr = app_state.config().http.bind_address;

    // 2. Action: Spawn server
    let server_handle = tokio::spawn(run_server(app_state.clone()));
    sleep(Duration::from_millis(200)).await; // Increased delay slightly

    // 3. Assert: Check /health
    let client = reqwest::Client::new();
    let health_url = format!("http://{}/health", addr); // Use the default addr
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
async fn test_server_starts_https() {
    // Install default crypto provider for rustls (needed for tests)
    let _ = ring::default_provider().install_default();

    // 1. Setup: Generate certs & Manually create AppState with TLS config
    let (_dir_guard, http_config) =
        generate_tls_certs().expect("Failed to generate certs");
    let mut config = JsonRpcConfig::test_config(); // Start with test defaults
    config.http = http_config; // Overwrite HTTP config with TLS settings
    let app_state = Arc::new(create_custom_app_state(config)); // Wrap in Arc

    let addr = app_state.config().http.bind_address; // Get the fixed TLS port

    // 2. Action: Spawn server
    let server_handle = tokio::spawn(run_server(app_state.clone()));
    sleep(Duration::from_millis(200)).await; // Increased delay slightly

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
