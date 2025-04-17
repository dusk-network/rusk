// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # JSON-RPC Server Configuration
//!
//! This module provides comprehensive configuration management for the Rusk
//! JSON-RPC server, incorporating robust security validation. It facilitates
//! loading configuration settings from multiple sources including TOML files
//! and environment variables, providing sensible defaults for all options.
//!
//! For a detailed external overview of all configuration options, the TOML file
//! format, environment variable mappings, and security considerations, please
//! refer to: [docs/jsonrpc_configuration.md](../../../docs/
//! jsonrpc_configuration.md)
//!
//! ## Core Features
//!
//! - **Layered Configuration:** Loads settings with clear precedence:
//!   Environment Variables > Configuration File > Default Values.
//! - **TOML File Support:** Reads configuration from a specified TOML file
//!   (default: `default.config.toml` at project root), expecting settings under
//!   the `[jsonrpc]` section.
//! - **Environment Variable Overrides:** Allows overriding any configuration
//!   field via environment variables prefixed with `RUSK_JSONRPC_` (e.g.,
//!   `RUSK_JSONRPC_HTTP_BIND_ADDRESS`).
//! - **Default Values:** Provides safe and reasonable defaults for every
//!   setting, ensuring the server can run without explicit configuration.
//! - **Security Validation:** Includes built-in checks (`validate()`) to
//!   prevent common insecure configurations (e.g., public binding without rate
//!   limits, insecure CORS).
//! - **Builder Pattern:** Offers a fluent builder API
//!   (`JsonRpcConfig::builder()`) for programmatic configuration, useful for
//!   testing or embedding.
//! - **Sanitization:** Configurable redaction of sensitive terms and path
//!   information in logs and error messages.
//!
//! ## Configuration Sections
//!
//! - `[jsonrpc.http]`: HTTP server settings (address, timeouts, CORS, TLS).
//! - `[jsonrpc.ws]`: WebSocket server settings (address, limits, timeouts).
//! - `[jsonrpc.rate_limit]`: Rate limiting rules (global, default,
//!   method-specific).
//! - `[jsonrpc.features]`: Toggles for specific server features (WebSocket
//!   support, error detail, block range limits).
//! - `[jsonrpc.sanitization]`: Control over message sanitization and sensitive
//!   data redaction.
//!
//! ## Usage Examples
//!
//! ### Loading Default Configuration
//!
//! The easiest way to get started is by loading the default configuration,
//! which reads `default.config.toml` if present and applies environment
//! variables.
//!
//! ```rust
//! use rusk::jsonrpc::config::{JsonRpcConfig, ConfigError};
//!
//! match JsonRpcConfig::load_default() {
//!     Ok(config) => {
//!         println!("Successfully loaded default config. HTTP runs on: {}", config.http.bind_address);
//!         // Proceed with using the config...
//!     }
//!     Err(e) => {
//!         eprintln!("Failed to load default JSON-RPC config: {}", e);
//!         // Handle error, potentially exiting the application
//!     }
//! }
//! ```
//!
//! ### Loading from a Specific File
//!
//! You can specify a custom configuration file path.
//!
//! ```rust
//! use rusk::jsonrpc::config::{JsonRpcConfig, ConfigError};
//! use std::path::Path;
//! use std::fs;
//!
//! fn load_from_custom_file(path_str: &str) -> Result<JsonRpcConfig, Box<dyn std::error::Error>> {
//!     let path = Path::new(path_str);
//!
//!     // Example: Create a temporary config file for the doc test
//!     let content = r#"
//! [jsonrpc]
//! [jsonrpc.http]
//! bind_address = "127.0.0.1:9999"
//! max_body_size = 1024
//! [jsonrpc.features]
//! max_block_range = 50
//! "#;
//!     fs::write(path, content)?;
//!
//!     // Load configuration specifically from this file
//!     let config = JsonRpcConfig::load(Some(path))?;
//!     assert_eq!(config.http.bind_address.port(), 9999);
//!     assert_eq!(config.features.max_block_range, 50);
//!     println!("Loaded config from {}: HTTP port={}, Max block range={}",
//!              path.display(), config.http.bind_address.port(), config.features.max_block_range);
//!
//!     // Clean up the temporary file
//!     fs::remove_file(path)?;
//!     Ok(config)
//! }
//!
//! // To run this example, ensure you have write permissions in the execution directory
//! // load_from_custom_file("my_custom_rpc_config.toml").expect("Failed to run custom file load example");
//! ```
//!
//! *Note: Environment variables starting with `RUSK_JSONRPC_` will still
//! override values from the specified file when using `load()`.*
//! *Use `load_from_file_only()` (typically for tests) to ignore environment
//! variables.*
//!
//! ### Using the Builder Pattern
//!
//! Configure the server programmatically.
//!
//! ```rust
//! use rusk::jsonrpc::config::JsonRpcConfig;
//! use std::net::SocketAddr;
//! use std::str::FromStr;
//! use std::time::Duration;
//!
//! let config = JsonRpcConfig::builder()
//!     .http_bind_address(SocketAddr::from_str("192.168.1.100:8080").unwrap())
//!     .enable_websocket(false)
//!     .enable_rate_limiting(true)
//!     .default_rate_limit(150, 60) // 150 requests per minute default
//!     .add_method_rate_limit("getBlockByHeight", 500, 60) // Higher limit for specific method
//!     .max_block_range(2000)
//!     .enable_sanitization(true)
//!     .add_sensitive_term("internal_user_id")
//!     .build();
//!
//! assert!(!config.features.enable_websocket);
//! assert!(config.rate_limit.enabled);
//! assert_eq!(config.rate_limit.default_limit.requests, 150);
//! assert_eq!(config.features.max_block_range, 2000);
//! assert!(config.sanitization.sensitive_terms.contains(&"internal_user_id".to_string()));
//! println!("Built custom config: HTTP Address = {}, Rate Limiting Enabled = {}",
//!          config.http.bind_address, config.rate_limit.enabled);
//! ```
//!
//! ### Configuration Validation
//!
//! The `validate()` method checks for logical inconsistencies and security
//! issues. It's called automatically by `load()` and `load_default()`.
//!
//! ```rust
//! use rusk::jsonrpc::config::{JsonRpcConfig, ConfigError};
//!
//! let mut insecure_config = JsonRpcConfig::builder()
//!     .http_bind_address("0.0.0.0:8080".parse().unwrap()) // Public binding
//!     .enable_rate_limiting(false) // But rate limiting disabled
//!     .build();
//!
//! // Validation should fail due to the security violation
//! match insecure_config.validate() {
//!     Ok(_) => panic!("Validation should have failed!"),
//!     Err(ConfigError::SecurityViolation(msg)) => {
//!         println!("Validation failed as expected: {}", msg);
//!         assert!(msg.contains("public interface without rate limiting"));
//!     }
//!     Err(e) => panic!("Unexpected validation error type: {}", e),
//! }
//!
//! let mut valid_config = JsonRpcConfig::builder()
//!     .http_bind_address("127.0.0.1:8081".parse().unwrap())
//!     .enable_rate_limiting(true)
//!     .default_rate_limit(100, 60)
//!     .build();
//!
//! // This configuration should validate successfully
//! assert!(valid_config.validate().is_ok());
//! println!("Valid configuration passed validation.");
//! ```
//!
//! ### Environment Variable Override Example
//!
//! ```rust,no_run
//! // Set environment variables *before* loading the config.
//! // This usually happens outside the Rust code (e.g., in the shell or a .env file).
//! std::env::set_var("RUSK_JSONRPC_HTTP_BIND_ADDRESS", "127.0.0.1:9999");
//! std::env::set_var("RUSK_JSONRPC_RATE_LIMIT_ENABLED", "false");
//!
//! use rusk::jsonrpc::config::JsonRpcConfig;
//!
//! // Load defaults, which will pick up the environment variables
//! let config = JsonRpcConfig::load_default().expect("Failed to load config");
//!
//! assert_eq!(config.http.bind_address.port(), 9999);
//! assert!(!config.rate_limit.enabled);
//!
//! println!("Config loaded with overrides: Port={}, Rate Limit Enabled={}",
//!          config.http.bind_address.port(), config.rate_limit.enabled);
//!
//! // Clean up environment variables (important in tests)
//! std::env::remove_var("RUSK_JSONRPC_HTTP_BIND_ADDRESS");
//! std::env::remove_var("RUSK_JSONRPC_RATE_LIMIT_ENABLED");
//! ```
//!
//! ## Sanitization Configuration
//!
//! Control how sensitive information is redacted from logs and error messages.
//!
//! ```rust
//! use rusk::jsonrpc::config::JsonRpcConfig;
//!
//! let config = JsonRpcConfig::builder()
//!     .enable_sanitization(true)
//!     .redaction_marker("[CONFIDENTIAL]")
//!     .max_message_length(100)
//!     .sanitize_paths(true)
//!     .add_sensitive_terms(&["api_token", "user_password"])
//!     .build();
//!
//! assert!(config.sanitization.enabled);
//! assert_eq!(config.sanitization.redaction_marker, "[CONFIDENTIAL]");
//! assert_eq!(config.sanitization.max_message_length, 100);
//! assert!(config.sanitization.sanitize_paths);
//! assert!(config.sanitization.sensitive_terms.contains(&"api_token".to_string()));
//! println!("Sanitization configured: Marker='{}', Max Length={}",
//!          config.sanitization.redaction_marker, config.sanitization.max_message_length);
//! ```
//!
//! For more details on specific fields, see the documentation for
//! [`JsonRpcConfig`] and its nested structs like [`HttpServerConfig`],
//! [`RateLimitConfig`], etc.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;
use std::{env, fs};

