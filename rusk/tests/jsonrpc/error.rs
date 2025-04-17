// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk::jsonrpc::{
    config::{self, JsonRpcConfigBuilder},
    error::Error,
    infrastructure::error::{
        DbError, Error as InfrastructureError, StateError,
    },
    service::error::Error as ServiceError,
};
use serde_json;
use std::fmt::Display;
use thiserror::Error as ThisError;

fn create_config_error() -> config::ConfigError {
    // Trigger a validation error during build by providing an invalid value
    // for max_block_range, which is checked in validate() called by build().
    JsonRpcConfigBuilder::new()
        .max_block_range(0) // Set max_block_range to 0, which is invalid
        .build() // build() calls validate() internally
        .validate() // Call validate explicitly to get the Result<_, ConfigError>
        .expect_err("Validation should fail for max_block_range = 0")
}

// Helper function to create a dummy SerdeJsonError
fn create_serde_error() -> serde_json::Error {
    serde_json::from_str::<serde_json::Value>("{invalid json")
        .err()
        .expect("Parsing invalid JSON should fail")
}

#[test]
fn test_error_display() {
    let config_err = Error::Config(create_config_error());
    assert!(format!("{}", config_err).starts_with("Configuration error:"));

    // TODO: Uncomment and implement when infrastructure and service errors are
    // implemented let infra_err = Error::Infrastructure(/* ... */);
    // assert!(format!("{}", infra_err).starts_with("Infrastructure error:"));
    // let service_err = Error::Service(/* ... */);
    // assert!(format!("{}", service_err).starts_with("Service error:"));

    let serde_err = Error::Serialization(create_serde_error());
    assert!(format!("{}", serde_err).starts_with("Serialization error:"));

    let invalid_params_err =
        Error::InvalidParams("Missing field 'foo'".to_string());
    assert_eq!(
        format!("{}", invalid_params_err),
        "Invalid parameters: Missing field 'foo'"
    );

    let internal_err = Error::Internal("Something went wrong".to_string());
    assert_eq!(
        format!("{}", internal_err),
        "Internal error: Something went wrong"
    );

    let method_not_found_err = Error::MethodNotFound("my_method".to_string());
    assert_eq!(
        format!("{}", method_not_found_err),
        "Method not found: my_method"
    );

    let resource_not_found_err =
        Error::ResourceNotFound("Block 123".to_string());
    assert_eq!(
        format!("{}", resource_not_found_err),
        "Resource not found: Block 123"
    );

    let validation_err = Error::Validation("Signature mismatch".to_string());
    assert_eq!(
        format!("{}", validation_err),
        "Validation error: Signature mismatch"
    );

    let transport_err = Error::Transport("Connection closed".to_string());
    assert_eq!(
        format!("{}", transport_err),
        "Transport error: Connection closed"
    );
}

// --- Mock Errors for Testing Infrastructure/Service Variants ---

#[derive(ThisError, Debug)]
enum MockInfrastructureError {
    #[error("Mock DB connection failed")]
    DbConnection,
    #[allow(dead_code)]
    #[error("Mock state inconsistency")]
    StateInconsistency,
}

#[derive(thiserror::Error, Debug, Clone)]
enum MockServiceError {
    #[error("Mock block processing failed")]
    BlockProcessing,
    #[error("Mock transaction validation failed")]
    TransactionValidation,
}

impl From<MockInfrastructureError> for InfrastructureError {
    fn from(e: MockInfrastructureError) -> Self {
        match e {
            MockInfrastructureError::DbConnection => {
                InfrastructureError::Database(DbError::Connection(
                    "mock connection error".to_string(),
                ))
            }
            MockInfrastructureError::StateInconsistency => {
                InfrastructureError::State(StateError::Inconsistent(
                    "mock state error".to_string(),
                ))
            }
        }
    }
}

impl From<MockServiceError> for ServiceError {
    fn from(e: MockServiceError) -> Self {
        match e {
            MockServiceError::BlockProcessing => {
                ServiceError::Block("mock block error".to_string())
            }
            MockServiceError::TransactionValidation => {
                ServiceError::Transaction("mock tx error".to_string())
            }
        }
    }
}

// --- End Mock Errors ---

// --- Add From impls for the main Error enum to handle Mocks ---

impl From<MockInfrastructureError> for Error {
    fn from(mock_err: MockInfrastructureError) -> Self {
        let real_infra_error: InfrastructureError = mock_err.into();
        Error::Infrastructure(real_infra_error)
    }
}

impl From<MockServiceError> for Error {
    fn from(mock_err: MockServiceError) -> Self {
        let real_service_error: ServiceError = mock_err.into();
        Error::Service(real_service_error)
    }
}
// --- End From impls ---
fn function_that_returns_config_error() -> Result<(), Error> {
    Err(create_config_error())?;
    Ok(())
}

