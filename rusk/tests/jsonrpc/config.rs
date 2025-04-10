// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::env;
use std::sync::Mutex;
use std::time::Duration;

use lazy_static::lazy_static;
use serde::Deserialize;
use tempfile::NamedTempFile;

use rusk::jsonrpc::config::{
    ConfigError, CorsConfig, JsonRpcConfig, MethodRateLimit, RateLimit,
};

use crate::jsonrpc::utils::{assert_security_error, create_environment_config};

lazy_static! {
    static ref ENV_MUTEX: Mutex<()> = Mutex::new(());
}
struct EnvVarGuard<'a> {
    key: &'a str,
    original_value: Option<String>,
}

impl<'a> EnvVarGuard<'a> {
    fn set(key: &'a str, value: &str) -> Self {
        let original_value = env::var(key).ok();
        env::set_var(key, value);
        Self {
            key,
            original_value,
        }
    }
}

impl<'a> Drop for EnvVarGuard<'a> {
    fn drop(&mut self) {
        if let Some(ref val) = self.original_value {
            env::set_var(self.key, val);
        } else {
            env::remove_var(self.key);
        }
    }
}
fn set_env_vars<'a>(vars: &'a [(&'a str, &'a str)]) -> Vec<EnvVarGuard<'a>> {
    vars.iter().map(|(k, v)| EnvVarGuard::set(k, v)).collect()
}

#[test]
fn test_default_config() {
    let config = JsonRpcConfig::default();
    assert_eq!(config.http.bind_address.port(), 8546);
    assert_eq!(config.ws.bind_address.port(), 8547);
    assert!(config.rate_limit.enabled);
    assert!(config.features.enable_websocket);
    assert_eq!(config.features.max_block_range, 1000);
    assert!(config.sanitization.enabled);
    assert_eq!(config.sanitization.max_message_length, 200);
}

#[test]
fn test_config_file_roundtrip() -> Result<(), ConfigError> {
    let config = JsonRpcConfig::default();
    let file = NamedTempFile::new().expect("Failed to create temp file");
    let path = file.path();

    // Write config to file
    config.to_file(path)?;

    // Read config from file using the load method that skips env vars
    let loaded_config = JsonRpcConfig::load_from_file_only(Some(path))?;

    assert_eq!(loaded_config.http.bind_address, config.http.bind_address);
    assert_eq!(
        loaded_config.features.max_block_range,
        1000, // Default value
        "Loaded max_block_range should match the default written to the file"
    );
    // Now compare loaded vs original default object
    assert_eq!(
        loaded_config.features.max_block_range, config.features.max_block_range,
        "Loaded max_block_range should equal original default object's value"
    );

    Ok(())
}

#[test]
fn test_env_config() {
    let _lock = ENV_MUTEX.lock().unwrap();

    let _guards = set_env_vars(&[
        ("RUSK_JSONRPC_HTTP_BIND_ADDRESS", "127.0.0.1:9000"),
        ("RUSK_JSONRPC_FEATURE_MAX_BLOCK_RANGE", "500"),
        ("RUSK_JSONRPC_RATE_LIMIT_ENABLED", "false"),
        ("RUSK_JSONRPC_SANITIZATION_ENABLED", "false"),
    ]);

    let config = JsonRpcConfig::load(None).unwrap();

    assert_eq!(config.http.bind_address.port(), 9000);
    assert_eq!(config.features.max_block_range, 500);
    assert!(!config.rate_limit.enabled);
    assert!(!config.sanitization.enabled);
}

#[test]
fn test_complex_env_vars() {
    let _lock = ENV_MUTEX.lock().unwrap();

    let _guards = set_env_vars(&[
        (
            "RUSK_JSONRPC_CORS_ALLOWED_ORIGINS",
            "https://example.com,https://test.com",
        ),
        ("RUSK_JSONRPC_CORS_ALLOWED_METHODS", ""),
        ("RUSK_JSONRPC_FEATURE_DETAILED_ERRORS", "TRUE"),
        ("RUSK_JSONRPC_RATE_LIMIT_ENABLED", "yes"),
    ]);

    let config = JsonRpcConfig::load(None).unwrap();

    assert_eq!(config.http.cors.allowed_origins.len(), 2);
    assert_eq!(config.http.cors.allowed_origins[0], "https://example.com");
    assert!(config.http.cors.allowed_methods.is_empty());
    assert!(config.features.detailed_errors);
    assert!(!config.rate_limit.enabled);
}

#[test]
fn test_malformed_env_vars() {
    let _lock = ENV_MUTEX.lock().expect("Mutex should not be poisoned");

    let _guards = set_env_vars(&[
        ("RUSK_JSONRPC_HTTP_BIND_ADDRESS", "not-an-address"),
        ("RUSK_JSONRPC_HTTP_REQUEST_TIMEOUT_SECS", "not-a-number"),
    ]);

    let config = JsonRpcConfig::load(None)
        .expect("Loading config should succeed even with malformed env vars");

    assert_eq!(config.http.bind_address.port(), 8546);
    assert_eq!(config.http.request_timeout.as_secs(), 30);
}