use http::{HeaderName, Method};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::{debug, error, info, instrument, warn};

/// Helper function to parse common boolean string representations from env
/// vars. Case-insensitive, accepts "true", "1", "yes", "on" as true,
/// and "false", "0", "no", "off" as false.
fn parse_bool_env(value: &str) -> Option<bool> {
    match value.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None, // Not a recognized boolean string
    }
}

/// Errors that can occur when loading or validating configuration.
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Error reading configuration file
    #[error("Failed to read configuration file: {0}")]
    FileRead(#[from] std::io::Error),

    /// Error parsing TOML
    #[error("Failed to parse TOML: {0}")]
    TomlParse(#[from] toml::de::Error),

    /// Error serializing to TOML
    #[error("Failed to serialize to TOML: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    /// Error parsing environment variable
    #[error("Failed to parse environment variable {0}: {1}")]
    EnvParse(String, String),

    /// Validation error
    #[error("Configuration validation error: {0}")]
    Validation(String),

    /// Security validation error
    #[error("Security configuration error: {0}")]
    SecurityViolation(String),
}

/// Main configuration for the JSON-RPC server.
///
/// This configuration is intended to be created at startup and then
/// shared as read-only data. It can be safely shared between threads
/// as long as it's not modified after initialization.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct JsonRpcConfig {
    /// HTTP server configuration
    #[serde(default)]
    pub http: HttpServerConfig,

    /// WebSocket server configuration
    #[serde(default)]
    pub ws: WebSocketServerConfig,

    /// Rate limiting configuration
    #[serde(default)]
    pub rate_limit: RateLimitConfig,

    /// Feature toggles
    #[serde(default)]
    pub features: FeatureToggles,

    /// Error message sanitization configuration
    #[serde(default)]
    pub sanitization: SanitizationConfig,
}

/// Builder for JsonRpcConfig
pub struct JsonRpcConfigBuilder {
    config: JsonRpcConfig,
}

/// HTTP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HttpServerConfig {
    /// Socket address to bind the HTTP server to
    #[serde(with = "socket_addr_serde", default = "default_http_address")]
    pub bind_address: SocketAddr,

    /// Maximum request body size in bytes
    pub max_body_size: usize,

    /// Request timeout in seconds
    #[serde(with = "duration_serde", default = "default_http_timeout")]
    pub request_timeout: Duration,

    /// Maximum number of concurrent connections
    pub max_connections: usize,

    /// CORS configuration
    #[serde(default)]
    pub cors: CorsConfig,

    /// Optional path to the TLS certificate file (PEM format)
    pub cert: Option<PathBuf>,

    /// Optional path to the TLS private key file (PEM format)
    pub key: Option<PathBuf>,
}

/// WebSocket server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WebSocketServerConfig {
    /// Socket address to bind the WebSocket server to
    #[serde(with = "socket_addr_serde", default = "default_ws_address")]
    pub bind_address: SocketAddr,

    /// Maximum message size in bytes
    pub max_message_size: usize,

    /// Maximum number of concurrent connections
    pub max_connections: usize,

    /// Maximum number of subscriptions per connection
    pub max_subscriptions_per_connection: usize,

    /// Idle timeout in seconds
    #[serde(with = "duration_serde", default = "default_idle_timeout")]
    pub idle_timeout: Duration,

    /// Maximum number of events per second per connection
    pub max_events_per_second: usize,
}

/// CORS configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CorsConfig {
    /// Whether CORS is enabled
    pub enabled: bool,

    /// Allowed origins (empty means all origins)
    pub allowed_origins: Vec<String>,

    /// Allowed methods
    pub allowed_methods: Vec<String>,

    /// Allowed headers
    pub allowed_headers: Vec<String>,

    /// Whether to allow credentials
    pub allow_credentials: bool,

    /// Maximum age for preflight requests in seconds
    pub max_age_seconds: u64,
}

/// Rate limiting configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RateLimitConfig {
    /// Whether rate limiting is enabled globally (affects both middleware and
    /// manual checks).
    pub enabled: bool,

    /// Default rate limit applied by the `tower-governor` middleware to all
    /// HTTP requests. This limit is typically based on the client's IP
    /// address. It is NOT used by the `ManualRateLimiters`.
    pub default_limit: RateLimit,

    /// Method-specific rate limits applied manually within RPC method handlers
    /// using `ManualRateLimiters::check_method_limit`.
    /// These limits are checked *after* the default middleware limit passes.
    pub method_limits: Vec<MethodRateLimit>,

    /// Rate limit for establishing new WebSocket connections, applied manually
    /// during connection setup using
    /// `ManualRateLimiters::check_websocket_limit`. This limit is checked
    /// *before* the default middleware limit (as WS upgrade is HTTP first),
    /// but applies specifically to the connection attempt rate.
    pub websocket_limit: RateLimit,
}

/// Rate limit for a specific method or endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MethodRateLimit {
    /// Method name pattern (supports wildcards)
    pub method_pattern: String,

    /// Rate limit configuration
    pub limit: RateLimit,
}

/// Rate limit configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RateLimit {
    /// Maximum number of requests
    pub requests: u64,

    /// Time window for the request limit in seconds
    #[serde(with = "duration_serde", default = "default_rate_limit_window")]
    pub window: Duration,
}

/// Feature toggles for the JSON-RPC server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FeatureToggles {
    /// Whether to enable WebSocket subscriptions
    pub enable_websocket: bool,

    /// Whether to enable detailed error messages
    pub detailed_errors: bool,

    /// Whether to enable method timing metrics
    pub method_timing: bool,

    /// Whether to enable strict version checking
    pub strict_version_checking: bool,

    /// Whether to validate parameters strictly
    pub strict_parameter_validation: bool,

    /// Maximum block range allowed for range queries
    pub max_block_range: u64,

    /// Maximum items in batch requests
    pub max_batch_size: usize,
}