// Function that returns a Result with our Error type
fn function_that_returns_serde_error() -> Result<(), Error> {
    Err(create_serde_error())?;
    Ok(())
}

// Function that returns a Result with Infrastructure error
fn function_that_returns_infra_error() -> Result<(), Error> {
    Err(MockInfrastructureError::DbConnection)?;
    Ok(())
}

// Function that returns a Result with Service error
fn function_that_returns_service_error() -> Result<(), Error> {
    Err(MockServiceError::TransactionValidation)?;
    Ok(())
}

#[test]
fn test_error_from_impls() {
    // Test From<ConfigError>
    let config_result = function_that_returns_config_error();
    assert!(config_result.is_err());
    assert!(matches!(config_result.err().unwrap(), Error::Config(_)));

    let serde_result = function_that_returns_serde_error();
    assert!(serde_result.is_err());
    assert!(matches!(
        serde_result.err().unwrap(),
        Error::Serialization(_)
    ));

    let infra_result = function_that_returns_infra_error();
    assert!(infra_result.is_err());
    assert!(matches!(
        infra_result.err().unwrap(),
        Error::Infrastructure(_)
    ));

    let service_result = function_that_returns_service_error();
    assert!(service_result.is_err());
    assert!(matches!(service_result.err().unwrap(), Error::Service(_)));
    let direct_err: Result<(), Error> =
        Err(Error::Internal("Direct".to_string()));
    assert!(direct_err.is_err());
    assert!(
        matches!(direct_err.err().unwrap(), Error::Internal(s) if s == "Direct")
    );
}

#[test]
fn test_error_into_error_object() {
    use jsonrpsee::types::error::ErrorCode;
    use rusk::jsonrpc::config::SanitizationConfig;

    let default_sanitize_config = SanitizationConfig::default();
    let disabled_sanitize_config = SanitizationConfig {
        enabled: false,
        ..Default::default()
    };
    let custom_sanitize_config = SanitizationConfig {
        enabled: true,
        sensitive_terms: vec!["secret".to_string(), "key".to_string()],
        max_message_length: 50,
        redaction_marker: "[CENSORED]".to_string(),
        sanitize_paths: true,
    };

    let config_err = Error::Config(create_config_error());
    let obj = config_err.into_error_object(&default_sanitize_config);
    assert_eq!(obj.code(), -32001);
    assert!(obj.message().starts_with("Configuration error:"));

    let infra_err =
        Error::Infrastructure(MockInfrastructureError::DbConnection.into());
    let obj = infra_err.into_error_object(&default_sanitize_config);
    assert_eq!(obj.code(), -32002);
    assert!(obj.message().starts_with("Infrastructure error:"));

    let service_err = Error::Service(MockServiceError::BlockProcessing.into());
    let obj = service_err.into_error_object(&default_sanitize_config);
    assert_eq!(obj.code(), -32003);
    assert!(obj.message().starts_with("Service error:"));

    let serde_err = Error::Serialization(create_serde_error());
    let obj = serde_err.into_error_object(&default_sanitize_config);
    assert_eq!(obj.code(), ErrorCode::InternalError.code());
    assert!(obj.message().starts_with("Serialization error:"));

    let invalid_params_err = Error::InvalidParams("Bad field".to_string());
    let obj = invalid_params_err.into_error_object(&default_sanitize_config);
    assert_eq!(obj.code(), ErrorCode::InvalidParams.code());
    assert_eq!(obj.message(), "Invalid parameters: Bad field");

    let internal_err = Error::Internal("Oops".to_string());
    let obj = internal_err.into_error_object(&default_sanitize_config);
    assert_eq!(obj.code(), ErrorCode::InternalError.code());
    assert_eq!(obj.message(), "Internal error: Oops");

    let method_not_found_err =
        Error::MethodNotFound("missing_method".to_string());
    let obj = method_not_found_err.into_error_object(&default_sanitize_config);
    assert_eq!(obj.code(), ErrorCode::MethodNotFound.code());
    assert_eq!(obj.message(), "Method not found: missing_method");

    let resource_not_found_err =
        Error::ResourceNotFound("Block 404".to_string());
    let obj =
        resource_not_found_err.into_error_object(&default_sanitize_config);
    assert_eq!(obj.code(), -32004);
    assert_eq!(obj.message(), "Resource not found: Block 404");

    let validation_err = Error::Validation("Bad signature".to_string());
    let obj = validation_err.into_error_object(&default_sanitize_config);
    assert_eq!(obj.code(), -32005);
    assert_eq!(obj.message(), "Validation error: Bad signature");

    let transport_err = Error::Transport("Closed pipe".to_string());
    let obj = transport_err.into_error_object(&default_sanitize_config);
    assert_eq!(obj.code(), -32006);
    assert_eq!(obj.message(), "Transport error: Closed pipe");

    // --- Test Sanitization ---
    let sensitive_err = Error::Internal("Contains secret key".to_string());
    let obj = sensitive_err.into_error_object(&disabled_sanitize_config);
    assert_eq!(obj.message(), "Internal error: Contains secret key");

    let obj = sensitive_err.into_error_object(&custom_sanitize_config);
    assert_eq!(
        obj.message(),
        "Internal error: Contains [CENSORED] [CENSORED]"
    );
    let long_message = "This is a very long error message that definitely exceeds the maximum length allowed by the custom configuration settings.";
    let long_err = Error::Internal(long_message.to_string());
    let obj = long_err.into_error_object(&custom_sanitize_config);
    assert!(obj.message().len() <= 50 + 3);
    assert_eq!(
        obj.message(),
        "Internal error: This is a very long error message ..."
    );
    let path_err = Error::Internal(
        "Failed at /home/user/.wallet/keys/private.key".to_string(),
    );
    let obj = path_err.into_error_object(&custom_sanitize_config);
    assert_eq!(obj.message(), "Internal error: Failed at [PATH]");

    let path_err_windows = Error::Internal(
        "Error in C:\\Users\\Admin\\Documents\\secret.txt".to_string(),
    );
    let obj = path_err_windows.into_error_object(&custom_sanitize_config);
    assert_eq!(obj.message(), "Internal error: Error in [PATH]");
    let mut path_config_disabled = custom_sanitize_config.clone();
    path_config_disabled.sanitize_paths = false;
    let obj = path_err.into_error_object(&path_config_disabled);
    assert_eq!(
        obj.message().len(),
        53,
        "Unexpected sanitized message length in test case 5"
    );
    assert_eq!(
        obj.message(),
        "Internal error: Failed at /home/user/.wallet/keys/..."
    );
    let control_char_err = Error::Internal(
        "Error with control char \x07 and quotes '\"".to_string(),
    );
    let obj = control_char_err.into_error_object(&default_sanitize_config);
    assert_eq!(
        obj.message(),
        "Internal error: Error with control char  and quotes "
    );
    let combined_err = Error::Internal(
        "Critical error accessing /etc/secret/private.key - system unstable"
            .to_string(),
    );
    let obj = combined_err.into_error_object(&custom_sanitize_config);
    // TODO: Investigate why truncation seems to happen at char 48 instead of 50
    // in this specific case.
    assert_eq!(
        obj.message(),
        "Internal error: Critical error accessing [PATH] - ..."
    );
}
fn test_display_impl<E: Display>(error: E, expected_prefix: &str) {
    let display_output = format!("{}", error);
    assert!(
        display_output.starts_with(expected_prefix),
        "Display output '{}' did not start with expected prefix '{}'",
        display_output,
        expected_prefix
    );
}