#[test]
fn test_load_with_precedence() -> Result<(), ConfigError> {
    let _lock = ENV_MUTEX.lock().unwrap();

    let mut config = JsonRpcConfig::default();
    config.http.bind_address = "127.0.0.1:8000".parse().unwrap();
    config.features.max_block_range = 300;

    let file = NamedTempFile::new().expect("Failed to create temp file");
    let path = file.path();
    config.to_file(path)?;

    let _guard =
        EnvVarGuard::set("RUSK_JSONRPC_HTTP_BIND_ADDRESS", "127.0.0.1:9000");

    let loaded_config = JsonRpcConfig::load(Some(path))?;

    assert_eq!(loaded_config.http.bind_address.port(), 9000);
    assert_eq!(loaded_config.features.max_block_range, 300);

    Ok(())
}

#[test]
fn test_complete_precedence_chain() -> Result<(), Box<dyn std::error::Error>> {
    let _lock = ENV_MUTEX.lock().unwrap();

    let file = NamedTempFile::new()?;
    std::fs::write(
        file.path(),
        r#"
        [jsonrpc]
          [jsonrpc.features]
          max_block_range = 500
    "#,
    )?;

    let guard = EnvVarGuard::set("RUSK_JSONRPC_FEATURE_MAX_BLOCK_RANGE", "200");

    let config = JsonRpcConfig::load(Some(file.path()))?;

    assert_eq!(config.features.max_block_range, 200);

    drop(guard);
    let config_after_cleanup = JsonRpcConfig::load(Some(file.path()))?;

    assert_eq!(config_after_cleanup.features.max_block_range, 500);

    let config_default = JsonRpcConfig::load(None)?;
    assert_eq!(config_default.features.max_block_range, 1000);

    Ok(())
}

#[test]
fn test_toml_format() -> Result<(), Box<dyn std::error::Error>> {
    let config = JsonRpcConfig::default();
    let toml_str = config.to_toml_string()?;

    assert!(
        toml_str.contains("[jsonrpc.http]"),
        "TOML string should contain [jsonrpc.http]"
    );
    assert!(
        toml_str.contains("[jsonrpc.ws]"),
        "TOML string should contain [jsonrpc.ws]"
    );
    assert!(toml_str.contains("[jsonrpc.rate_limit]"));
    assert!(toml_str.contains("[jsonrpc.features]"));
    assert!(toml_str.contains("[jsonrpc.sanitization]"));

    let parsed_wrapper: RuskConfigFile = toml::from_str(&toml_str)?;
    assert_eq!(
        parsed_wrapper.jsonrpc.http.bind_address,
        config.http.bind_address
    );

    Ok(())
}

#[test]
fn test_cors_config() -> Result<(), Box<dyn std::error::Error>> {
    let default_cors = CorsConfig::default();
    assert!(default_cors.enabled);
    assert!(default_cors.allowed_origins.is_empty());
    assert_eq!(default_cors.max_age_seconds, 86400);

    let custom_cors = CorsConfig {
        enabled: true,
        allowed_origins: vec!["https://example.com".to_string()],
        allowed_methods: vec!["POST".to_string()],
        allowed_headers: vec!["Content-Type".to_string()],
        allow_credentials: true,
        max_age_seconds: 3600,
    };
    assert_eq!(custom_cors.allowed_origins.len(), 1);
    assert_eq!(custom_cors.allowed_origins[0], "https://example.com");

    let file = NamedTempFile::new()?;
    let toml_content = r#"
        [jsonrpc]
          [jsonrpc.http.cors]
          enabled = false
          allowed_origins = ["https://test1.com", "https://test2.com"]
          allowed_methods = ["GET", "OPTIONS"]
          allowed_headers = ["X-Custom-Header"]
          allow_credentials = true
          max_age_seconds = 7200
    "#;
    std::fs::write(file.path(), toml_content)?;
    let loaded_config = JsonRpcConfig::load(Some(file.path()))?;
    let loaded_cors = &loaded_config.http.cors;

    assert!(!loaded_cors.enabled);
    assert_eq!(loaded_cors.allowed_origins.len(), 2);
    assert_eq!(loaded_cors.allowed_origins[0], "https://test1.com");
    assert_eq!(loaded_cors.allowed_methods.len(), 2);
    assert_eq!(loaded_cors.allowed_methods[0], "GET");
    assert_eq!(loaded_cors.allowed_headers.len(), 1);
    assert_eq!(loaded_cors.allowed_headers[0], "X-Custom-Header");
    assert!(loaded_cors.allow_credentials);
    assert_eq!(loaded_cors.max_age_seconds, 7200);

    let serialized_toml = loaded_config.to_toml_string()?;
    let deserialized_wrapper: RuskConfigFile =
        toml::from_str(&serialized_toml)?;
    let deserialized_cors = &deserialized_wrapper.jsonrpc.http.cors;
    assert_eq!(
        deserialized_cors.enabled, loaded_cors.enabled,
        "CORS enabled mismatch after serialization roundtrip"
    );
    assert_eq!(
        deserialized_cors.allowed_origins, loaded_cors.allowed_origins,
        "CORS allowed_origins mismatch after serialization roundtrip"
    );
    assert_eq!(
        deserialized_cors.allowed_methods, loaded_cors.allowed_methods,
        "CORS allowed_methods mismatch after serialization roundtrip"
    );
    assert_eq!(
        deserialized_cors.allowed_headers, loaded_cors.allowed_headers,
        "CORS allowed_headers mismatch after serialization roundtrip"
    );
    assert_eq!(
        deserialized_cors.allow_credentials, loaded_cors.allow_credentials,
        "CORS allow_credentials mismatch after serialization roundtrip"
    );
    assert_eq!(
        deserialized_cors.max_age_seconds, loaded_cors.max_age_seconds,
        "CORS max_age_seconds mismatch after serialization roundtrip"
    );

    {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvVarGuard::set("RUSK_JSONRPC_CORS_ENABLED", "true");
        let config_with_env = JsonRpcConfig::load(Some(file.path()))?;
        assert!(
            config_with_env.http.cors.enabled,
            "CORS should be enabled by env var"
        );
        assert_eq!(config_with_env.http.cors.max_age_seconds, 7200);
    }

    Ok(())
}