/// Message sanitization configuration.
///
/// This struct contains settings that control how error messages are sanitized
/// to prevent sensitive information from being exposed in API responses.
///
/// # Security Considerations
///
/// - Proper message sanitization is critical for preventing information leakage
/// - Default settings provide a baseline level of protection
/// - Consider adding organization-specific sensitive terms
/// - Path sanitization should remain enabled in production environments
///
/// # Examples
///
/// Customizing sanitization via the config file:
///
/// ```toml
/// [sanitization]
/// enabled = true
/// sensitive_terms = ["password", "secret", "key", "token", "credential"]
/// max_message_length = 200
/// redaction_marker = "[REDACTED]"
/// sanitize_paths = true
/// ```
///
/// Programmatically customizing sanitization settings using the builder
/// pattern:
///
/// ```rust
/// use rusk::jsonrpc::config::JsonRpcConfig;
///
/// // Configure sanitization using the builder pattern
/// let config = JsonRpcConfig::builder()
///     // Configure HTTP settings
///     .http_bind_address("127.0.0.1:8546".parse().unwrap())
///     // Configure sanitization settings
///     .enable_sanitization(true)
///     .max_message_length(500)
///     .redaction_marker("[CLASSIFIED]")
///     .sanitize_paths(true)
///     // Add organization-specific sensitive terms
///     .add_sensitive_term("internal_id")
///     .add_sensitive_term("company_secret")
///     // Add multiple terms at once
///     .add_sensitive_terms(&["org_key", "customer_id", "project_name"])
///     // Build the final config
///     .build();
///
/// // Verify our configuration
/// assert_eq!(config.sanitization.redaction_marker, "[CLASSIFIED]");
/// assert_eq!(config.sanitization.max_message_length, 500);
/// assert!(config.sanitization.sensitive_terms.contains(&"internal_id".to_string()));
/// ```
///
/// For testing, you might want to disable sanitization temporarily:
///
/// ```rust
/// use rusk::jsonrpc::config::JsonRpcConfig;
///
/// // Create a test configuration with minimal sanitization
/// let test_config = JsonRpcConfig::builder()
///     // Disable sanitization for this test
///     .enable_sanitization(false)
///     .build();
///
/// assert!(!test_config.sanitization.enabled);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SanitizationConfig {
    /// Whether to enable message sanitization
    pub enabled: bool,

    /// List of sensitive terms to redact in error messages
    pub sensitive_terms: Vec<String>,

    /// Maximum message length before truncation
    pub max_message_length: usize,

    /// Redaction placeholder
    pub redaction_marker: String,

    /// Whether to sanitize file paths
    pub sanitize_paths: bool,
}

impl ConfigError {
    pub fn validation(msg: impl Into<String>) -> Self {
        ConfigError::Validation(msg.into())
    }
    pub fn security_violation(msg: impl Into<String>) -> Self {
        ConfigError::SecurityViolation(msg.into())
    }
}

impl JsonRpcConfigBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: JsonRpcConfig::default(),
        }
    }

    /// Set HTTP bind address
    pub fn http_bind_address(mut self, address: SocketAddr) -> Self {
        self.config.http.bind_address = address;
        self
    }

    /// Set the path to the TLS certificate file for the HTTP server
    pub fn http_cert(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.http.cert = Some(path.into());
        self
    }

    /// Set the path to the TLS private key file for the HTTP server
    pub fn http_key(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.http.key = Some(path.into());
        self
    }

    /// Set WebSocket bind address
    pub fn ws_bind_address(mut self, address: SocketAddr) -> Self {
        self.config.ws.bind_address = address;
        self
    }

    /// Enable or disable WebSocket support
    pub fn enable_websocket(mut self, enable: bool) -> Self {
        self.config.features.enable_websocket = enable;
        self
    }

    /// Set max block range for queries
    pub fn max_block_range(mut self, range: u64) -> Self {
        self.config.features.max_block_range = range;
        self
    }

    /// Enable or disable rate limiting
    pub fn enable_rate_limiting(mut self, enable: bool) -> Self {
        self.config.rate_limit.enabled = enable;
        self
    }

    /// Set default rate limit
    pub fn default_rate_limit(
        mut self,
        requests: u64,
        window_secs: u64,
    ) -> Self {
        self.config.rate_limit.default_limit = RateLimit {
            requests,
            window: Duration::from_secs(window_secs),
        };
        self
    }

    /// Add a method-specific rate limit
    pub fn add_method_rate_limit(
        mut self,
        pattern: &str,
        requests: u64,
        window_secs: u64,
    ) -> Self {
        self.config.rate_limit.method_limits.push(MethodRateLimit {
            method_pattern: pattern.to_string(),
            limit: RateLimit {
                requests,
                window: Duration::from_secs(window_secs),
            },
        });
        self
    }

    /// Enable or disable message sanitization
    pub fn enable_sanitization(mut self, enable: bool) -> Self {
        self.config.sanitization.enabled = enable;
        self
    }

    /// Set the custom redaction marker for sensitive information
    pub fn redaction_marker(mut self, marker: impl Into<String>) -> Self {
        self.config.sanitization.redaction_marker = marker.into();
        self
    }

    /// Set the maximum message length before truncation
    pub fn max_message_length(mut self, length: usize) -> Self {
        self.config.sanitization.max_message_length = length;
        self
    }

    /// Enable or disable path sanitization
    pub fn sanitize_paths(mut self, enable: bool) -> Self {
        self.config.sanitization.sanitize_paths = enable;
        self
    }

    /// Replace the entire list of sensitive terms
    pub fn sensitive_terms(mut self, terms: Vec<String>) -> Self {
        self.config.sanitization.sensitive_terms = terms;
        self
    }

    /// Add a single sensitive term to the list
    pub fn add_sensitive_term(mut self, term: impl Into<String>) -> Self {
        self.config.sanitization.sensitive_terms.push(term.into());
        self
    }

    /// Add multiple sensitive terms to the list
    pub fn add_sensitive_terms(mut self, terms: &[impl AsRef<str>]) -> Self {
        for term in terms {
            self.config
                .sanitization
                .sensitive_terms
                .push(term.as_ref().to_string());
        }
        self
    }

    /// Build the final configuration
    pub fn build(self) -> JsonRpcConfig {
        self.config
    }
}

impl Default for JsonRpcConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// Wrapper struct for deserializing the relevant part of default.config.toml.
// Needed because default.config.toml contains more than just [jsonrpc]
// settings.
#[derive(Deserialize, Default)]
struct RuskConfigFile {
    #[serde(default)]
    jsonrpc: JsonRpcConfig,
    // Capture other sections to ignore them during JSON-RPC config loading
    #[serde(flatten)]
    _other: HashMap<String, toml::Value>,
}

// Wrapper struct for serializing JsonRpcConfig under the [jsonrpc] key.
// Needed to ensure the `[jsonrpc]` table header is explicitly added during
// serialization, as `toml-rs` might otherwise flatten the output if this
// wrapper wasn't used.
#[derive(Serialize)]
struct JsonRpcConfigWrapper<'a> {
    // This field name 'jsonrpc' will become the key in the TOML output
    jsonrpc: &'a JsonRpcConfig,
}

impl JsonRpcConfig {
    /// Default configuration file name (used if RUSK_JSONRPC_CONFIG_PATH is not
    /// set)
    pub const DEFAULT_CONFIG_FILENAME: &'static str = "default.config.toml";

    /// Create a new builder for JsonRpcConfig
    pub fn builder() -> JsonRpcConfigBuilder {
        JsonRpcConfigBuilder::new()
    }

    /// Load configuration from the default path (`default.config.toml` in
    /// project root). Environment variables override file settings.
    pub fn load_default() -> Result<Self, ConfigError> {
        let default_path = Self::default_config_path();
        Self::load(Some(&default_path))
    }

    /// Load configuration with the following precedence:
    /// 1. Environment variables (flat structure, e.g.,
    ///    RUSK_JSONRPC_HTTP_BIND_ADDRESS)
    /// 2. Config file specified by RUSK_JSONRPC_CONFIG_PATH env var (expects
    ///    [jsonrpc] section)
    /// 3. Config file specified by `config_file` argument (expects [jsonrpc]
    ///    section)
    /// 4. Default values
    #[instrument(level = "info", name = "load_jsonrpc_config", skip_all, fields(path = ?config_file))]
    pub fn load(config_file: Option<&Path>) -> Result<Self, ConfigError> {
        info!("Starting JSON-RPC configuration loading process");

        let mut config = Self::default(); // Start with defaults
        info!("Initialized with default configuration values");

        // Determine which config file path to use (env var takes precedence)
        let path_to_load = env::var("RUSK_JSONRPC_CONFIG_PATH")
            .map(PathBuf::from)
            .ok()
            .filter(|p| p.exists())
            .or_else(|| {
                config_file.filter(|p| p.exists()).map(|p| p.to_path_buf())
            });

        // Load from the determined file path, if any
        if let Some(path) = path_to_load {
            info!(path = %path.display(), "Attempting to load configuration from file");
            let content = fs::read_to_string(&path).map_err(|e| {
                 error!(error = %e, path = %path.display(), "Failed to read configuration file");
                 ConfigError::FileRead(e)
            })?;
            info!(path = %path.display(), "Successfully read configuration file content");

            debug!("Parsing TOML configuration from file content");
            let file_wrapper: RuskConfigFile = toml::from_str(&content).map_err(|e| {
                error!(error = %e, path = %path.display(), "Failed to parse TOML configuration from file");
                ConfigError::TomlParse(e)
            })?;
            config = file_wrapper.jsonrpc; // Overwrite defaults with file content
            info!(path = %path.display(), "Successfully parsed TOML configuration from file, applying values");
        } else {
            info!("No configuration file specified or found, using defaults and environment variables");
        }

        // Apply flat environment variable overrides on top of defaults/file
        // config
        match config.apply_env_overrides() {
            Ok(_) => {
                info!("Successfully checked for environment variable overrides")
            }
            Err(e) => {
                error!(error = %e, "Error applying environment variable overrides");
                return Err(e); // Propagate error if override parsing fails
            }
        }

        // Log the final configuration at debug level (be cautious with
        // sensitive data if any) Note: Current config struct doesn't
        // hold secrets like API keys directly.
        debug!(final_config = ?config, "Final configuration after loading defaults, file, and env vars");

        // Validate final config
        match config.validate() {
            Ok(_) => {
                info!(
                    "JSON-RPC configuration loaded and validated successfully"
                );
                Ok(config)
            }
            Err(e) => {
                error!(error = %e, "Configuration validation failed");
                Err(e) // Return validation error
            }
        }
    }

