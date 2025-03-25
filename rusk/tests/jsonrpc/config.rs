// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::env;
use std::sync::Mutex;
use std::time::Duration;

use lazy_static::lazy_static;
use tempfile::NamedTempFile;

use rusk::jsonrpc::config::{
    ConfigError, CorsConfig, JsonRpcConfig, MethodRateLimit, RateLimit,
};

use crate::jsonrpc::utils::{assert_security_error, create_environment_config};

lazy_static! {
    // This ensures only one test can modify environment variables at a time
    // Mutex automatically unlocked when dropped
    static ref ENV_MUTEX: Mutex<()> = Mutex::new(());
}

#[test]
fn test_default_config() {
    let config = JsonRpcConfig::default();
    assert_eq!(config.http.bind_address.port(), 8546);
    assert_eq!(config.ws.bind_address.port(), 8547);
    assert!(config.rate_limit.enabled);
    assert!(config.features.enable_websocket);
    assert_eq!(config.features.max_block_range, 1000);
}

#[test]
fn test_config_file_roundtrip() -> Result<(), ConfigError> {
    let config = JsonRpcConfig::default();
    let file = NamedTempFile::new().expect("Failed to create temp file");
    let path = file.path();

    // Write config to file
    config.to_file(path)?;

    // Read config from file
    let loaded_config = JsonRpcConfig::from_file(path)?;

    // Verify config was preserved
    assert_eq!(loaded_config.http.bind_address, config.http.bind_address);
    assert_eq!(
        loaded_config.features.max_block_range,
        config.features.max_block_range
    );

    Ok(())
}

#[test]
fn test_env_config() {
    let _lock = ENV_MUTEX.lock().unwrap();

    // Set environment variables
    env::set_var("RUSK_JSONRPC_HTTP_BIND_ADDRESS", "127.0.0.1:9000");
    env::set_var("RUSK_JSONRPC_FEATURE_MAX_BLOCK_RANGE", "500");
    env::set_var("RUSK_JSONRPC_RATE_LIMIT_ENABLED", "false");

    // Load config from env
    let config = JsonRpcConfig::from_env().unwrap();

    // Verify env values were used
    assert_eq!(config.http.bind_address.port(), 9000);
    assert_eq!(config.features.max_block_range, 500);
    assert!(!config.rate_limit.enabled);

    // Clean up
    env::remove_var("RUSK_JSONRPC_HTTP_BIND_ADDRESS");
    env::remove_var("RUSK_JSONRPC_FEATURE_MAX_BLOCK_RANGE");
    env::remove_var("RUSK_JSONRPC_RATE_LIMIT_ENABLED");
}

#[test]
fn test_complex_env_vars() {
    let _lock = ENV_MUTEX.lock().unwrap();

    // Test array/list parsing from environment variables
    env::set_var(
        "RUSK_JSONRPC_CORS_ALLOWED_ORIGINS",
        "https://example.com,https://test.com",
    );

    // Test empty arrays
    env::set_var("RUSK_JSONRPC_CORS_ALLOWED_METHODS", "");

    // Test boolean parsing variations
    env::set_var("RUSK_JSONRPC_FEATURE_DETAILED_ERRORS", "TRUE"); // Uppercase
    env::set_var("RUSK_JSONRPC_RATE_LIMIT_ENABLED", "yes"); // Non-standard boolean

    let config = JsonRpcConfig::from_env().unwrap();

    // Verify parsing behavior
    assert_eq!(config.http.cors.allowed_origins.len(), 2);
    assert_eq!(config.http.cors.allowed_origins[0], "https://example.com");
    assert!(config.http.cors.allowed_methods.is_empty());
    assert!(config.features.detailed_errors); // Should handle uppercase TRUE

    // Clean up
    env::remove_var("RUSK_JSONRPC_CORS_ALLOWED_ORIGINS");
    env::remove_var("RUSK_JSONRPC_CORS_ALLOWED_METHODS");
    env::remove_var("RUSK_JSONRPC_FEATURE_DETAILED_ERRORS");
    env::remove_var("RUSK_JSONRPC_RATE_LIMIT_ENABLED");
}

