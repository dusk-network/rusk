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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jsonrpc::config::SanitizationConfig;

    // Tests for the sanitize_message function are placed here,
    // within the same module where sanitize_message is defined,
    // because we want to keep sanitize_message private and test its internal
    // behavior directly.

    #[test]
    fn test_sanitization_disabled() {
        let config = SanitizationConfig {
            enabled: false,
            ..Default::default()
        };
        let message = "Sensitive password info /path/to/key";
        assert_eq!(sanitize_message(message, &config), message);
    }

    #[test]
    fn test_basic_term_redaction() {
        let config = SanitizationConfig {
            enabled: true,
            sensitive_terms: vec!["password".to_string(), "secret".to_string()],
            redaction_marker: "[REDACTED]".to_string(),
            ..Default::default()
        };
        let message = "User entered password Password SECRET secret";
        let expected =
            "User entered [REDACTED] [REDACTED] [REDACTED] [REDACTED]";
        assert_eq!(sanitize_message(message, &config), expected);
    }

    #[test]
    fn test_path_sanitization_unix() {
        let config = SanitizationConfig {
            enabled: true,
            sanitize_paths: true,
            ..Default::default()
        };
        let message = "Error accessing /home/user/.ssh/id_rsa or ./local/file";
        let expected = "Error accessing [PATH] or [PATH]";
        assert_eq!(sanitize_message(message, &config), expected);
    }

    #[test]
    fn test_path_sanitization_windows() {
        let config = SanitizationConfig {
            enabled: true,
            sanitize_paths: true,
            ..Default::default()
        };
        let message = r#"Failed on C:\Users\Admin\secret.key and \\server\share\data.txt"#;
        let expected = "Failed on [PATH] and [PATH]";
        assert_eq!(sanitize_message(message, &config), expected);
    }

    #[test]
    fn test_path_sanitization_disabled() {
        let config = SanitizationConfig {
            enabled: true,
            sanitize_paths: false,   // Disabled
            max_message_length: 100, // Keep length reasonable for test
            ..Default::default()
        };
        let message = "Error accessing /home/user/.ssh/id_rsa";
        // Path should NOT be replaced, but quotes/control chars still would be
        // if present
        assert_eq!(sanitize_message(message, &config), message);
    }

    #[test]
    fn test_control_char_filtering() {
        let config = SanitizationConfig {
            enabled: true,
            ..Default::default()
        };
        let message = "Error \t with \n control \x07 chars";
        let expected = "Error  with  control  chars"; // Tabs, newlines, bell removed
        assert_eq!(sanitize_message(message, &config), expected);
    }

    #[test]
    fn test_quote_filtering() {
        let config = SanitizationConfig {
            enabled: true,
            ..Default::default()
        };
        let message = "User's 'input' contained \"quotes\"";
        let expected = "Users input contained quotes";
        assert_eq!(sanitize_message(message, &config), expected);
    }

    #[test]
    fn test_truncation() {
        let config = SanitizationConfig {
            enabled: true,
            max_message_length: 10,
            ..Default::default()
        };
        let message = "This message is too long";
        let expected = "This messa..."; // 10 chars + ...
        assert_eq!(sanitize_message(message, &config), expected);
    }

    #[test]
    fn test_truncation_exact_length() {
        let config = SanitizationConfig {
            enabled: true,
            max_message_length: 15,
            ..Default::default()
        };
        let message = "Exact length 15"; // Exactly 15 chars
        assert_eq!(sanitize_message(message, &config), message); // No truncation
    }

    #[test]
    fn test_combination() {
        let config = SanitizationConfig {
            enabled: true,
            sensitive_terms: vec!["token".to_string()],
            redaction_marker: "[XXX]".to_string(),
            sanitize_paths: true,
            max_message_length: 30,
            ..Default::default()
        };
        let message = "Invalid token found in /etc/secrets/api.token file!";
        // Input: "Invalid token found in /etc/secrets/api.token file!" (50
        // chars) After term: "Invalid [XXX] found in
        // /etc/secrets/api.[XXX] file!" After path: "Invalid [XXX]
        // found in [PATH] file!" (29 chars) After filter: "Invalid
        // [XXX] found in [PATH] file!" (29 chars) Length 29 <= max 30,
        // so no truncation.
        let expected = "Invalid [XXX] found in [PATH] ...";
        assert_eq!(sanitize_message(message, &config), expected); // Reverted to original expectation after verifying logic again.

        let message_long = "Invalid access token 'abc' found in /etc/secrets/api.token, causing failure.";
        // Input: "Invalid access token 'abc' found in /etc/secrets/api.token,
        // causing failure." After term: "Invalid access [XXX] 'abc'
        // found in /etc/secrets/api.[XXX], causing failure."
        // After path: "Invalid access [XXX] 'abc' found in [PATH], causing
        // failure." After filter: "Invalid access [XXX] abc found in
        // [PATH], causing failure." (58 chars) Length 58 > max 30,
        // truncate. take(30) = "Invalid access [XXX] abc found" Append
        // "...": "Invalid access [XXX] abc found..."
        let expected_long = "Invalid access [XXX] abc found...";
        assert_eq!(sanitize_message(message_long, &config), expected_long);
    }

    #[test]
    fn test_empty_input() {
        let config = SanitizationConfig {
            enabled: true,
            ..Default::default()
        };
        assert_eq!(sanitize_message("", &config), "");
    }

    #[test]
    fn test_no_sensitive_terms_config() {
        let config = SanitizationConfig {
            enabled: true,
            sensitive_terms: vec![], // Empty list
            ..Default::default()
        };
        let message = "This has password but shouldn't be redacted";
        // Apostrophe will be removed by quote filtering
        let expected = "This has password but shouldnt be redacted";
        assert_eq!(sanitize_message(message, &config), expected);
    }

    #[test]
    fn test_unicode_handling() {
        // Test truncation with multi-byte characters
        let config_trunc = SanitizationConfig {
            enabled: true,
            max_message_length: 5,
            ..Default::default()
        };
        let message_unicode = "你好世界🌍"; // 5 chars counted by .chars().count()
                                            // Length 5 is NOT > max_length 5, so no truncation occurs.
        let expected_trunc = "你好世界🌍";
        assert_eq!(
            sanitize_message(message_unicode, &config_trunc),
            expected_trunc
        );

        // Test term redaction with unicode
        let config_redact = SanitizationConfig {
            enabled: true,
            sensitive_terms: vec!["世界".to_string()], // Unicode term
            redaction_marker: "[PLANET]".to_string(),
            ..Default::default()
        };
        let message_redact = "你好 世界";
        let expected_redact = "你好 [PLANET]";
        assert_eq!(
            sanitize_message(message_redact, &config_redact),
            expected_redact
        );
    }
}