    /// Applies environment variable overrides to the current configuration.
    /// Logs applied overrides.
    #[instrument(level = "info", name = "apply_env_overrides", skip(self))]
    fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        info!("Checking for environment variable overrides");

        // Helper to get env vars with prefix and log if found
        let get_env = |name: &str| -> Option<String> {
            let key = format!("RUSK_JSONRPC_{}", name);
            match std::env::var(&key) {
                Ok(value) => {
                    // Avoid logging potentially sensitive values directly in
                    // info logs Log the key and indicate
                    // that a value was found.
                    // Sensitive values like TLS key paths are handled
                    // specifically below.
                    if name.ends_with("_KEY")
                        || name.ends_with("_CERT")
                        || name.contains("SENSITIVE")
                        || name.contains("REDACTION")
                    {
                        debug!(env_var = %key, "Found environment variable override (value masked)");
                    } else {
                        debug!(env_var = %key, value = %value, "Found environment variable override");
                    }
                    Some(value)
                }
                Err(std::env::VarError::NotPresent) => None,
                Err(e) => {
                    // Log other errors (e.g., invalid UTF-8) but don't fail the
                    // whole process
                    warn!(env_var = %key, error = %e, "Error reading environment variable");
                    None
                }
            }
        };

        // --- HTTP config ---
        if let Some(addr_str) = get_env("HTTP_BIND_ADDRESS") {
            match addr_str.parse() {
                Ok(addr) => {
                    info!(config_key = "http.bind_address", value = %addr, "Applying override");
                    self.http.bind_address = addr;
                }
                Err(e) => {
                    warn!(key = "HTTP_BIND_ADDRESS", value = %addr_str, error = %e, "Failed to parse address override")
                }
            }
        }
        if let Some(size_str) = get_env("HTTP_MAX_BODY_SIZE") {
            match size_str.parse() {
                Ok(size) => {
                    self.http.max_body_size = size;
                    info!(
                        env_var = "RUSK_JSONRPC_HTTP_MAX_BODY_SIZE",
                        config_key = "http.max_body_size",
                        value = size,
                        "Applying override"
                    );
                }
                Err(e) => {
                    warn!(key = "HTTP_MAX_BODY_SIZE", value = %size_str, error = %e, "Failed to parse size override")
                }
            }
        }
        if let Some(timeout_str) = get_env("HTTP_REQUEST_TIMEOUT_SECS") {
            match timeout_str.parse() {
                Ok(secs) => {
                    self.http.request_timeout = Duration::from_secs(secs);
                    info!(
                        env_var = "RUSK_JSONRPC_HTTP_REQUEST_TIMEOUT_SECS",
                        config_key = "http.request_timeout",
                        value = secs,
                        "Applying override"
                    );
                }
                Err(e) => {
                    warn!(key = "HTTP_REQUEST_TIMEOUT_SECS", value = %timeout_str, error = %e, "Failed to parse timeout override")
                }
            }
        }
        if let Some(connections_str) = get_env("HTTP_MAX_CONNECTIONS") {
            match connections_str.parse() {
                Ok(conn) => {
                    self.http.max_connections = conn;
                    info!(
                        env_var = "RUSK_JSONRPC_HTTP_MAX_CONNECTIONS",
                        config_key = "http.max_connections",
                        value = conn,
                        "Applying override"
                    );
                }
                Err(e) => {
                    warn!(key = "HTTP_MAX_CONNECTIONS", value = %connections_str, error = %e, "Failed to parse connections override")
                }
            }
        }
        if let Some(cert_path_str) = get_env("HTTP_CERT") {
            info!(config_key = "http.cert", value = %cert_path_str, "Applying override");
            self.http.cert = Some(PathBuf::from(cert_path_str));
        }
        if let Some(key_path_str) = get_env("HTTP_KEY") {
            info!(
                config_key = "http.key",
                value = "<path>",
                "Applying override"
            ); // Mask path
            self.http.key = Some(PathBuf::from(key_path_str));
        }

        // --- CORS config ---
        if let Some(enabled_str) = get_env("CORS_ENABLED") {
            match parse_bool_env(&enabled_str) {
                Some(enabled) => {
                    info!(
                        config_key = "http.cors.enabled",
                        value = enabled,
                        "Applying override"
                    );
                    self.http.cors.enabled = enabled;
                }
                None => {
                    warn!(key = "CORS_ENABLED", value = %enabled_str, "Failed to parse boolean override, skipping")
                }
            }
        }
        if let Some(origins_str) = get_env("CORS_ALLOWED_ORIGINS") {
            let origins = parse_env_string_array(&origins_str);
            info!(config_key = "http.cors.allowed_origins", value = ?origins, "Applying override");
            self.http.cors.allowed_origins = origins;
        }
        if let Some(methods_str) = get_env("CORS_ALLOWED_METHODS") {
            let methods = parse_env_string_array(&methods_str);
            info!(config_key = "http.cors.allowed_methods", value = ?methods, "Applying override");
            self.http.cors.allowed_methods = methods;
        }
        if let Some(headers_str) = get_env("CORS_ALLOWED_HEADERS") {
            let headers = parse_env_string_array(&headers_str);
            info!(config_key = "http.cors.allowed_headers", value = ?headers, "Applying override");
            self.http.cors.allowed_headers = headers;
        }
        if let Some(credentials_str) = get_env("CORS_ALLOW_CREDENTIALS") {
            match parse_bool_env(&credentials_str) {
                Some(credentials) => {
                    info!(
                        config_key = "http.cors.allow_credentials",
                        value = credentials,
                        "Applying override"
                    );
                    self.http.cors.allow_credentials = credentials;
                }
                None => {
                    warn!(key = "CORS_ALLOW_CREDENTIALS", value = %credentials_str, "Failed to parse boolean override, skipping")
                }
            }
        }
        if let Some(max_age_str) = get_env("CORS_MAX_AGE_SECONDS") {
            match max_age_str.parse() {
                Ok(age) => {
                    self.http.cors.max_age_seconds = age;
                    info!(
                        env_var = "RUSK_JSONRPC_CORS_MAX_AGE_SECONDS",
                        config_key = "http.cors.max_age_seconds",
                        value = age,
                        "Applying override"
                    );
                }
                Err(e) => {
                    warn!(key = "CORS_MAX_AGE_SECONDS", value = %max_age_str, error = %e, "Failed to parse max_age override")
                }
            }
        }