#[test]
fn test_invalid_toml_file() -> Result<(), Box<dyn std::error::Error>> {
    let file = NamedTempFile::new()?;
    std::fs::write(file.path(), "invalid [ toml syntax")?;

    let result = JsonRpcConfig::load(Some(file.path()));
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ConfigError::TomlParse(_)));

    Ok(())
}

#[test]
fn test_partial_config_file() -> Result<(), Box<dyn std::error::Error>> {
    let file = NamedTempFile::new()?;
    std::fs::write(
        file.path(),
        r#"
        [jsonrpc]
          [jsonrpc.http]
          max_body_size = 5242880

          [jsonrpc.features]
          max_block_range = 500
    "#,
    )?;

    let config = JsonRpcConfig::load(Some(file.path()))?;

    assert_eq!(config.http.max_body_size, 5242880);
    assert_eq!(config.features.max_block_range, 500);

    let _lock = ENV_MUTEX.lock().expect("Mutex should not be poisoned");
    env::remove_var("RUSK_JSONRPC_HTTP_BIND_ADDRESS");
    let config = JsonRpcConfig::load(Some(file.path()))?;

    assert_eq!(
        config.http.bind_address.port(),
        8546,
        "Port should default to 8546 when not in file or env"
    );
    assert_eq!(config.http.request_timeout.as_secs(), 30);
    assert_eq!(config.ws.max_connections, 50);

    Ok(())
}

#[test]
fn test_nonexistent_config_file() -> Result<(), Box<dyn std::error::Error>> {
    let _lock = ENV_MUTEX.lock().unwrap();

    let path = std::path::Path::new("/path/that/doesnt/exist/config.toml");

    let config = JsonRpcConfig::load(Some(path))?;
    assert_eq!(config.http.bind_address.port(), 8546);
    assert_eq!(config.features.max_block_range, 1000);
    assert!(config.sanitization.enabled);

    Ok(())
}

#[test]
fn test_config_path_resolution() {
    let root = JsonRpcConfig::project_root();

    assert!(
        root.join("Cargo.toml").exists(),
        "Project root should contain Cargo.toml: {:?}",
        root
    );
    let config_path = JsonRpcConfig::default_config_path();
    assert_eq!(
        config_path.file_name().unwrap().to_str().unwrap(),
        JsonRpcConfig::DEFAULT_CONFIG_FILENAME
    );
}

#[test]
fn test_custom_socket_addr_serialization(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = JsonRpcConfig::default();
    config.http.bind_address = "192.168.1.1:9000".parse()?;
    config.ws.bind_address = "[::1]:9001".parse()?;

    let toml_str = config.to_toml_string()?;

    assert!(toml_str.contains("bind_address = \"192.168.1.1:9000\""));
    assert!(toml_str.contains("bind_address = \"[::1]:9001\""));

    let parsed_wrapper: RuskConfigFile = toml::from_str(&toml_str)?;
    let parsed = parsed_wrapper.jsonrpc;
    assert_eq!(parsed.http.bind_address.to_string(), "192.168.1.1:9000");
    assert_eq!(parsed.ws.bind_address.to_string(), "[::1]:9001");

    Ok(())
}