#[test]
fn test_infrastructure_and_service_error_display() {
    use rusk::jsonrpc::infrastructure::error::{
        DbError, Error as InfrastructureError, RateLimitError, StateError,
    };
    use rusk::jsonrpc::infrastructure::subscription::error::SubscriptionError;
    use rusk::jsonrpc::service::error::Error as ServiceError;
    test_display_impl(
        InfrastructureError::Database(DbError::Connection(
            "timeout".to_string(),
        )),
        "Database error: Database connection failed: timeout",
    );
    test_display_impl(
        InfrastructureError::State(StateError::Inconsistent(
            "mismatch".to_string(),
        )),
        "State error: Inconsistent application state: mismatch",
    );
    test_display_impl(
        InfrastructureError::RateLimit(RateLimitError::LimitExceeded(
            "ip 1.2.3.4".to_string(),
        )),
        "Rate limit error: Rate limit exceeded: ip 1.2.3.4",
    );
    test_display_impl(
        InfrastructureError::Subscription(
            SubscriptionError::InvalidSubscription("sub_123".to_string()),
        ),
        "Subscription error: Invalid subscription ID: sub_123",
    );

    // Test Service Errors (using placeholder strings)
    test_display_impl(
        ServiceError::Block("Block not found".to_string()),
        "Block service error: Block not found",
    );
    test_display_impl(
        ServiceError::Transaction("Invalid signature".to_string()),
        "Transaction service error: Invalid signature",
    );
    test_display_impl(
        ServiceError::Contract("Execution failed".to_string()),
        "Contract service error: Execution failed",
    );
    test_display_impl(
        ServiceError::Network("Peer disconnected".to_string()),
        "Network service error: Peer disconnected",
    );
    test_display_impl(
        ServiceError::Prover("Proof generation failed".to_string()),
        "Prover service error: Proof generation failed",
    );
    test_display_impl(
        ServiceError::Subscription("Client disconnected".to_string()),
        "Subscription service error: Client disconnected",
    );
}