        // --- WebSocket config ---
        if let Some(addr_str) = get_env("WS_BIND_ADDRESS") {
            match addr_str.parse() {
                Ok(addr) => {
                    info!(config_key = "ws.bind_address", value = %addr, "Applying override");
                    self.ws.bind_address = addr;
                }
                Err(e) => {
                    warn!(key = "WS_BIND_ADDRESS", value = %addr_str, error = %e, "Failed to parse address override")
                }
            }
        }
        if let Some(size_str) = get_env("WS_MAX_MESSAGE_SIZE") {
            match size_str.parse() {
                Ok(size) => {
                    self.ws.max_message_size = size;
                    info!(
                        env_var = "RUSK_JSONRPC_WS_MAX_MESSAGE_SIZE",
                        config_key = "ws.max_message_size",
                        value = size,
                        "Applying override"
                    );
                }
                Err(e) => {
                    warn!(key = "WS_MAX_MESSAGE_SIZE", value = %size_str, error = %e, "Failed to parse size override")
                }
            }
        }
        if let Some(connections_str) = get_env("WS_MAX_CONNECTIONS") {
            match connections_str.parse() {
                Ok(conn) => {
                    info!(
                        config_key = "ws.max_connections",
                        value = conn,
                        "Applying override"
                    );
                    self.ws.max_connections = conn;
                }
                Err(e) => {
                    warn!(key = "WS_MAX_CONNECTIONS", value = %connections_str, error = %e, "Failed to parse connections override")
                }
            }
        }
        if let Some(subscriptions_str) =
            get_env("WS_MAX_SUBSCRIPTIONS_PER_CONNECTION")
        {
            match subscriptions_str.parse() {
                Ok(subs) => {
                    self.ws.max_subscriptions_per_connection = subs;
                    info!(
                        env_var =
                            "RUSK_JSONRPC_WS_MAX_SUBSCRIPTIONS_PER_CONNECTION",
                        config_key = "ws.max_subscriptions_per_connection",
                        value = subs,
                        "Applying override"
                    );
                }
                Err(e) => {
                    warn!(key = "WS_MAX_SUBSCRIPTIONS_PER_CONNECTION", value = %subscriptions_str, error = %e, "Failed to parse subscriptions override")
                }
            }
        }
        if let Some(timeout_str) = get_env("WS_IDLE_TIMEOUT_SECS") {
            match timeout_str.parse() {
                Ok(secs) => {
                    self.ws.idle_timeout = Duration::from_secs(secs);
                    info!(
                        env_var = "RUSK_JSONRPC_WS_IDLE_TIMEOUT_SECS",
                        config_key = "ws.idle_timeout",
                        value = secs,
                        "Applying override"
                    );
                }
                Err(e) => {
                    warn!(key = "WS_IDLE_TIMEOUT_SECS", value = %timeout_str, error = %e, "Failed to parse timeout override")
                }
            }
        }
        if let Some(events_str) = get_env("WS_MAX_EVENTS_PER_SECOND") {
            match events_str.parse() {
                Ok(count) => {
                    self.ws.max_events_per_second = count;
                    info!(
                        env_var = "RUSK_JSONRPC_WS_MAX_EVENTS_PER_SECOND",
                        config_key = "ws.max_events_per_second",
                        value = count,
                        "Applying override"
                    );
                }
                Err(e) => {
                    warn!(key = "WS_MAX_EVENTS_PER_SECOND", value = %events_str, error = %e, "Failed to parse events override")
                }
            }
        }

        // --- Rate limiting ---
        if let Some(enabled_str) = get_env("RATE_LIMIT_ENABLED") {
            match parse_bool_env(&enabled_str) {
                Some(enabled) => {
                    info!(
                        config_key = "rate_limit.enabled",
                        value = enabled,
                        "Applying override"
                    );
                    self.rate_limit.enabled = enabled;
                }
                None => {
                    warn!(key = "RATE_LIMIT_ENABLED", value = %enabled_str, "Failed to parse boolean override, skipping")
                }
            }
        }
        if let Some(requests_str) = get_env("RATE_LIMIT_DEFAULT_REQUESTS") {
            match requests_str.parse() {
                Ok(req) => {
                    info!(
                        config_key = "rate_limit.default_limit.requests",
                        value = req,
                        "Applying override"
                    );
                    self.rate_limit.default_limit.requests = req;
                }
                Err(e) => {
                    warn!(key = "RATE_LIMIT_DEFAULT_REQUESTS", value = %requests_str, error = %e, "Failed to parse requests override")
                }
            }
        }
        if let Some(window_str) = get_env("RATE_LIMIT_DEFAULT_WINDOW_SECS") {
            match window_str.parse() {
                Ok(secs) => {
                    self.rate_limit.default_limit.window =
                        Duration::from_secs(secs);
                    info!(
                        env_var = "RUSK_JSONRPC_RATE_LIMIT_DEFAULT_WINDOW_SECS",
                        config_key = "rate_limit.default_limit.window",
                        value = secs,
                        "Applying override"
                    );
                }
                Err(e) => {
                    warn!(key = "RATE_LIMIT_DEFAULT_WINDOW_SECS", value = %window_str, error = %e, "Failed to parse window override")
                }
            }
        }
        // Note: Method-specific rate limits are not easily overridden via flat
        // env vars. They should primarily be configured via the config
        // file. Log a warning if the env var for method limits is
        // detected, as it's not supported.
        if get_env("RATE_LIMIT_METHOD_LIMITS").is_some() {
            warn!("Environment variable RUSK_JSONRPC_RATE_LIMIT_METHOD_LIMITS is not supported for overriding method-specific rate limits. Please configure these in the TOML file.");
        }

        // --- Features ---
        if let Some(enabled_str) = get_env("FEATURE_ENABLE_WEBSOCKET") {
            match parse_bool_env(&enabled_str) {
                Some(enabled) => {
                    info!(
                        config_key = "features.enable_websocket",
                        value = enabled,
                        "Applying override"
                    );
                    self.features.enable_websocket = enabled;
                }
                None => {
                    warn!(key = "FEATURE_ENABLE_WEBSOCKET", value = %enabled_str, "Failed to parse boolean override, skipping")
                }
            }
        }
        if let Some(enabled_str) = get_env("FEATURE_DETAILED_ERRORS") {
            match parse_bool_env(&enabled_str) {
                Some(enabled) => {
                    info!(
                        config_key = "features.detailed_errors",
                        value = enabled,
                        "Applying override"
                    );
                    self.features.detailed_errors = enabled;
                }
                None => {
                    warn!(key = "FEATURE_DETAILED_ERRORS", value = %enabled_str, "Failed to parse boolean override, skipping")
                }
            }
        }
        if let Some(enabled_str) = get_env("FEATURE_METHOD_TIMING") {
            match parse_bool_env(&enabled_str) {
                Some(enabled) => {
                    info!(
                        config_key = "features.method_timing",
                        value = enabled,
                        "Applying override"
                    );
                    self.features.method_timing = enabled;
                }
                None => {
                    warn!(key = "FEATURE_METHOD_TIMING", value = %enabled_str, "Failed to parse boolean override, skipping")
                }
            }
        }
        if let Some(enabled_str) = get_env("FEATURE_STRICT_VERSION_CHECKING") {
            match parse_bool_env(&enabled_str) {
                Some(enabled) => {
                    info!(
                        config_key = "features.strict_version_checking",
                        value = enabled,
                        "Applying override"
                    );
                    self.features.strict_version_checking = enabled;
                }
                None => {
                    warn!(key = "FEATURE_STRICT_VERSION_CHECKING", value = %enabled_str, "Failed to parse boolean override, skipping")
                }
            }
        }
        if let Some(enabled_str) =
            get_env("FEATURE_STRICT_PARAMETER_VALIDATION")
        {
            match parse_bool_env(&enabled_str) {
                Some(enabled) => {
                    info!(
                        config_key = "features.strict_parameter_validation",
                        value = enabled,
                        "Applying override"
                    );
                    self.features.strict_parameter_validation = enabled;
                }
                None => {
                    warn!(key = "FEATURE_STRICT_PARAMETER_VALIDATION", value = %enabled_str, "Failed to parse boolean override, skipping")
                }
            }
        }
        if let Some(range_str) = get_env("FEATURE_MAX_BLOCK_RANGE") {
            match range_str.parse() {
                Ok(r) => {
                    self.features.max_block_range = r;
                    info!(
                        env_var = "RUSK_JSONRPC_FEATURE_MAX_BLOCK_RANGE",
                        config_key = "features.max_block_range",
                        value = r,
                        "Applying override"
                    );
                }
                Err(e) => {
                    warn!(key = "FEATURE_MAX_BLOCK_RANGE", value = %range_str, error = %e, "Failed to parse range override")
                }
            }
        }
        if let Some(size_str) = get_env("FEATURE_MAX_BATCH_SIZE") {
            match size_str.parse() {
                Ok(s) => {
                    self.features.max_batch_size = s;
                    info!(
                        env_var = "RUSK_JSONRPC_FEATURE_MAX_BATCH_SIZE",
                        config_key = "features.max_batch_size",
                        value = s,
                        "Applying override"
                    );
                }
                Err(e) => {
                    warn!(key = "FEATURE_MAX_BATCH_SIZE", value = %size_str, error = %e, "Failed to parse size override")
                }
            }
        }