#[test]
fn test_custom_duration_serialization() -> Result<(), Box<dyn std::error::Error>>
{
    let mut config = JsonRpcConfig::default();
    config.http.request_timeout = Duration::from_secs(120);
    config.ws.idle_timeout = Duration::from_secs(600);

    let toml_str = config.to_toml_string()?;

    assert!(toml_str.contains("request_timeout = 120"));
    assert!(toml_str.contains("idle_timeout = 600"));

    let parsed_wrapper: RuskConfigFile = toml::from_str(&toml_str)?;
    let parsed = parsed_wrapper.jsonrpc;
    assert_eq!(parsed.http.request_timeout.as_secs(), 120);
    assert_eq!(parsed.ws.idle_timeout.as_secs(), 600);

    Ok(())
}

#[test]
fn test_config_builder_api() {
    let config = JsonRpcConfig::builder()
        .http_bind_address("127.0.0.1:9000".parse().unwrap())
        .max_block_range(500)
        .enable_rate_limiting(false)
        .build();

    assert_eq!(config.http.bind_address.port(), 9000);
    assert_eq!(config.features.max_block_range, 500);
    assert!(!config.rate_limit.enabled);
}

#[test]
fn test_to_file_method() -> Result<(), Box<dyn std::error::Error>> {
    let file = NamedTempFile::new()?;
    let path = file.path();

    let config = JsonRpcConfig::builder().max_block_range(500).build();

    config.to_file(path)?;

    let content = std::fs::read_to_string(path)?;

    assert!(
        content.contains("[jsonrpc.features]"),
        "File content should contain [jsonrpc.features] subsection"
    );
    assert!(
        content.contains("[jsonrpc.features]"),
        "TOML should contain [jsonrpc.features] subsection"
    );
    assert!(
        content.contains("max_block_range = 500"),
        "TOML should contain max_block_range setting"
    );
    let loaded = JsonRpcConfig::load(Some(path))?;
    assert_eq!(loaded.features.max_block_range, 500);

    Ok(())
}

#[test]
fn test_nested_config_objects() -> Result<(), Box<dyn std::error::Error>> {
    // --- UNSET relevant environment variables before loading ---
    std::env::remove_var("RUSK_JSONRPC_CONFIG_PATH"); // Ensure it loads the provided file
    std::env::remove_var("RUSK_JSONRPC_CORS_ENABLED");
    std::env::remove_var("RUSK_JSONRPC_CORS_ALLOWED_ORIGINS");
    std::env::remove_var("RUSK_JSONRPC_CORS_MAX_AGE_SECONDS");
    std::env::remove_var("RUSK_JSONRPC_RATE_LIMIT_DEFAULT_REQUESTS");
    std::env::remove_var("RUSK_JSONRPC_RATE_LIMIT_DEFAULT_WINDOW_SECS");

    let file = NamedTempFile::new()?;
    std::fs::write(
        file.path(),
        r#"
        [jsonrpc]
          [jsonrpc.http.cors]
          enabled = false
          allowed_origins = ["https://example.com", "https://test.com"]
          max_age_seconds = 3600

          [jsonrpc.rate_limit.default_limit]
          requests = 50
          window = 120
    "#,
    )?;

    let config = JsonRpcConfig::load(Some(file.path()))?;
    assert!(!config.http.cors.enabled);
    assert_eq!(config.http.cors.allowed_origins.len(), 2);
    assert_eq!(config.http.cors.allowed_origins[0], "https://example.com");
    assert_eq!(config.http.cors.max_age_seconds, 3600);

    assert_eq!(config.rate_limit.default_limit.requests, 50);
    assert_eq!(config.rate_limit.default_limit.window.as_secs(), 120);

    Ok(())
}

#[test]
fn test_method_limits_configuration() -> Result<(), Box<dyn std::error::Error>>
{
    let file = NamedTempFile::new()?;
    std::fs::write(
        file.path(),
        r#"
        [jsonrpc]
          [[jsonrpc.rate_limit.method_limits]]
          method_pattern = "getBlock*"
          limit = { requests = 200, window = 60 }

          [[jsonrpc.rate_limit.method_limits]]
          method_pattern = "sendTransaction"
          limit = { requests = 10, window = 60 }
    "#,
    )?;

    let config = JsonRpcConfig::load(Some(file.path()))?;
    assert_eq!(config.rate_limit.method_limits.len(), 2);

    let getblock_limit = &config.rate_limit.method_limits[0];
    assert_eq!(getblock_limit.method_pattern, "getBlock*");
    assert_eq!(getblock_limit.limit.requests, 200);
    assert_eq!(getblock_limit.limit.window.as_secs(), 60);

    let send_tx_limit = &config.rate_limit.method_limits[1];
    assert_eq!(send_tx_limit.method_pattern, "sendTransaction");
    assert_eq!(send_tx_limit.limit.requests, 10);
    assert_eq!(send_tx_limit.limit.window.as_secs(), 60);

    Ok(())
}

#[test]
fn test_error_message_quality() -> Result<(), Box<dyn std::error::Error>> {
    use tempfile::NamedTempFile;

    let file = NamedTempFile::new()?;
    std::fs::write(file.path(), "invalid [ toml syntax")?;

    let result = JsonRpcConfig::load(Some(file.path()));
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(err_msg.contains("syntax"));
    assert!(err_msg.contains("parse"));

    Ok(())
}

