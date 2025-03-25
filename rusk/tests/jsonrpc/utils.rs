// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk::jsonrpc::config::{ConfigError, JsonRpcConfig};

// Helper function to check if an error is a security error and contains the
// expected message
pub fn assert_security_error(
    result: &Result<(), ConfigError>,
    expected_message_part: &str,
) {
    if let Err(err) = result {
        match err {
            ConfigError::SecurityViolation(msg) => {
                assert!(
                    msg.to_lowercase()
                        .contains(&expected_message_part.to_lowercase()),
                    "Security error message '{}' should contain '{}'",
                    msg,
                    expected_message_part
                );
            }
            _ => panic!("Expected SecurityViolation, got: {:?}", err),
        }
    } else {
        panic!("Expected error, but validation succeeded");
    }
}

/// Create a secure configuration for a specific environment type
pub fn create_environment_config(env_type: &str) -> JsonRpcConfig {
    match env_type {
        "development" => {
            // Development environment - localhost only, minimal security
            JsonRpcConfig::builder()
                .http_bind_address("127.0.0.1:8546".parse().unwrap())
                .ws_bind_address("127.0.0.1:8547".parse().unwrap())
                .enable_rate_limiting(false) // OK for local dev
                .enable_websocket(true)
                .build()
        }
        "testing" => {
            // Testing environment - more permissive for testing
            JsonRpcConfig::builder()
                .http_bind_address("0.0.0.0:8546".parse().unwrap())
                .ws_bind_address("0.0.0.0:8547".parse().unwrap())
                .enable_rate_limiting(true)
                .default_rate_limit(500, 60) // Higher for testing
                .build()
        }
        "production" => {
            // Production environment - secure defaults
            let mut config = JsonRpcConfig::builder()
                .http_bind_address("0.0.0.0:8546".parse().unwrap())
                .ws_bind_address("0.0.0.0:8547".parse().unwrap())
                .enable_rate_limiting(true)
                .default_rate_limit(100, 60)
                .enable_websocket(true)
                .build();

            // Set specific CORS for production
            config.http.cors.allowed_origins =
                vec!["https://app.example.com".to_string()];
            config.features.strict_parameter_validation = true;
            config.features.strict_version_checking = true;

            config
        }
        _ => JsonRpcConfig::default(),
    }
}