        // --- Sanitization config ---
        if let Some(enabled_str) = get_env("SANITIZATION_ENABLED") {
            match parse_bool_env(&enabled_str) {
                Some(enabled) => {
                    info!(
                        config_key = "sanitization.enabled",
                        value = enabled,
                        "Applying override"
                    );
                    self.sanitization.enabled = enabled;
                }
                None => {
                    warn!(key = "SANITIZATION_ENABLED", value = %enabled_str, "Failed to parse boolean override, skipping")
                }
            }
        }
        if let Some(max_length_str) = get_env("SANITIZATION_MAX_MESSAGE_LENGTH")
        {
            match max_length_str.parse() {
                Ok(length) => {
                    self.sanitization.max_message_length = length;
                    info!(
                        env_var =
                            "RUSK_JSONRPC_SANITIZATION_MAX_MESSAGE_LENGTH",
                        config_key = "sanitization.max_message_length",
                        value = length,
                        "Applying override"
                    );
                }
                Err(e) => {
                    warn!(key = "SANITIZATION_MAX_MESSAGE_LENGTH", value = %max_length_str, error = %e, "Failed to parse length override")
                }
            }
        }
        if let Some(marker) = get_env("SANITIZATION_REDACTION_MARKER") {
            info!(config_key = "sanitization.redaction_marker", value = %marker, "Applying override");
            self.sanitization.redaction_marker = marker;
        }
        if let Some(sanitize_paths_str) = get_env("SANITIZATION_SANITIZE_PATHS")
        {
            match parse_bool_env(&sanitize_paths_str) {
                Some(sanitize_paths) => {
                    info!(
                        config_key = "sanitization.sanitize_paths",
                        value = sanitize_paths,
                        "Applying override"
                    );
                    self.sanitization.sanitize_paths = sanitize_paths;
                }
                None => {
                    warn!(key = "SANITIZATION_SANITIZE_PATHS", value = %sanitize_paths_str, "Failed to parse boolean override, skipping")
                }
            }
        }
        if let Some(terms_str) = get_env("SANITIZATION_SENSITIVE_TERMS") {
            let terms = parse_env_string_array(&terms_str);
            info!(
                config_key = "sanitization.sensitive_terms",
                value = "<masked>",
                "Applying override"
            ); // Mask terms
            self.sanitization.sensitive_terms = terms;
        }