#[test]
fn test_malformed_env_vars() {
    let _lock = ENV_MUTEX.lock().unwrap();

    // Set invalid environment variables
    env::set_var("RUSK_JSONRPC_HTTP_BIND_ADDRESS", "not-an-address");
    env::set_var("RUSK_JSONRPC_HTTP_REQUEST_TIMEOUT_SECS", "not-a-number");

    // Load config and verify it falls back to defaults
    let config = JsonRpcConfig::from_env().unwrap();

    // Should use default values when parsing fails
    assert_eq!(config.http.bind_address.port(), 8546); // Default port
    assert_eq!(config.http.request_timeout.as_secs(), 30); // Default timeout

    // Clean up
    env::remove_var("RUSK_JSONRPC_HTTP_BIND_ADDRESS");
    env::remove_var("RUSK_JSONRPC_HTTP_REQUEST_TIMEOUT_SECS");
}

#[test]
fn test_load_with_precedence() -> Result<(), ConfigError> {
    let _lock = ENV_MUTEX.lock().unwrap();

    // Create a config file
    let mut config = JsonRpcConfig::default();
    config.http.bind_address = "127.0.0.1:8000".parse().unwrap();
    config.features.max_block_range = 300;

    let file = NamedTempFile::new().expect("Failed to create temp file");
    let path = file.path();
    config.to_file(path)?;

    // Set environment variable that should override file
    env::set_var("RUSK_JSONRPC_HTTP_BIND_ADDRESS", "127.0.0.1:9000");

    // Load with precedence
    let loaded_config = JsonRpcConfig::load(Some(path))?;

    // Verify precedence: env var overrides file, file overrides default
    assert_eq!(loaded_config.http.bind_address.port(), 9000); // From env
    assert_eq!(loaded_config.features.max_block_range, 300); // From file

    // Clean up
    env::remove_var("RUSK_JSONRPC_HTTP_BIND_ADDRESS");

    Ok(())
}

#[test]
fn test_complete_precedence_chain() -> Result<(), Box<dyn std::error::Error>> {
    let _lock = ENV_MUTEX.lock().unwrap();

    // Default value for max_block_range is 1000

    // Create config file with non-default value
    let file = NamedTempFile::new()?;
    std::fs::write(
        file.path(),
        r#"
        [features]
        max_block_range = 500
    "#,
    )?;

    // Set environment variable with third value
    env::set_var("RUSK_JSONRPC_FEATURE_MAX_BLOCK_RANGE", "200");

    // Load with all three sources
    let config = JsonRpcConfig::load(Some(file.path()))?;

    // Env var should have highest precedence
    assert_eq!(config.features.max_block_range, 200);

    // Remove env var and test again
    env::remove_var("RUSK_JSONRPC_FEATURE_MAX_BLOCK_RANGE");
    let config = JsonRpcConfig::load(Some(file.path()))?;

    // File value should take precedence over default
    assert_eq!(config.features.max_block_range, 500);

    // Now load with no file
    let config = JsonRpcConfig::load(None)?;

    // Should get default value
    assert_eq!(config.features.max_block_range, 1000);

    Ok(())
}

#[test]
fn test_toml_format() -> Result<(), Box<dyn std::error::Error>> {
    let config = JsonRpcConfig::default();
    let toml_str = config.to_toml_string()?;

    // Verify TOML contains expected sections
    assert!(toml_str.contains("[http]"));
    assert!(toml_str.contains("[ws]"));
    assert!(toml_str.contains("[rate_limit]"));
    assert!(toml_str.contains("[features]"));

    // Parse back to verify
    let _parsed: JsonRpcConfig = toml::from_str(&toml_str)?;

    Ok(())
}

#[test]
fn test_cors_config() {
    let default_cors = CorsConfig::default();
    assert!(default_cors.enabled);
    assert!(default_cors.allowed_origins.is_empty()); // All origins allowed
    assert_eq!(default_cors.max_age_seconds, 86400);

    // Create custom CORS config
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
}

#[test]
fn test_invalid_toml_file() -> Result<(), Box<dyn std::error::Error>> {
    // Create file with invalid TOML
    let file = NamedTempFile::new()?;
    std::fs::write(file.path(), "invalid [ toml syntax")?;

    // Attempt to load should return error
    let result = JsonRpcConfig::from_file(file.path());
    assert!(result.is_err());

    // Error should be the expected type
    assert!(matches!(result.unwrap_err(), ConfigError::TomlParse(_)));

    Ok(())
}