#[test]
fn test_validation_constraints() -> Result<(), Box<dyn std::error::Error>> {
    // Test 1: Default config should validate successfully
    let default_config = JsonRpcConfig::default();
    assert!(default_config.validate().is_ok());

    // Test 2: Setting invalid max_block_range (zero)
    let mut invalid_block_range = JsonRpcConfig::default();
    invalid_block_range.features.max_block_range = 0;
    let result = invalid_block_range.validate();
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err.to_string().contains("max_block_range"));
    }

    // Test 3: Setting invalid max_body_size (zero)
    let mut invalid_body_size = JsonRpcConfig::default();
    invalid_body_size.http.max_body_size = 0;
    let result = invalid_body_size.validate();
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err.to_string().contains("max_body_size"));
    }

    // Test 4: Setting invalid rate limit (zero requests)
    let mut invalid_rate_limit = JsonRpcConfig::default();
    invalid_rate_limit.rate_limit.default_limit.requests = 0;
    let result = invalid_rate_limit.validate();
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err.to_string().contains("requests"));
    }

    // Test 5: Setting invalid method limit pattern (empty string)
    let mut invalid_method_pattern = JsonRpcConfig::default();
    invalid_method_pattern
        .rate_limit
        .method_limits
        .push(MethodRateLimit {
            method_pattern: "".to_string(),
            limit: RateLimit::default(),
        });
    let result = invalid_method_pattern.validate();
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err.to_string().contains("method_pattern"));
    }

    // Test 6: Disabled rate limiting should skip rate limit validations
    let mut disabled_rate_limit = JsonRpcConfig::default();
    disabled_rate_limit.rate_limit.enabled = false;
    disabled_rate_limit.rate_limit.default_limit.requests = 0; // Would be invalid if enabled
    assert!(disabled_rate_limit.validate().is_ok());

    Ok(())
}

#[test]
fn test_boundary_values() -> Result<(), Box<dyn std::error::Error>> {
    let min_config = JsonRpcConfig::builder()
        .http_bind_address("127.0.0.1:1".parse().unwrap())
        .build();

    let mut min_config = min_config;

    min_config.http.max_body_size = 1;
    min_config.http.max_connections = 1;
    min_config.http.request_timeout = Duration::from_secs(1);

    min_config.http.cors.max_age_seconds = 1;

    min_config.ws.max_message_size = 1;
    min_config.ws.max_connections = 1;
    min_config.ws.max_subscriptions_per_connection = 1;
    min_config.ws.idle_timeout = Duration::from_secs(1);
    min_config.ws.max_events_per_second = 1;

    min_config.rate_limit.default_limit.requests = 1;
    min_config.rate_limit.default_limit.window = Duration::from_secs(1);
    min_config.rate_limit.websocket_limit.requests = 1;
    min_config.rate_limit.websocket_limit.window = Duration::from_secs(1);

    min_config.features.max_block_range = 1;
    min_config.features.max_batch_size = 1;

    assert!(
        min_config.validate().is_ok(),
        "Minimum config should be valid"
    );

    let toml_str = min_config.to_toml_string()?;
    let parsed_min_wrapper: RuskConfigFile = toml::from_str(&toml_str)?;
    let parsed_min = parsed_min_wrapper.jsonrpc;
    assert_eq!(
        parsed_min.http.max_body_size, 1,
        "Parsed min max_body_size mismatch"
    );
    assert_eq!(
        parsed_min.features.max_block_range, 1,
        "Parsed min max_block_range mismatch"
    );

    let mut max_config = JsonRpcConfig::default();

    const MAX_SAFE_REQUEST_SIZE: usize = 100 * 1024 * 1024;
    const MAX_SAFE_WS_MESSAGE_SIZE: usize = 10 * 1024 * 1024;
    const MAX_SAFE_DEFAULT_RATE: u64 = 1000;
    const MAX_SAFE_BLOCK_RANGE: u64 = 10000;

    max_config.http.max_body_size = MAX_SAFE_REQUEST_SIZE;
    max_config.http.max_connections = usize::MAX / 2;

    max_config.ws.max_message_size = MAX_SAFE_WS_MESSAGE_SIZE;
    max_config.ws.max_connections = usize::MAX / 2;
    max_config.ws.max_subscriptions_per_connection = usize::MAX / 2;
    max_config.ws.max_events_per_second = usize::MAX / 2;

    max_config.rate_limit.default_limit.requests = MAX_SAFE_DEFAULT_RATE;
    max_config.rate_limit.websocket_limit.requests = MAX_SAFE_DEFAULT_RATE;

    max_config.features.max_block_range = MAX_SAFE_BLOCK_RANGE;
    max_config.features.max_batch_size = usize::MAX / 2;

    let validation_result = max_config.validate();
    assert!(
        validation_result.is_ok(),
        "Maximum config should be valid: {:?}",
        validation_result.err()
    );

    let toml_str = max_config.to_toml_string()?;
    let parsed_max_wrapper: RuskConfigFile = toml::from_str(&toml_str)?;
    let parsed_max = parsed_max_wrapper.jsonrpc;

    assert_eq!(
        parsed_max.http.max_body_size, MAX_SAFE_REQUEST_SIZE,
        "Parsed max max_body_size mismatch"
    );
    assert_eq!(
        parsed_max.features.max_block_range, MAX_SAFE_BLOCK_RANGE,
        "Parsed max max_block_range mismatch"
    );

    let mut empty_collections = JsonRpcConfig::default();
    empty_collections.http.cors.allowed_origins = Vec::new();
    empty_collections.http.cors.allowed_methods = Vec::new();
    empty_collections.http.cors.allowed_headers = Vec::new();
    empty_collections.rate_limit.method_limits = Vec::new();

    assert!(
        empty_collections.validate().is_ok(),
        "Empty collections should be valid"
    );

    let mut large_collections = JsonRpcConfig::default();

    large_collections.http.cors.allowed_origins = (0..1000)
        .map(|i| format!("https://example{}.com", i))
        .collect();
    large_collections.http.cors.allowed_methods = vec![
        "GET".to_string(),
        "POST".to_string(),
        "PUT".to_string(),
        "DELETE".to_string(),
        "PATCH".to_string(),
    ];
    large_collections.http.cors.allowed_headers =
        (0..1000).map(|i| format!("Header-{}", i)).collect();

    large_collections.rate_limit.method_limits = (0..1000)
        .map(|i| MethodRateLimit {
            method_pattern: format!("method{}", i),
            limit: RateLimit {
                requests: 10,
                window: Duration::from_secs(60),
            },
        })
        .collect();

    assert!(
        large_collections.validate().is_ok(),
        "Large collections should be valid"
    );

    let toml_str = large_collections.to_toml_string()?;
    let parsed_large_wrapper: RuskConfigFile = toml::from_str(&toml_str)?;
    let parsed_large = parsed_large_wrapper.jsonrpc;

    assert_eq!(
        parsed_large.http.cors.allowed_origins.len(),
        1000,
        "Parsed large allowed_origins length mismatch"
    );
    assert_eq!(
        parsed_large.rate_limit.method_limits.len(),
        1000,
        "Parsed large method_limits length mismatch"
    );

    Ok(())
}