        info!("Finished checking for environment variable overrides");
        Ok(())
    }

    /// Load configuration ONLY from the specified file, falling back to
    /// defaults. Does NOT apply environment variable overrides. Intended
    /// for testing.
    // #[cfg(test)] // Removed cfg attribute to make it available for
    // integration tests
    pub fn load_from_file_only(
        config_file: Option<&Path>,
    ) -> Result<Self, ConfigError> {
        let mut config = Self::default();

        let path_to_load = config_file.filter(|p| p.exists());

        if let Some(path) = path_to_load {
            let content = fs::read_to_string(path)?;
            let file_wrapper: RuskConfigFile = toml::from_str(&content)?;
            config = file_wrapper.jsonrpc;
        }

        config.validate()?;
        Ok(config)
    }

    /// Create a configuration for testing (uses default values, disables rate
    /// limiting)
    pub fn test_config() -> Self {
        let mut config = Self::default();
        config.http.bind_address =
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        config.ws.bind_address =
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        config.rate_limit.enabled = false;
        config
    }

    /// Export the current configuration to a TOML string, nested under
    /// `[jsonrpc]`.
    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        // Ensure the wrapper is created and passed to toml::to_string_pretty
        let wrapper = JsonRpcConfigWrapper { jsonrpc: self };
        toml::to_string_pretty(&wrapper)
    }

    /// Export the current configuration (nested under `[jsonrpc]`) to a file.
    /// Note: This will overwrite the file with *only* the `[jsonrpc]` section.
    /// It does not preserve other sections from the original file.
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let toml_string = self.to_toml_string()?;
        fs::write(path, toml_string)?;
        Ok(())
    }

    /// Validates configuration values with additional security checks. Logs
    /// results.
    #[instrument(level = "info", name = "validate_jsonrpc_config", skip(self))]
    pub fn validate(&self) -> Result<(), ConfigError> {
        info!("Starting configuration validation");

        // Validate HTTP configuration
        if self.http.max_body_size == 0 {
            error!(
                "Validation failed: http.max_body_size must be greater than 0"
            );
            return Err(ConfigError::validation(
                "http.max_body_size must be greater than 0",
            ));
        }
        if self.http.max_connections == 0 {
            error!("Validation failed: http.max_connections must be greater than 0");
            return Err(ConfigError::validation(
                "http.max_connections must be greater than 0",
            ));
        }
        if self.http.request_timeout.as_secs() == 0 {
            error!("Validation failed: http.request_timeout must be greater than 0");
            return Err(ConfigError::validation(
                "http.request_timeout must be greater than 0",
            ));
        }

        // Validate TLS configuration (if provided)
        match (&self.http.cert, &self.http.key) {
            (Some(cert_path), Some(key_path)) => {
                debug!(cert_path = %cert_path.display(), key_path = %key_path.display(), "Validating TLS configuration");
                if !cert_path.exists() {
                    let err_msg = format!(
                        "TLS certificate file not found: {}",
                        cert_path.display()
                    );
                    error!(error = %err_msg, "Configuration validation failed");
                    return Err(ConfigError::validation(err_msg));
                }
                if !key_path.exists() {
                    let err_msg = format!(
                        "TLS key file not found: {}",
                        key_path.display()
                    );
                    error!(error = %err_msg, "Configuration validation failed");
                    return Err(ConfigError::validation(err_msg));
                }
                info!("TLS configuration files found and validated");
            }
            (Some(_), None) => {
                let err_msg = "TLS certificate provided but key is missing";
                error!(error = %err_msg, "Configuration validation failed");
                return Err(ConfigError::validation(err_msg));
            }
            (None, Some(_)) => {
                let err_msg = "TLS key provided but certificate is missing";
                error!(error = %err_msg, "Configuration validation failed");
                return Err(ConfigError::validation(err_msg));
            }
            (None, None) => {
                debug!("TLS not configured");
            }
        }

        // Validate WebSocket configuration
        if self.ws.max_message_size == 0 {
            error!(
                "Validation failed: ws.max_message_size must be greater than 0"
            );
            return Err(ConfigError::validation(
                "ws.max_message_size must be greater than 0",
            ));
        }
        if self.ws.max_connections == 0 {
            error!(
                "Validation failed: ws.max_connections must be greater than 0"
            );
            return Err(ConfigError::validation(
                "ws.max_connections must be greater than 0",
            ));
        }
        if self.ws.max_subscriptions_per_connection == 0 {
            error!("Validation failed: ws.max_subscriptions_per_connection must be greater than 0");
            return Err(ConfigError::validation(
                "ws.max_subscriptions_per_connection must be greater than 0",
            ));
        }
        if self.ws.idle_timeout.as_secs() == 0 {
            error!("Validation failed: ws.idle_timeout must be greater than 0");
            return Err(ConfigError::validation(
                "ws.idle_timeout must be greater than 0",
            ));
        }
        if self.ws.max_events_per_second == 0 {
            error!("Validation failed: ws.max_events_per_second must be greater than 0");
            return Err(ConfigError::validation(
                "ws.max_events_per_second must be greater than 0",
            ));
        }

        // Validate rate limit configuration
        if self.rate_limit.enabled {
            debug!("Validating rate limit configuration (enabled)");
            if self.rate_limit.default_limit.requests == 0 {
                let err_msg =
                    "rate_limit.default_limit.requests must be greater than 0";
                error!(error = %err_msg, "Configuration validation failed");
                return Err(ConfigError::validation(err_msg));
            }
            if self.rate_limit.default_limit.window.as_secs() == 0 {
                let err_msg =
                    "rate_limit.default_limit.window must be greater than 0";
                error!(error = %err_msg, "Configuration validation failed");
                return Err(ConfigError::validation(err_msg));
            }
            if self.rate_limit.websocket_limit.requests == 0 {
                let err_msg = "rate_limit.websocket_limit.requests must be greater than 0";
                error!(error = %err_msg, "Configuration validation failed");
                return Err(ConfigError::validation(err_msg));
            }
            if self.rate_limit.websocket_limit.window.as_secs() == 0 {
                let err_msg =
                    "rate_limit.websocket_limit.window must be greater than 0";
                error!(error = %err_msg, "Configuration validation failed");
                return Err(ConfigError::validation(err_msg));
            }
            for (i, method_limit) in
                self.rate_limit.method_limits.iter().enumerate()
            {
                if method_limit.method_pattern.is_empty() {
                    let err_msg = format!("rate_limit.method_limits[{}].method_pattern must not be empty", i);
                    error!(error = %err_msg, "Configuration validation failed");
                    return Err(ConfigError::validation(err_msg));
                }
                if method_limit.limit.requests == 0 {
                    let err_msg = format!("rate_limit.method_limits[{}].limit.requests must be greater than 0", i);
                    error!(error = %err_msg, "Configuration validation failed");
                    return Err(ConfigError::validation(err_msg));
                }
                if method_limit.limit.window.as_secs() == 0 {
                    let err_msg = format!("rate_limit.method_limits[{}].limit.window must be greater than 0", i);
                    error!(error = %err_msg, "Configuration validation failed");
                    return Err(ConfigError::validation(err_msg));
                }
            }
            debug!("Rate limit configuration validated");
        } else {
            debug!("Rate limiting is disabled");
        }

        // Validate feature configuration
        if self.features.max_block_range == 0 {
            error!("Validation failed: features.max_block_range must be greater than 0");
            return Err(ConfigError::validation(
                "features.max_block_range must be greater than 0",
            ));
        }
        if self.features.max_batch_size == 0 {
            error!("Validation failed: features.max_batch_size must be greater than 0");
            return Err(ConfigError::validation(
                "features.max_batch_size must be greater than 0",
            ));
        }

        // Validate sanitization configuration
        if self.sanitization.max_message_length == 0 {
            error!("Validation failed: sanitization.max_message_length must be greater than 0");
            return Err(ConfigError::validation(
                "sanitization.max_message_length must be greater than 0",
            ));
        }
        if self.sanitization.redaction_marker.is_empty() {
            error!("Validation failed: sanitization.redaction_marker must not be empty");
            return Err(ConfigError::validation(
                "sanitization.redaction_marker must not be empty",
            ));
        }

        // --- Security validation checks ---
        debug!("Performing security validation checks");

        // 1. Check for insecure bind addresses
        if !self.is_rate_limiting_enabled() && self.is_binding_publicly() {
            let err_msg =
                "Binding to public interface without rate limiting is insecure";
            error!(error = %err_msg, "Configuration security validation failed");
            return Err(ConfigError::security_violation(err_msg));
        }

        // 2. Check CORS configuration
        if self.is_cors_insecure() {
            let err_msg = "Allowing wildcard CORS origin (*) with credentials is insecure";
            error!(error = %err_msg, "Configuration security validation failed");
            return Err(ConfigError::security_violation(err_msg));
        }

        // 3. Check request size limits
        const MAX_SAFE_REQUEST_SIZE: usize = 100 * 1024 * 1024; // 100 MB
        if self.http.max_body_size > MAX_SAFE_REQUEST_SIZE {
            let err_msg = format!(
                "Request body size limit ({}) exceeds recommended maximum ({})",
                self.http.max_body_size, MAX_SAFE_REQUEST_SIZE
            );
            error!(error = %err_msg, "Configuration security validation failed");
            return Err(ConfigError::security_violation(err_msg));
        }

        // 4. Check WebSocket message size limit
        const MAX_SAFE_WS_MESSAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MB
        if self.ws.max_message_size > MAX_SAFE_WS_MESSAGE_SIZE {
            let err_msg = format!(
                "WebSocket message size limit ({}) exceeds recommended maximum ({})",
                self.ws.max_message_size, MAX_SAFE_WS_MESSAGE_SIZE
            );
            error!(error = %err_msg, "Configuration security validation failed");
            return Err(ConfigError::security_violation(err_msg));
        }

        // 5. Check strict parameter validation
        if !self.features.strict_parameter_validation {
            let err_msg = "Disabling strict parameter validation is not recommended for production";
            error!(error = %err_msg, "Configuration security validation failed");
            return Err(ConfigError::security_violation(err_msg));
        }

        // 6. Check rate limiting settings (if enabled)
        if self.rate_limit.enabled {
            const MAX_SAFE_DEFAULT_RATE: u64 = 1000; // per minute
            if self.rate_limit.default_limit.requests > MAX_SAFE_DEFAULT_RATE
                && self.rate_limit.default_limit.window.as_secs() <= 60
            {
                let err_msg = format!(
                    "Default rate limit ({}/min) exceeds recommended maximum ({}/min)",
                    self.rate_limit.default_limit.requests, MAX_SAFE_DEFAULT_RATE
                );
                error!(error = %err_msg, "Configuration security validation failed");
                return Err(ConfigError::security_violation(err_msg));
            }

            for method_limit in &self.rate_limit.method_limits {
                const UNLIMITED_THRESHOLD: u64 = 10000; // per minute
                let requests_per_minute =
                    if method_limit.limit.window.as_secs() == 0 {
                        u64::MAX
                    } else {
                        method_limit.limit.requests * 60
                            / method_limit.limit.window.as_secs()
                    };

                if requests_per_minute > UNLIMITED_THRESHOLD {
                    let err_msg = format!(
                        "Method '{}' has an extremely high rate limit (~{}/min)",
                        method_limit.method_pattern, requests_per_minute
                    );
                    error!(error = %err_msg, "Configuration security validation failed");
                    return Err(ConfigError::security_violation(err_msg));
                }
            }
        }

        // 7. Check block query limits
        const MAX_SAFE_BLOCK_RANGE: u64 = 10000;
        if self.features.max_block_range > MAX_SAFE_BLOCK_RANGE {
            let err_msg = format!(
                "Block range limit ({}) exceeds recommended maximum ({})",
                self.features.max_block_range, MAX_SAFE_BLOCK_RANGE
            );
            error!(error = %err_msg, "Configuration security validation failed");
            return Err(ConfigError::security_violation(err_msg));
        }

        // 8. Validate sanitization settings for security
        if self.is_sanitization_disabled_on_public() {
            let err_msg = "Disabling error message sanitization while binding to public interfaces is a security risk";
            error!(error = %err_msg, "Configuration security validation failed");
            return Err(ConfigError::security_violation(err_msg));
        }

        const MINIMUM_SENSITIVE_TERMS: usize = 5;
        if self.sanitization.enabled
            && self.sanitization.sensitive_terms.len() < MINIMUM_SENSITIVE_TERMS
        {
            let err_msg = format!(
                "Fewer than {} sensitive terms defined for sanitization. This may leave sensitive data exposed.",
                MINIMUM_SENSITIVE_TERMS
            );
            error!(error = %err_msg, "Configuration security validation failed");
            return Err(ConfigError::security_violation(err_msg));
        }

        if !self.sanitization.sanitize_paths && self.is_binding_publicly() {
            let err_msg = "Path sanitization must be enabled when binding to public interfaces";
            error!(error = %err_msg, "Configuration security validation failed");
            return Err(ConfigError::security_violation(err_msg));
        }

        info!(
            "Configuration validation and security checks passed successfully"
        );
        Ok(())
    }

    /// Get the project root directory (where Cargo.toml is located)
    pub fn project_root() -> PathBuf {
        env::var("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                // Fall back to the directory detection logic if
                // CARGO_MANIFEST_DIR is not set
                // (This may happen when run outside of Cargo)
                let mut path =
                    env::current_exe().unwrap_or_else(|_| PathBuf::from("."));

                while path.parent().is_some()
                    && !path.join("Cargo.toml").exists()
                {
                    path = path.parent().unwrap().to_path_buf();
                }

                if !path.join("Cargo.toml").exists() {
                    // Try current working directory
                    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                } else {
                    path
                }
            })
    }

    /// Get the default config file path
    pub fn default_config_path() -> PathBuf {
        Self::project_root().join(Self::DEFAULT_CONFIG_FILENAME)
    }

    // --- Security Helper Methods ---

    /// Checks if either the HTTP or WebSocket server is configured to bind to a
    /// public interface.
    pub fn is_binding_publicly(&self) -> bool {
        is_public_interface(&self.http.bind_address)
            || (self.features.enable_websocket
                && is_public_interface(&self.ws.bind_address))
    }

    /// Checks if rate limiting is enabled in the configuration.
    pub fn is_rate_limiting_enabled(&self) -> bool {
        self.rate_limit.enabled
    }

    /// Checks if the CORS configuration is potentially insecure (wildcard
    /// origin with credentials).
    pub fn is_cors_insecure(&self) -> bool {
        if !self.http.cors.enabled {
            return false;
        }
        let has_wildcard_origin = self.http.cors.allowed_origins.is_empty()
            || self
                .http
                .cors
                .allowed_origins
                .iter()
                .any(|origin| origin == "*");
        has_wildcard_origin && self.http.cors.allow_credentials
    }

    /// Checks if message sanitization is disabled while binding to a public
    /// interface.
    pub fn is_sanitization_disabled_on_public(&self) -> bool {
        !self.sanitization.enabled && self.is_binding_publicly()
    }

    // --- End Security Helper Methods ---

    /// Builds a `tower_http::cors::CorsLayer` based on the configuration.
    ///
    /// Returns `None` if CORS is disabled in the configuration.
    /// Logs warnings if invalid origins, methods, or headers are encountered.
    pub fn build_cors_layer(&self) -> Option<CorsLayer> {
        if !self.http.cors.enabled {
            return None;
        }

        let mut cors = CorsLayer::new();

        // Configure origins
        if self.http.cors.allowed_origins.is_empty()
            || self.http.cors.allowed_origins.iter().any(|o| o == "*")
        {
            cors = cors.allow_origin(AllowOrigin::any());
        } else {
            let origins = self
                .http
                .cors
                .allowed_origins
                .iter()
                .filter_map(|origin_str| {
                    match origin_str.parse::<http::HeaderValue>() {
                        Ok(val) => Some(val),
                        Err(e) => {
                            warn!(origin = %origin_str, error = %e, "Failed to parse allowed CORS origin, skipping");
                            None
                        }
                    }
                })
                .collect::<Vec<_>>();
            if !origins.is_empty() {
                cors = cors.allow_origin(origins);
            } else {
                warn!(
                    "No valid CORS allowed_origins parsed, defaulting to none."
                );
                cors = cors.allow_origin(AllowOrigin::mirror_request());
            }
        }

        // Configure methods
        let methods = self
            .http
            .cors
            .allowed_methods
            .iter()
            .filter_map(|m| match Method::from_str(m) {
                Ok(method) => Some(method),
                Err(e) => {
                    warn!(method = %m, error = %e, "Failed to parse allowed CORS method, skipping");
                    None
                }
            })
            .collect::<Vec<_>>();
        if !methods.is_empty() {
            cors = cors.allow_methods(methods);
        }

        // Configure headers
        let headers = self
            .http
            .cors
            .allowed_headers
            .iter()
            .filter_map(|h| match HeaderName::from_str(h) {
                Ok(header) => Some(header),
                Err(e) => {
                    warn!(header = %h, error = %e, "Failed to parse allowed CORS header, skipping");
                    None
                }
            })
            .collect::<Vec<_>>();
        if !headers.is_empty() {
            cors = cors.allow_headers(headers);
        }

        // Configure credentials
        cors = cors.allow_credentials(self.http.cors.allow_credentials);
        cors =
            cors.max_age(Duration::from_secs(self.http.cors.max_age_seconds));

        Some(cors)
    }
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            bind_address: default_http_address(),
            max_body_size: 10 * 1024 * 1024, // 10 MB
            request_timeout: default_http_timeout(),
            max_connections: 100,
            cors: CorsConfig::default(),
            cert: None,
            key: None,
        }
    }
}

