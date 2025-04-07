// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Central error handling for the Rusk JSON-RPC module.
//!
//! This module defines the primary `Error` enum used throughout the `jsonrpc`
//! crate. It leverages the `thiserror` crate to create a structured error type
//! that consolidates errors from various layers (configuration, infrastructure,
//! service) and also includes variants for common JSON-RPC specific error
//! conditions.
//!
//! ## Error Handling Strategy
//!
//! The error handling follows a two-step process:
//!
//! 1. **Internal Handling:** Within the Rusk application logic (services,
//!    infrastructure components), functions return `Result<T,
//!    jsonrpc::error::Error>`. This allows for specific internal error variants
//!    to be propagated and handled appropriately within the server. Underlying
//!    errors from dependencies (like `config::ConfigError`,
//!    `infrastructure::error::Error`, `service::error::Error`,
//!    `serde_json::Error`) are wrapped using the `#[from]` attribute.
//!
//! 2. **External Mapping:** Before sending a response back to the JSON-RPC
//!    client, the internal `jsonrpc::error::Error` must be converted into a
//!    `jsonrpsee::types::ErrorObjectOwned`. This conversion is handled by the
//!    [`Error::into_error_object`] method and is the designated point for:
//!     * Mapping internal error types to standard JSON-RPC error codes (e.g.,
//!       -32602 for `InvalidParams`, -32000 for `ResourceNotFound`).
//!     * Formatting user-friendly error messages.
//!     * Applying sanitization rules (based on [`config::SanitizationConfig`])
//!       to prevent leaking sensitive information.
//!
//! This approach keeps internal error handling expressive and detailed while
//! ensuring that client-facing errors adhere to the JSON-RPC specification and
//! security best practices.

use crate::jsonrpc::config::{self, SanitizationConfig};
use crate::jsonrpc::infrastructure;
use crate::jsonrpc::service;
use jsonrpsee::types::{error::ErrorCode, ErrorObjectOwned};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json;
use std::borrow::Cow;
use thiserror::Error;
use tracing::warn;

static PATH_REGEX: Lazy<Regex> = Lazy::new(|| {
    match Regex::new(r#"(?:[a-zA-Z]:\\|\\|/|\./|\.\./)[^[:cntrl:]"'\s]*"#) {
        Ok(re) => re,
        Err(e) => {
            warn!(error = %e, "Failed to compile PATH_REGEX for error sanitization");
            Regex::new("$^").unwrap()
        }
    }
});

/// Central error type for the JSON-RPC module.
///
/// This enum consolidates errors from different layers and provides
/// specific variants for common JSON-RPC error scenarios.
#[derive(Error, Debug)]
pub enum Error {
    /// Configuration loading or validation error.
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    /// Error originating from the infrastructure layer (DB, State, etc.).
    #[error("Infrastructure error: {0}")]
    Infrastructure(#[from] infrastructure::error::Error),

    /// Error originating from the service layer (business logic).
    #[error("Service error: {0}")]
    Service(#[from] service::error::Error),

    /// Error during JSON serialization or deserialization.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Error due to invalid parameters in the JSON-RPC request.
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    /// General internal server error.
    #[error("Internal error: {0}")]
    Internal(String),

    /// Requested JSON-RPC method was not found.
    #[error("Method not found: {0}")]
    MethodNotFound(String),

    /// A requested resource (e.g., block, transaction) could not be found.
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    /// General validation error not covered by InvalidParams.
    #[error("Validation error: {0}")]
    Validation(String),

    /// Error related to the transport layer (HTTP/WebSocket), if needed later.
    #[error("Transport error: {0}")]
    Transport(String),
}

/// Sanitizes an error message based on the provided configuration.
///
/// This function applies several sanitization steps:
/// 1. Checks if sanitization is enabled.
/// 2. Redacts sensitive terms defined in the configuration.
/// 3. Sanitizes potential file paths if enabled.
/// 4. Filters control characters and quotes.
/// 5. Truncates the message to the configured maximum length.
///
/// Uses `Cow` to avoid allocating a new string if no changes are made.
fn sanitize_message<'a>(
    message: &'a str,
    config: &SanitizationConfig,
) -> Cow<'a, str> {
    if !config.enabled {
        return Cow::Borrowed(message);
    }

    // Start with a borrowed Cow
    let mut sanitized_cow = Cow::Borrowed(message);

    // 1. Redact sensitive terms (case-insensitive, whole word)
    for term in &config.sensitive_terms {
        match Regex::new(&format!(r"(?i)\b{}\b", regex::escape(term))) {
            Ok(re) => {
                let replaced =
                    re.replace_all(&sanitized_cow, &config.redaction_marker);
                if let Cow::Owned(s) = replaced {
                    // Update Cow only if a replacement actually happened
                    sanitized_cow = Cow::Owned(s);
                }
            }
            Err(e) => {
                warn!(term = %term, error = %e, "Failed to compile regex for sensitive term redaction");
            }
        }
    }