#[test]
fn test_security_configuration_audit() -> Result<(), Box<dyn std::error::Error>>
{
    // Test 1: Default configuration should pass security audit
    let default_config = JsonRpcConfig::default();
    assert!(
        default_config.validate().is_ok(),
        "Default configuration should be secure"
    );

    // Test 2: Binding to public interface without rate limiting
    let mut public_binding_config = JsonRpcConfig::default();
    public_binding_config.http.bind_address = "0.0.0.0:8546".parse().unwrap();
    public_binding_config.rate_limit.enabled = false;

    let result = public_binding_config.validate();
    assert!(
        result.is_err(),
        "Public binding without rate limiting should fail"
    );
    assert_security_error(&result, "public interface without rate limiting");

    // Test 3: Wildcard CORS origin with credentials
    let mut insecure_cors_config = JsonRpcConfig::default();
    insecure_cors_config.http.cors.enabled = true;
    insecure_cors_config.http.cors.allowed_origins = vec!["*".to_string()];
    insecure_cors_config.http.cors.allow_credentials = true;

    let result = insecure_cors_config.validate();
    assert!(
        result.is_err(),
        "Wildcard CORS with credentials should fail"
    );
    assert_security_error(&result, "wildcard CORS origin");

    // Test 4: Excessive request body size
    let mut large_body_config = JsonRpcConfig::default();
    large_body_config.http.max_body_size = 1024 * 1024 * 1024; // 1 GB

    let result = large_body_config.validate();
    assert!(
        result.is_err(),
        "Excessively large request body size should fail"
    );
    assert_security_error(&result, "body size");

    // Test 5: Disabled strict parameter validation
    let mut insecure_validation_config = JsonRpcConfig::default();
    insecure_validation_config
        .features
        .strict_parameter_validation = false;

    let result = insecure_validation_config.validate();
    assert!(result.is_err(), "Disabled parameter validation should fail");
    assert_security_error(&result, "parameter validation");

    // Test 6: Excessive rate limits
    let mut excessive_rate_config = JsonRpcConfig::default();
    excessive_rate_config.rate_limit.default_limit.requests = 100000; // 100K per minute
    excessive_rate_config.rate_limit.default_limit.window =
        Duration::from_secs(60);

    let result = excessive_rate_config.validate();
    assert!(result.is_err(), "Excessive rate limits should fail");
    assert_security_error(&result, "rate limit");

    // Test 7: Method with practically unlimited rate
    let mut unlimited_method_config = JsonRpcConfig::default();
    unlimited_method_config
        .rate_limit
        .method_limits
        .push(MethodRateLimit {
            method_pattern: "sensitiveMethod".to_string(),
            limit: RateLimit {
                requests: 1000000,
                window: Duration::from_secs(60),
            },
        });

    let result = unlimited_method_config.validate();
    assert!(
        result.is_err(),
        "Method with extremely high limit should fail"
    );
    assert_security_error(&result, "high rate limit");

    // Test 8: Excessive block range
    let mut excessive_block_range_config = JsonRpcConfig::default();
    excessive_block_range_config.features.max_block_range = 1000000;

    let result = excessive_block_range_config.validate();
    assert!(result.is_err(), "Excessive block range should fail");
    assert_security_error(&result, "block range");

    // Test 9: Check that a reasonably secure production configuration passes
    let secure_production_config = JsonRpcConfig::builder()
        .http_bind_address("0.0.0.0:8546".parse().unwrap()) // Public interface, but with rate limiting
        .enable_rate_limiting(true)
        .default_rate_limit(500, 60) // 500 requests per minute
        .max_block_range(5000) // Reasonable block range
        .build();

    assert!(
        secure_production_config.validate().is_ok(),
        "Secure production configuration should pass validation"
    );

    // Test 10: Check sanitization settings
    let mut sanitization_config = JsonRpcConfig::default();
    sanitization_config.http.bind_address = "127.0.0.1:8545".parse().unwrap();
    sanitization_config.rate_limit.default_limit.requests = 100;
    sanitization_config.rate_limit.default_limit.window =
        Duration::from_secs(60);
    sanitization_config.http.cors.allowed_origins =
        vec!["http://localhost".to_string()];
    assert!(
        sanitization_config.validate().is_ok(),
        "Sanitization configuration should pass validation"
    );

    // Test 11: Check combination of potential issues
    let mut combined_issues_config = JsonRpcConfig::default();
    // 1. Public binding (not an issue by itself with rate limiting)
    combined_issues_config.http.bind_address = "0.0.0.0:8546".parse().unwrap();
    // 2. High but acceptable rate limit
    combined_issues_config.rate_limit.default_limit.requests = 900;
    combined_issues_config.rate_limit.default_limit.window =
        Duration::from_secs(60);
    // 3. Reasonable CORS (wildcard without credentials is acceptable)
    combined_issues_config.http.cors.allowed_origins = vec!["*".to_string()];
    combined_issues_config.http.cors.allow_credentials = false;

    assert!(
        combined_issues_config.validate().is_ok(),
        "Configuration with acceptable security trade-offs should pass"
    );

    Ok(())
}