impl Default for WebSocketServerConfig {
    fn default() -> Self {
        Self {
            bind_address: default_ws_address(),
            max_message_size: 1024 * 1024,
            max_connections: 50,
            max_subscriptions_per_connection: 10,
            idle_timeout: default_idle_timeout(),
            max_events_per_second: 100,
        }
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_origins: Vec::new(),
            allowed_methods: vec!["POST".to_string(), "GET".to_string()],
            allowed_headers: vec![
                "Content-Type".to_string(),
                "Rusk-Version".to_string(),
            ],
            allow_credentials: false,
            max_age_seconds: 86400,
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_limit: RateLimit {
                requests: 100,
                window: default_rate_limit_window(),
            },
            method_limits: vec![
                // Higher limit for read-only methods
                MethodRateLimit {
                    method_pattern: "get*".to_string(),
                    limit: RateLimit {
                        requests: 200,
                        window: default_rate_limit_window(),
                    },
                },
                // Lower limit for resource-intensive methods
                MethodRateLimit {
                    method_pattern: "prove".to_string(),
                    limit: RateLimit {
                        requests: 10,
                        window: default_rate_limit_window(),
                    },
                },
            ],
            websocket_limit: RateLimit {
                requests: 10,
                window: default_rate_limit_window(),
            },
        }
    }
}

impl Default for RateLimit {
    fn default() -> Self {
        Self {
            requests: 100,
            window: default_rate_limit_window(),
        }
    }
}

impl Default for MethodRateLimit {
    fn default() -> Self {
        Self {
            method_pattern: "".to_string(),
            limit: RateLimit::default(),
        }
    }
}

impl Default for FeatureToggles {
    fn default() -> Self {
        Self {
            enable_websocket: true,
            detailed_errors: true,
            method_timing: true,
            strict_version_checking: false,
            strict_parameter_validation: true,
            max_block_range: 1000,
            max_batch_size: 20,
        }
    }
}

impl Default for SanitizationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sensitive_terms: vec![
                "password".to_string(),
                ".wallet".to_string(),
                ".key".to_string(),
                "keys".to_string(),
                "secret".to_string(),
                "private".to_string(),
                "credential".to_string(),
                "token".to_string(),
                "api_key".to_string(),
                "apikey".to_string(),
                "auth".to_string(),
                "passphrase".to_string(),
                "cert".to_string(),
                "certificate".to_string(),
                "mnemonic".to_string(),
                "seed".to_string(),
                "wallet".to_string(),
                "pk".to_string(),
                "sk".to_string(),
                "signing_key".to_string(),
                "encryption_key".to_string(),
            ],
            max_message_length: 200,
            redaction_marker: "[REDACTED]".to_string(),
            sanitize_paths: true,
        }
    }
}

// Custom serialization for SocketAddr
mod socket_addr_serde {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::net::SocketAddr;

    pub fn serialize<S>(
        addr: &SocketAddr,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&addr.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SocketAddr, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<SocketAddr>().map_err(serde::de::Error::custom)
    }
}

// Custom serialization for Duration
mod duration_serde {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(
        duration: &Duration,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

// Default functions
fn default_http_address() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8546)
}

fn default_ws_address() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8547)
}

fn default_http_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_idle_timeout() -> Duration {
    Duration::from_secs(300) // 5 minutes
}

fn default_rate_limit_window() -> Duration {
    Duration::from_secs(60)
}

/// Helper function to parse comma-separated environment variable into a string
/// vector
fn parse_env_string_array(value: &str) -> Vec<String> {
    if value.trim().is_empty() {
        return Vec::new();
    }

    value
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

// Helper function to determine if an address is bound to a public interface
fn is_public_interface(addr: &SocketAddr) -> bool {
    match addr.ip() {
        IpAddr::V4(ip) => {
            !(ip.is_loopback() || ip.is_private() || ip.is_link_local())
        }
        IpAddr::V6(ip) => {
            let segments = ip.segments();

            // ::1 loopback
            if segments == [0, 0, 0, 0, 0, 0, 0, 1] {
                return false;
            }

            // fe80::/10 link-local unicast
            if (segments[0] & 0xffc0) == 0xfe80 {
                return false;
            }

            // fc00::/7 unique local address
            if (segments[0] & 0xfe00) == 0xfc00 {
                return false;
            }

            // ::ffff:0:0/96 IPv4-mapped addresses (should be handled like IPv4)
            if segments[0] == 0
                && segments[1] == 0
                && segments[2] == 0
                && segments[3] == 0
                && segments[4] == 0
                && segments[5] == 0xffff
            {
                let v4_addr = Ipv4Addr::new(
                    (segments[6] >> 8) as u8,
                    (segments[6] & 0xff) as u8,
                    (segments[7] >> 8) as u8,
                    (segments[7] & 0xff) as u8,
                );
                return !(v4_addr.is_loopback()
                    || v4_addr.is_private()
                    || v4_addr.is_link_local());
            }

            // Consider all other IPv6 addresses public
            true
        }
    }
}