#[test]
fn test_partial_config_file() -> Result<(), Box<dyn std::error::Error>> {
    // Create file with only a subset of settings
    let file = NamedTempFile::new()?;
    std::fs::write(
        file.path(),
        r#"
        # Minimal configuration with only essential overrides
        [http]
        # Custom body size limit
        max_body_size = 5242880  # 5MB instead of default 10MB

        [features]
        # Custom block range limit
        max_block_range = 500
    "#,
    )?;

    // Load the partial config
    let config = JsonRpcConfig::from_file(file.path())?;

    // Specified values should be overridden
    assert_eq!(config.http.max_body_size, 5242880);
    assert_eq!(config.features.max_block_range, 500);

    // Unspecified values should use defaults
    assert_eq!(config.http.bind_address.port(), 8546);
    assert_eq!(config.http.request_timeout.as_secs(), 30);
    assert_eq!(config.ws.max_connections, 50);

    Ok(())
}

#[test]
fn test_nonexistent_config_file() -> Result<(), Box<dyn std::error::Error>> {
    // Specify a path that doesn't exist
    let path = std::path::Path::new("/path/that/doesnt/exist/config.toml");

    // Should not error when loading with a nonexistent file
    let config = JsonRpcConfig::load(Some(path))?;

    // Should get default values
    assert_eq!(config.http.bind_address.port(), 8546);
    assert_eq!(config.features.max_block_range, 1000);

    Ok(())
}

#[test]
fn test_config_path_resolution() {
    // Get the project root
    let root = JsonRpcConfig::project_root();

    // Verify Cargo.toml exists at the root
    assert!(
        root.join("Cargo.toml").exists(),
        "Project root should contain Cargo.toml: {:?}",
        root
    );

    // Verify default config path resolution
    let config_path = JsonRpcConfig::default_config_path();
    assert_eq!(
        config_path.file_name().unwrap().to_str().unwrap(),
        JsonRpcConfig::DEFAULT_CONFIG_FILENAME
    );

    // Print the resolved paths for debugging
    println!("Project root resolved to: {:?}", root);
    println!("Config path resolved to: {:?}", config_path);
}

#[test]
fn test_custom_socket_addr_serialization(
) -> Result<(), Box<dyn std::error::Error>> {
    // Create config with custom socket addresses
    let mut config = JsonRpcConfig::default();
    config.http.bind_address = "192.168.1.1:9000".parse()?;
    config.ws.bind_address = "[::1]:9001".parse()?;

    // Serialize to TOML
    let toml_str = config.to_toml_string()?;

    // Should contain string representation
    assert!(toml_str.contains("bind_address = \"192.168.1.1:9000\""));
    assert!(toml_str.contains("bind_address = \"[::1]:9001\""));

    // Deserialize back
    let parsed: JsonRpcConfig = toml::from_str(&toml_str)?;

    // Should have the same values
    assert_eq!(parsed.http.bind_address.to_string(), "192.168.1.1:9000");
    assert_eq!(parsed.ws.bind_address.to_string(), "[::1]:9001");

    Ok(())
}

#[test]
fn test_custom_duration_serialization() -> Result<(), Box<dyn std::error::Error>>
{
    // Create config with custom durations
    let mut config = JsonRpcConfig::default();
    config.http.request_timeout = Duration::from_secs(120);
    config.ws.idle_timeout = Duration::from_secs(600);

    // Serialize to TOML
    let toml_str = config.to_toml_string()?;

    // Should contain numeric representation of seconds
    assert!(toml_str.contains("request_timeout = 120"));
    assert!(toml_str.contains("idle_timeout = 600"));

    // Deserialize back
    let parsed: JsonRpcConfig = toml::from_str(&toml_str)?;

    // Should have the same values
    assert_eq!(parsed.http.request_timeout.as_secs(), 120);
    assert_eq!(parsed.ws.idle_timeout.as_secs(), 600);

    Ok(())
}

#[test]
fn test_config_builder_api() {
    // Test that our builder API is ergonomic
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
    // Create temporary file
    let file = NamedTempFile::new()?;
    let path = file.path();

    // Create custom config
    let config = JsonRpcConfig::builder().max_block_range(500).build();

    // Save to file
    config.to_file(path)?;

    // Read file content
    let content = std::fs::read_to_string(path)?;
    assert!(content.contains("max_block_range = 500"));

    // Load from the same file
    let loaded = JsonRpcConfig::from_file(path)?;
    assert_eq!(loaded.features.max_block_range, 500);

    Ok(())
}