    // 2. Sanitize paths
    if config.sanitize_paths {
        let replaced = PATH_REGEX.replace_all(&sanitized_cow, "[PATH]");
        if let Cow::Owned(s) = replaced {
            // Update Cow only if a replacement actually happened
            sanitized_cow = Cow::Owned(s);
        }
    }

    // 3. Filter control characters and quotes
    let needs_filtering = sanitized_cow
        .chars()
        .any(|c| c.is_control() || c == '"' || c == '\'');

    if needs_filtering {
        // Convert to owned string only if filtering is needed
        let mut owned_string = sanitized_cow.into_owned();
        owned_string.retain(|c| !c.is_control() && c != '"' && c != '\'');
        sanitized_cow = Cow::Owned(owned_string); // Update Cow
    }

    // 4. Truncate message based on character count
    // Note: This counts grapheme clusters (what users perceive as characters)
    // which might differ slightly from `char` count if complex scripts are
    // involved, but it's generally closer to the intended behavior than
    // byte truncation. For simplicity and performance, we'll use
    // `chars().count()` here. If precise grapheme cluster counting is
    // needed, the `unicode-segmentation` crate could be used, but adds a
    // dependency.
    let char_count = sanitized_cow.chars().count();
    if char_count > config.max_message_length {
        // Truncate based on character count
        let truncated_string: String = sanitized_cow
            .chars()
            .take(config.max_message_length)
            .collect();
        // Append ellipsis
        sanitized_cow = Cow::Owned(format!("{}...", truncated_string));
    }

    // Return the final Cow (might be Borrowed or Owned)
    sanitized_cow
}

impl Error {
    /// Converts the internal `Error` into a `jsonrpsee` compatible error
    /// object.
    ///
    /// This method performs the crucial mapping from internal error types to
    /// standard JSON-RPC error codes and messages. It also applies message
    /// sanitization based on the provided `SanitizationConfig`.
    ///
    /// # Arguments
    ///
    /// * `config` - The sanitization configuration to apply.
    ///
    /// # Arguments
    ///
    /// * `config` - The sanitization configuration to apply.
    ///
    /// # Returns
    ///
    /// An `ErrorObjectOwned` suitable for returning in a JSON-RPC response.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::jsonrpc::error::Error;
    /// use rusk::jsonrpc::config::SanitizationConfig;
    ///
    /// let internal_error = Error::Internal("Something failed".to_string());
    /// let config = SanitizationConfig::default(); // Use default sanitization
    ///
    /// let error_object = internal_error.into_error_object(&config);
    ///
    /// assert_eq!(error_object.code(), -32603);
    /// assert_eq!(error_object.message(), "Internal error: Something failed");
    /// ```
    ///
    /// ```rust
    /// use rusk::jsonrpc::error::Error;
    /// use rusk::jsonrpc::config::SanitizationConfig;
    ///
    /// let sensitive_error = Error::Validation("Invalid password".to_string());
    /// let mut config = SanitizationConfig::default();
    /// config.sensitive_terms = vec!["password".to_string()];
    /// config.redaction_marker = "[HIDDEN]".to_string();
    ///
    /// let error_object = sensitive_error.into_error_object(&config);
    ///
    /// assert_eq!(error_object.code(), -32005);
    /// assert_eq!(error_object.message(), "Validation error: Invalid [HIDDEN]");
    /// ```
    pub fn into_error_object(
        &self,
        config: &SanitizationConfig,
    ) -> ErrorObjectOwned {
        let (code, message) = match self {
            Error::Config(e) => (
                ErrorCode::ServerError(-32001), // Custom server error code
                format!("Configuration error: {}", e),
            ),
            Error::Infrastructure(e) => (
                ErrorCode::ServerError(-32002), // Custom server error code
                format!("Infrastructure error: {}", e),
            ),
            Error::Service(e) => (
                ErrorCode::ServerError(-32003), // Custom server error code
                format!("Service error: {}", e),
            ),
            Error::Serialization(e) => (
                ErrorCode::InternalError, // Standard code (-32603)
                format!("Serialization error: {}", e),
            ),
            Error::InvalidParams(msg) => (
                ErrorCode::InvalidParams, // Standard code (-32602)
                format!("Invalid parameters: {}", msg),
            ),
            Error::Internal(msg) => (
                ErrorCode::InternalError, // Standard code (-32603)
                format!("Internal error: {}", msg),
            ),
            Error::MethodNotFound(method) => (
                ErrorCode::MethodNotFound, // Standard code (-32601)
                format!("Method not found: {}", method),
            ),
            Error::ResourceNotFound(resource) => (
                ErrorCode::ServerError(-32004), // Custom server error code
                format!("Resource not found: {}", resource),
            ),
            Error::Validation(msg) => (
                ErrorCode::ServerError(-32005), // Custom server error code
                format!("Validation error: {}", msg),
            ),
            Error::Transport(msg) => (
                ErrorCode::ServerError(-32006), // Custom server error code
                format!("Transport error: {}", msg),
            ),
        };

        // Sanitize the message before creating the final error object
        let sanitized_message = sanitize_message(&message, config);

        ErrorObjectOwned::owned(code.code(), sanitized_message, None::<()>)
    }
}