#[test]
fn test_production_environment_security(
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a typical production configuration
    let mut prod_config = JsonRpcConfig::default();

    // 1. Public interfaces with appropriate rate limiting
    prod_config.http.bind_address = "0.0.0.0:8546".parse().unwrap();
    prod_config.ws.bind_address = "0.0.0.0:8547".parse().unwrap();

    // 2. Strict CORS configuration
    prod_config.http.cors.enabled = true;
    prod_config.http.cors.allowed_origins =
        vec!["https://app.example.com".to_string()];
    prod_config.http.cors.allowed_methods = vec!["POST".to_string()];
    prod_config.http.cors.allowed_headers =
        vec!["Content-Type".to_string(), "Rusk-Version".to_string()];
    prod_config.http.cors.allow_credentials = true;

    // 3. Appropriate rate limiting
    prod_config.rate_limit.enabled = true;
    prod_config.rate_limit.default_limit.requests = 100;
    prod_config.rate_limit.default_limit.window = Duration::from_secs(60);

    // 4. Method-specific rate limits
    prod_config.rate_limit.method_limits = vec![
        MethodRateLimit {
            method_pattern: "get*".to_string(),
            limit: RateLimit {
                requests: 200,
                window: Duration::from_secs(60),
            },
        },
        MethodRateLimit {
            method_pattern: "prove".to_string(),
            limit: RateLimit {
                requests: 10,
                window: Duration::from_secs(60),
            },
        },
    ];

    // 5. Security features enabled
    prod_config.features.strict_parameter_validation = true;
    prod_config.features.strict_version_checking = true;

    // This should pass security validation
    assert!(
        prod_config.validate().is_ok(),
        "Production configuration should be secure"
    );

    // Verify specific modifications that would make it insecure

    // Test 1: Disable rate limiting on public interface
    let mut insecure_prod1 = prod_config.clone();
    insecure_prod1.rate_limit.enabled = false;
    let result1 = insecure_prod1.validate();
    assert!(
        result1.is_err(),
        "Public interface without rate limiting should fail"
    );
    // Add type annotation for T (e.g., ()) as the Ok variant type is irrelevant
    // here
    assert_security_error::<()>(
        &result1,
        "public interface without rate limiting",
    );

    // Test 2: Set permissive CORS with wildcard origin
    let mut cors_config = prod_config.clone();
    cors_config.http.cors.allowed_origins = vec!["*".to_string()];
    cors_config.http.cors.allow_credentials = false;

    let validation_result = cors_config.validate();
    assert!(validation_result.is_ok(),
        "CORS with wildcard origin but no credentials should be acceptable: {:?}",
        validation_result.err());

    // Now enable credentials which makes it insecure
    cors_config.http.cors.allow_credentials = true;

    let validation_result_cors = cors_config.validate();
    assert!(
        validation_result_cors.is_err(),
        "CORS with wildcard origin and credentials should fail"
    );
    assert_security_error::<()>(
        &validation_result_cors,
        "wildcard CORS origin",
    );

    // Test 3: Set overly generous rate limits
    let mut insecure_prod3 = prod_config.clone();
    insecure_prod3.rate_limit.default_limit.requests = 10000;
    let result3 = insecure_prod3.validate();
    assert!(result3.is_err(), "Excessive rate limits should fail");
    assert_security_error::<()>(&result3, "rate limit");

    Ok(())
}