#[test]
fn test_nested_config_objects() -> Result<(), Box<dyn std::error::Error>> {
    // Create file with deeply nested config
    let file = NamedTempFile::new()?;
    std::fs::write(
        file.path(),
        r#"
        [http.cors]
        enabled = false
        allowed_origins = ["https://example.com", "https://test.com"]
        max_age_seconds = 3600

        [rate_limit.default_limit]
        requests = 50
        window = 120
    "#,
    )?;

    // Load the config
    let config = JsonRpcConfig::from_file(file.path())?;

    // Verify nested values
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
    // Create file with method limits config
    let file = NamedTempFile::new()?;
    std::fs::write(
        file.path(),
        r#"
        [[rate_limit.method_limits]]
        method_pattern = "getBlock*"
        limit = { requests = 200, window = 60 }

        [[rate_limit.method_limits]]
        method_pattern = "sendTransaction"
        limit = { requests = 10, window = 60 }
    "#,
    )?;

    // Load the config
    let config = JsonRpcConfig::from_file(file.path())?;

    // Verify method limits
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

    // Create file with invalid TOML
    let file = NamedTempFile::new()?;
    std::fs::write(file.path(), "invalid [ toml syntax")?;

    // Attempt to load
    let result = JsonRpcConfig::from_file(file.path());
    assert!(result.is_err());

    // Error should have useful message
    let err = result.unwrap_err();
    let err_msg = err.to_string();

    // Check that the error message is helpful
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
    // Test with minimum acceptable values
    let min_config = JsonRpcConfig::builder()
        // HTTP settings
        .http_bind_address("127.0.0.1:1".parse().unwrap())
        .build();

    // Modify the config to set all numeric values to their minimums
    let mut min_config = min_config;

    // HTTP minimum values
    min_config.http.max_body_size = 1;
    min_config.http.max_connections = 1;
    min_config.http.request_timeout = Duration::from_secs(1);

    // CORS minimum values
    min_config.http.cors.max_age_seconds = 1;

    // WebSocket minimum values
    min_config.ws.max_message_size = 1;
    min_config.ws.max_connections = 1;
    min_config.ws.max_subscriptions_per_connection = 1;
    min_config.ws.idle_timeout = Duration::from_secs(1);
    min_config.ws.max_events_per_second = 1;

    // Rate limit minimum values
    min_config.rate_limit.default_limit.requests = 1;
    min_config.rate_limit.default_limit.window = Duration::from_secs(1);
    min_config.rate_limit.websocket_limit.requests = 1;
    min_config.rate_limit.websocket_limit.window = Duration::from_secs(1);

    // Feature minimum values
    min_config.features.max_block_range = 1;
    min_config.features.max_batch_size = 1;

    // Validate minimum config
    assert!(
        min_config.validate().is_ok(),
        "Minimum config should be valid"
    );

    // Serialize and deserialize minimum config to ensure it works
    let toml_str = min_config.to_toml_string()?;
    let parsed_min: JsonRpcConfig = toml::from_str(&toml_str)?;
    assert_eq!(parsed_min.http.max_body_size, 1);
    assert_eq!(parsed_min.features.max_block_range, 1);

    // Test with large but security-compliant values
    let mut max_config = JsonRpcConfig::default();

    // Define security-compliant maximum values
    const MAX_SAFE_REQUEST_SIZE: usize = 100 * 1024 * 1024; // 100 MB
    const MAX_SAFE_WS_MESSAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MB
    const MAX_SAFE_DEFAULT_RATE: u64 = 1000; // per minute
    const MAX_SAFE_BLOCK_RANGE: u64 = 10000;

    // HTTP maximum values (within security limits)
    max_config.http.max_body_size = MAX_SAFE_REQUEST_SIZE;
    max_config.http.max_connections = usize::MAX / 2;
    // Keep reasonable timeout to avoid test duration issues

    // WebSocket maximum values
    max_config.ws.max_message_size = MAX_SAFE_WS_MESSAGE_SIZE;
    max_config.ws.max_connections = usize::MAX / 2;
    max_config.ws.max_subscriptions_per_connection = usize::MAX / 2;
    // Keep reasonable timeout to avoid test duration issues
    max_config.ws.max_events_per_second = usize::MAX / 2;

    // Rate limit maximum values
    max_config.rate_limit.default_limit.requests = MAX_SAFE_DEFAULT_RATE;
    // Keep reasonable window to avoid test duration issues
    max_config.rate_limit.websocket_limit.requests = MAX_SAFE_DEFAULT_RATE;
    // Keep reasonable window to avoid test duration issues

    // Feature maximum values
    max_config.features.max_block_range = MAX_SAFE_BLOCK_RANGE;
    max_config.features.max_batch_size = usize::MAX / 2;

    // Validate maximum config
    let validation_result = max_config.validate();
    assert!(
        validation_result.is_ok(),
        "Maximum config should be valid: {:?}",
        validation_result.err()
    );

    // Serialize and deserialize maximum config to ensure it works
    let toml_str = max_config.to_toml_string()?;
    let parsed_max: JsonRpcConfig = toml::from_str(&toml_str)?;

    // Check a few key fields to verify deserialization worked
    assert_eq!(parsed_max.http.max_body_size, MAX_SAFE_REQUEST_SIZE);
    assert_eq!(parsed_max.features.max_block_range, MAX_SAFE_BLOCK_RANGE);

    // Test with empty collections
    let mut empty_collections = JsonRpcConfig::default();
    empty_collections.http.cors.allowed_origins = Vec::new();
    empty_collections.http.cors.allowed_methods = Vec::new();
    empty_collections.http.cors.allowed_headers = Vec::new();
    empty_collections.rate_limit.method_limits = Vec::new();

    // Validate empty collections config
    assert!(
        empty_collections.validate().is_ok(),
        "Empty collections should be valid"
    );

    // Test with very large collections
    let mut large_collections = JsonRpcConfig::default();

    // Create large arrays of origins, methods, and headers
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

    // Create a large number of method rate limits
    large_collections.rate_limit.method_limits = (0..1000)
        .map(|i| MethodRateLimit {
            method_pattern: format!("method{}", i),
            limit: RateLimit {
                requests: 10,
                window: Duration::from_secs(60),
            },
        })
        .collect();

    // Validate large collections config
    assert!(
        large_collections.validate().is_ok(),
        "Large collections should be valid"
    );

    // Serialize and deserialize to ensure it works with large collections
    let toml_str = large_collections.to_toml_string()?;
    let parsed_large: JsonRpcConfig = toml::from_str(&toml_str)?;

    assert_eq!(parsed_large.http.cors.allowed_origins.len(), 1000);
    assert_eq!(parsed_large.rate_limit.method_limits.len(), 1000);

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

    // Test 10: Check combination of potential issues
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
    assert!(
        insecure_prod1.validate().is_err(),
        "Public interface without rate limiting should fail"
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

    let validation_result = cors_config.validate();
    assert!(
        validation_result.is_err(),
        "CORS with wildcard origin and credentials should fail"
    );

    // Test 3: Set overly generous rate limits
    let mut insecure_prod3 = prod_config.clone();
    insecure_prod3.rate_limit.default_limit.requests = 10000;
    assert!(
        insecure_prod3.validate().is_err(),
        "Excessive rate limits should fail"
    );

    Ok(())
}

#[test]
fn test_environment_specific_configs() -> Result<(), Box<dyn std::error::Error>>
{
    // Test development config
    let dev_config = create_environment_config("development");
    assert!(dev_config.validate().is_ok(),
        "Development config should be valid (security checks adjusted for localhost)");
    assert_eq!(dev_config.http.bind_address.ip().to_string(), "127.0.0.1");

    // Test testing config
    let test_config = create_environment_config("testing");
    assert!(
        test_config.validate().is_ok(),
        "Testing config should be valid"
    );
    assert_eq!(test_config.http.bind_address.ip().to_string(), "0.0.0.0");
    assert!(test_config.rate_limit.enabled);

    // Test production config
    let prod_config = create_environment_config("production");
    assert!(
        prod_config.validate().is_ok(),
        "Production config should be valid and secure"
    );
    assert!(prod_config.rate_limit.enabled);
    assert!(prod_config.features.strict_parameter_validation);
    assert!(prod_config.features.strict_version_checking);

    Ok(())
}

mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Generator for valid configurations
    fn arb_config() -> impl Strategy<Value = JsonRpcConfig> {
        // Define strategies for each field
        let arb_port = 1000u16..10000u16;
        let arb_max_block_range = 1u64..10000u64;

        // Combine field strategies into a configuration
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
            // Disable file persistence to avoid the error
            failure_persistence: None,
            ..ProptestConfig::default()
        })]

        // Test that any valid config can be serialized and deserialized
        #[test]
        fn test_roundtrip_serialization(config in arb_config()) {
            let toml_str = config.to_toml_string().unwrap();
            let parsed: JsonRpcConfig = toml::from_str(&toml_str).unwrap();

            // Compare key fields
            assert_eq!(config.http.bind_address, parsed.http.bind_address);
            assert_eq!(config.features.max_block_range, parsed.features.max_block_range);
        }
    }
}