#[test]
fn test_environment_specific_configs() -> Result<(), Box<dyn std::error::Error>>
{
    let dev_config = create_environment_config(&[]);
    assert!(dev_config.validate().is_ok(),
        "Development config should be valid (security checks adjusted for localhost)");

    let test_config = create_environment_config(&[]);
    assert!(
        test_config.validate().is_ok(),
        "Testing config should be valid"
    );

    let prod_config = create_environment_config(&[]);
    assert!(
        prod_config.validate().is_ok(),
        "Production config should be valid and secure"
    );
    assert!(prod_config.rate_limit.enabled);
    assert!(prod_config.features.strict_parameter_validation);

    Ok(())
}

#[test]
fn test_tls_file_validation() {
    let mut config1 = JsonRpcConfig::default();
    config1.http.cert = Some("/path/to/nonexistent/cert.pem".into());
    config1.http.key = None;
    let result1 = config1.validate();
    assert!(result1.is_err());
    assert!(
        matches!(result1.unwrap_err(), ConfigError::Validation(s) if s.contains("key is missing"))
    );

    let mut config2 = JsonRpcConfig::default();
    config2.http.cert = None;
    config2.http.key = Some("/path/to/nonexistent/key.pem".into());
    let result2 = config2.validate();
    assert!(result2.is_err());
    assert!(
        matches!(result2.unwrap_err(), ConfigError::Validation(s) if s.contains("certificate is missing"))
    );

    let mut config3 = JsonRpcConfig::default();
    let temp_key = NamedTempFile::new().unwrap();
    config3.http.cert = Some("/path/to/nonexistent/cert.pem".into());
    config3.http.key = Some(temp_key.path().to_path_buf());
    let result3 = config3.validate();
    assert!(result3.is_err());
    assert!(
        matches!(result3.unwrap_err(), ConfigError::Validation(s) if s.contains("certificate file not found"))
    );

    let mut config4 = JsonRpcConfig::default();
    let temp_cert = NamedTempFile::new().unwrap();
    config4.http.cert = Some(temp_cert.path().to_path_buf());
    config4.http.key = Some("/path/to/nonexistent/key.pem".into());
    let result4 = config4.validate();
    assert!(result4.is_err());
    assert!(
        matches!(result4.unwrap_err(), ConfigError::Validation(s) if s.contains("key file not found"))
    );
    let mut config5 = JsonRpcConfig::default();
    let temp_cert_ok = NamedTempFile::new().unwrap();
    let temp_key_ok = NamedTempFile::new().unwrap();
    config5.http.cert = Some(temp_cert_ok.path().to_path_buf());
    config5.http.key = Some(temp_key_ok.path().to_path_buf());
    assert!(config5.validate().is_ok());
}

mod property_tests {
    use super::*;
    use proptest::prelude::*;

    fn arb_config() -> impl Strategy<Value = JsonRpcConfig> {
        let arb_port = 1000u16..10000u16;
        let arb_max_block_range = 1u64..10000u64;
        (arb_port, arb_max_block_range).prop_map(|(port, max_block_range)| {
            JsonRpcConfig::builder()
                .http_bind_address(
                    format!("127.0.0.1:{}", port).parse().unwrap(),
                )
                .max_block_range(max_block_range)
                .build()
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            failure_persistence: None,
            ..ProptestConfig::default()
        })]

        #[test]
        fn test_roundtrip_serialization(config in arb_config()) {
            let toml_str = config.to_toml_string().unwrap();
            let parsed_wrapper: RuskConfigFile = toml::from_str(&toml_str).unwrap();
            let parsed = parsed_wrapper.jsonrpc;
            prop_assert_eq!(config.http.bind_address, parsed.http.bind_address, "HTTP bind address mismatch after roundtrip");
            prop_assert_eq!(config.features.max_block_range, parsed.features.max_block_range, "Max block range mismatch after roundtrip");
        }
    }
}
#[derive(Deserialize, Default)]
struct RuskConfigFile {
    #[serde(default)]
    jsonrpc: JsonRpcConfig,
    #[serde(flatten)]
    _other: HashMap<String, toml::Value>,
}
