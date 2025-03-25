// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # JSON-RPC Server Configuration
//!
//! This module provides configuration management for the JSON-RPC server in
//! Rusk with security validation. It handles loading configuration from files,
//! environment variables, and provides default values.
//!
//! ## Features
//!
//! - Load configuration from TOML files
//! - Override settings via environment variables
//! - Default values for all settings
//! - Security validation to prevent insecure configurations
//! - Builder pattern for programmatic configuration
//!
//! ## Configuration Sources (in order of precedence)
//!
//! 1. Environment variables (prefixed with `RUSK_JSONRPC_`)
//! 2. Configuration file (if specified)
//! 3. Default values
//!
//! ## Configuration File Location
//!
//! By default, the configuration file (`jsonrpc_server_config.toml`) is
//! expected at the project root. This can be overridden by setting the
//! `RUSK_JSONRPC_CONFIG_PATH` environment variable.
//!
//! ## Usage Examples
//!
//! ### Basic Usage with Axum
//!
//! ```rust,no_run
//! use axum::{
//!     Router,
//!     routing::post,
//!     extract::State,
//!     response::IntoResponse,
//!     Json
//! };
//! use std::sync::Arc;
//! use std::net::SocketAddr;
//! use rusk::jsonrpc::config::{JsonRpcConfig, ConfigError};
//!
//! // App state that will hold our configuration
//! struct AppState {
//!     config: JsonRpcConfig,
//! }
//!
//! async fn health_check() -> &'static str {
//!     "OK"
//! }
//!
//! async fn handle_request(
//!     State(state): State<Arc<AppState>>,
//!     Json(payload): Json<serde_json::Value>,
//! ) -> impl IntoResponse {
//!     // Access configuration values
//!     let max_batch_size = state.config.features.max_batch_size;
//!
//!     // Use configuration in request handling...
//!     Json(serde_json::json!({
//!         "jsonrpc": "2.0",
//!         "result": format!("Request handled with batch size limit: {}", max_batch_size),
//!         "id": 1
//!     }))
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load configuration (from default location or env var)
//!     let config = JsonRpcConfig::load_default()?;
//!
//!     // Create app state with config
//!     let state = Arc::new(AppState { config: config.clone() });
//!
//!     // Build router with appropriate routes
//!     let app = Router::new()
//!         .route("/health", axum::routing::get(health_check))
//!         .route("/", post(handle_request))
//!         .with_state(state);
//!
//!     // Get bind address from config
//!     let addr = config.http.bind_address;
//!
//!     // Bind server to address and start
//!     let listener = tokio::net::TcpListener::bind(addr).await?;
//!     println!("Listening on {}", addr);
//!
//!     // Start Axum server
//!     axum::serve(listener, app).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Customizing Configuration
//!
//! ```rust
//! use rusk::jsonrpc::config::JsonRpcConfig;
//! use std::net::SocketAddr;
//! use std::str::FromStr;
//! use std::time::Duration;
//!
//! // Using the builder pattern
//! fn create_custom_config() -> JsonRpcConfig {
//!     JsonRpcConfig::builder()
//!         .http_bind_address(SocketAddr::from_str("127.0.0.1:9000").unwrap())
//!         .enable_websocket(true)
//!         .max_block_range(500)
//!         .enable_rate_limiting(true)
//!         .default_rate_limit(200, 60) // 200 requests per minute
//!         .build()
//! }
//!
//! // Loading from a specific file
//! fn load_from_specific_file() -> Result<JsonRpcConfig, Box<dyn std::error::Error>> {
//!     let config = JsonRpcConfig::load(Some(std::path::Path::new("/path/to/config.toml")))?;
//!     Ok(config)
//! }
//! ```
//!
//! ### Applying CORS Configuration with Axum
//!
//! ```rust,no_run
//! use axum::{Router, routing::post};
//! use tower_http::cors::{CorsLayer, Any};
//! use rusk::jsonrpc::config::JsonRpcConfig;
//! use http::{Method, HeaderName};
//! use std::str::FromStr;
//! use std::convert::TryFrom;
//!
//! fn configure_cors(config: &JsonRpcConfig) -> Router {
//!     let mut cors = CorsLayer::new();
//!
//!     // Apply CORS configuration from JsonRpcConfig
//!     if config.http.cors.enabled {
//!         // Configure origins
//!         if config.http.cors.allowed_origins.is_empty() {
//!             // Empty list means allow any origin
//!             cors = cors.allow_origin(Any);
//!         } else if config.http.cors.allowed_origins.iter().any(|o| o == "*") {
//!             // Wildcard origin
//!             cors = cors.allow_origin(Any);
//!         } else {
//!             // Specific origins - in a real implementation, you would need to
//!             // convert these to http::HeaderValue instances
//!             cors = cors.allow_origin(Any); // Simplified for doc test
//!         }
//!
//!         // Configure methods
//!         let methods = config.http.cors.allowed_methods.iter()
//!             .filter_map(|m| Method::from_str(m).ok())
//!             .collect::<Vec<_>>();
//!         cors = cors.allow_methods(methods);
//!
//!         // Configure headers
//!         let headers = config.http.cors.allowed_headers.iter()
//!             .filter_map(|h| HeaderName::from_str(h).ok())
//!             .collect::<Vec<_>>();
//!         cors = cors.allow_headers(headers);
//!
//!         // Configure credentials
//!         cors = cors.allow_credentials(config.http.cors.allow_credentials);
//!
//!         // Configure max age
//!         cors = cors.max_age(std::time::Duration::from_secs(
//!             config.http.cors.max_age_seconds
//!         ));
//!     }
//!
//!     // Create router with CORS middleware
//!     Router::new()
//!         .route("/", post(|| async { "JSON-RPC Server" }))
//!         .layer(cors)
//! }
//! ```
//!
//! ### Setting Up Rate Limiting
//!
//! ```rust,no_run
//! use axum::{Router, routing::post};
//! use tower::ServiceBuilder;
//! use tower_http::limit::RequestBodyLimitLayer;
//! use rusk::jsonrpc::config::JsonRpcConfig;
//!
//! fn configure_layers(config: &JsonRpcConfig) -> Router {
//!     // Create service builder with layers from config
//!     let service = ServiceBuilder::new()
//!         // Apply body size limit from config
//!         .layer(RequestBodyLimitLayer::new(config.http.max_body_size));
//!
//!     // Additional layers would be added here...
//!
//!     // Create router with middleware
//!     Router::new()
//!         .route("/", post(|| async { "JSON-RPC Server" }))
//!         .layer(service)
//! }
//! ```
//!
//! ### In-Memory Configuration for Testing
//!
//! ```rust
//! use rusk::jsonrpc::config::JsonRpcConfig;
//! use std::net::SocketAddr;
//! use std::str::FromStr;
//!
//! #[tokio::test]
//! async fn test_with_config() {
//!     // Create a test configuration
//!     let config = JsonRpcConfig::test_config();
//!
//!     // Or create a specific test configuration
//!     let custom_test_config = JsonRpcConfig::builder()
//!         .http_bind_address(SocketAddr::from_str("127.0.0.1:0").unwrap()) // Random port
//!         .enable_rate_limiting(false) // Disable rate limiting for tests
//!         .build();
//!
//!     // Use in your test...
//!     assert_eq!(custom_test_config.http.bind_address.ip().to_string(), "127.0.0.1");
//!     assert!(!custom_test_config.rate_limit.enabled);
//! }
//! ```
//!
//! ## Environment Variables
//!
//! All configuration options can be set via environment variables with the
//! prefix `RUSK_JSONRPC_`. For example:
//!
//! - `RUSK_JSONRPC_HTTP_BIND_ADDRESS=0.0.0.0:8546`
//! - `RUSK_JSONRPC_FEATURE_MAX_BLOCK_RANGE=500`
//! - `RUSK_JSONRPC_RATE_LIMIT_ENABLED=true`
//!
//! ## Security Validation
//!
//! The configuration module performs security validation to prevent insecure
//! settings. This includes checks for:
//!
//! - Public network interfaces without rate limiting
//! - Insecure CORS configurations
//! - Excessive request body sizes
//! - Overly permissive rate limits
//!
//! Security validations are automatically applied when loading configuration.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{env, fs};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur when loading configuration.
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Whether rate limiting is enabled
    pub enabled: bool,

    /// Default rate limit for all methods
    pub default_limit: RateLimit,

    /// Method-specific rate limits
    pub method_limits: Vec<MethodRateLimit>,

    /// Rate limit for WebSocket connections
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

impl ConfigError {
    /// Create a new validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        ConfigError::Validation(msg.into())
    }

    /// Create a new security validation error
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

    /// Build the final configuration
    pub fn build(self) -> JsonRpcConfig {
        self.config
    }
}

impl JsonRpcConfig {
    /// Default configuration file name
    pub const DEFAULT_CONFIG_FILENAME: &'static str =
        "jsonrpc_server_config.toml";

    /// Create a new builder for JsonRpcConfig
    pub fn builder() -> JsonRpcConfigBuilder {
        JsonRpcConfigBuilder::new()
    }

    /// Load configuration from a TOML file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        let config: JsonRpcConfig = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Load configuration from environment variables.
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = Self::default();

        // Helper to get env vars with prefix
        let get_env =
            |name: &str| std::env::var(format!("RUSK_JSONRPC_{}", name)).ok();

        // HTTP config
        if let Some(addr) = get_env("HTTP_BIND_ADDRESS") {
            if let Ok(socket_addr) = addr.parse() {
                config.http.bind_address = socket_addr;
            }
        }
        if let Some(size) = get_env("HTTP_MAX_BODY_SIZE") {
            if let Ok(size) = size.parse() {
                config.http.max_body_size = size;
            }
        }
        if let Some(timeout) = get_env("HTTP_REQUEST_TIMEOUT_SECS") {
            if let Ok(secs) = timeout.parse() {
                config.http.request_timeout = Duration::from_secs(secs);
            }
        }
        if let Some(connections) = get_env("HTTP_MAX_CONNECTIONS") {
            if let Ok(conn) = connections.parse() {
                config.http.max_connections = conn;
            }
        }

        // CORS config
        if let Some(enabled) = get_env("CORS_ENABLED") {
            config.http.cors.enabled = enabled.to_lowercase() == "true";
        }
        if let Some(origins) = get_env("CORS_ALLOWED_ORIGINS") {
            config.http.cors.allowed_origins = parse_env_string_array(&origins);
        }
        if let Some(methods) = get_env("CORS_ALLOWED_METHODS") {
            config.http.cors.allowed_methods = parse_env_string_array(&methods);
        }
        if let Some(headers) = get_env("CORS_ALLOWED_HEADERS") {
            config.http.cors.allowed_headers = parse_env_string_array(&headers);
        }
        if let Some(credentials) = get_env("CORS_ALLOW_CREDENTIALS") {
            config.http.cors.allow_credentials =
                credentials.to_lowercase() == "true";
        }
        if let Some(max_age) = get_env("CORS_MAX_AGE_SECONDS") {
            if let Ok(age) = max_age.parse() {
                config.http.cors.max_age_seconds = age;
            }
        }

        // WebSocket config
        if let Some(addr) = get_env("WS_BIND_ADDRESS") {
            if let Ok(socket_addr) = addr.parse() {
                config.ws.bind_address = socket_addr;
            }
        }
        if let Some(size) = get_env("WS_MAX_MESSAGE_SIZE") {
            if let Ok(size) = size.parse() {
                config.ws.max_message_size = size;
            }
        }
        if let Some(connections) = get_env("WS_MAX_CONNECTIONS") {
            if let Ok(conn) = connections.parse() {
                config.ws.max_connections = conn;
            }
        }
        if let Some(subscriptions) =
            get_env("WS_MAX_SUBSCRIPTIONS_PER_CONNECTION")
        {
            if let Ok(subs) = subscriptions.parse() {
                config.ws.max_subscriptions_per_connection = subs;
            }
        }
        if let Some(timeout) = get_env("WS_IDLE_TIMEOUT_SECS") {
            if let Ok(secs) = timeout.parse() {
                config.ws.idle_timeout = Duration::from_secs(secs);
            }
        }
        if let Some(events) = get_env("WS_MAX_EVENTS_PER_SECOND") {
            if let Ok(count) = events.parse() {
                config.ws.max_events_per_second = count;
            }
        }

        // Rate limiting
        if let Some(enabled) = get_env("RATE_LIMIT_ENABLED") {
            config.rate_limit.enabled = enabled.to_lowercase() == "true";
        }
        if let Some(requests) = get_env("RATE_LIMIT_DEFAULT_REQUESTS") {
            if let Ok(req) = requests.parse() {
                config.rate_limit.default_limit.requests = req;
            }
        }
        if let Some(window) = get_env("RATE_LIMIT_DEFAULT_WINDOW_SECS") {
            if let Ok(secs) = window.parse() {
                config.rate_limit.default_limit.window =
                    Duration::from_secs(secs);
            }
        }

        // Features
        if let Some(enabled) = get_env("FEATURE_ENABLE_WEBSOCKET") {
            config.features.enable_websocket = enabled.to_lowercase() == "true";
        }
        if let Some(enabled) = get_env("FEATURE_DETAILED_ERRORS") {
            config.features.detailed_errors = enabled.to_lowercase() == "true";
        }
        if let Some(enabled) = get_env("FEATURE_METHOD_TIMING") {
            config.features.method_timing = enabled.to_lowercase() == "true";
        }
        if let Some(enabled) = get_env("FEATURE_STRICT_VERSION_CHECKING") {
            config.features.strict_version_checking =
                enabled.to_lowercase() == "true";
        }
        if let Some(enabled) = get_env("FEATURE_STRICT_PARAMETER_VALIDATION") {
            config.features.strict_parameter_validation =
                enabled.to_lowercase() == "true";
        }
        if let Some(range) = get_env("FEATURE_MAX_BLOCK_RANGE") {
            if let Ok(r) = range.parse() {
                config.features.max_block_range = r;
            }
        }
        if let Some(size) = get_env("FEATURE_MAX_BATCH_SIZE") {
            if let Ok(s) = size.parse() {
                config.features.max_batch_size = s;
            }
        }

        config.validate()?;
        Ok(config)
    }

    /// Load configuration from the default path
    pub fn load_default() -> Result<Self, ConfigError> {
        // Load configuration from the default path
        let default_path = Self::default_config_path();
        // Merge default configuration with environment variables
        Self::load(Some(&default_path))
    }

    /// Load configuration with the following precedence:
    /// 1. Environment variables
    /// 2. Config file (if specified)
    /// 3. Default values
    pub fn load(config_file: Option<&Path>) -> Result<Self, ConfigError> {
        // Start with defaults
        let mut config = Self::default();

        // Override with config file if present
        if let Some(path) = config_file {
            if path.exists() {
                config = Self::from_file(path)?;
            }
        }

        // Get environment configuration
        let env_config = Self::from_env()?;

        // Merge environment variables with higher precedence
        // HTTP configuration
        if std::env::var("RUSK_JSONRPC_HTTP_BIND_ADDRESS").is_ok() {
            config.http.bind_address = env_config.http.bind_address;
        }
        if std::env::var("RUSK_JSONRPC_HTTP_MAX_BODY_SIZE").is_ok() {
            config.http.max_body_size = env_config.http.max_body_size;
        }
        if std::env::var("RUSK_JSONRPC_HTTP_REQUEST_TIMEOUT_SECS").is_ok() {
            config.http.request_timeout = env_config.http.request_timeout;
        }
        if std::env::var("RUSK_JSONRPC_HTTP_MAX_CONNECTIONS").is_ok() {
            config.http.max_connections = env_config.http.max_connections;
        }

        // CORS configuration
        if std::env::var("RUSK_JSONRPC_CORS_ENABLED").is_ok() {
            config.http.cors.enabled = env_config.http.cors.enabled;
        }
        if std::env::var("RUSK_JSONRPC_CORS_ALLOWED_ORIGINS").is_ok() {
            config.http.cors.allowed_origins =
                env_config.http.cors.allowed_origins;
        }
        if std::env::var("RUSK_JSONRPC_CORS_ALLOWED_METHODS").is_ok() {
            config.http.cors.allowed_methods =
                env_config.http.cors.allowed_methods;
        }
        if std::env::var("RUSK_JSONRPC_CORS_ALLOWED_HEADERS").is_ok() {
            config.http.cors.allowed_headers =
                env_config.http.cors.allowed_headers;
        }
        if std::env::var("RUSK_JSONRPC_CORS_ALLOW_CREDENTIALS").is_ok() {
            config.http.cors.allow_credentials =
                env_config.http.cors.allow_credentials;
        }
        if std::env::var("RUSK_JSONRPC_CORS_MAX_AGE_SECONDS").is_ok() {
            config.http.cors.max_age_seconds =
                env_config.http.cors.max_age_seconds;
        }

        // WebSocket configuration
        if std::env::var("RUSK_JSONRPC_WS_BIND_ADDRESS").is_ok() {
            config.ws.bind_address = env_config.ws.bind_address;
        }
        if std::env::var("RUSK_JSONRPC_WS_MAX_MESSAGE_SIZE").is_ok() {
            config.ws.max_message_size = env_config.ws.max_message_size;
        }
        if std::env::var("RUSK_JSONRPC_WS_MAX_CONNECTIONS").is_ok() {
            config.ws.max_connections = env_config.ws.max_connections;
        }
        if std::env::var("RUSK_JSONRPC_WS_MAX_SUBSCRIPTIONS_PER_CONNECTION")
            .is_ok()
        {
            config.ws.max_subscriptions_per_connection =
                env_config.ws.max_subscriptions_per_connection;
        }
        if std::env::var("RUSK_JSONRPC_WS_IDLE_TIMEOUT_SECS").is_ok() {
            config.ws.idle_timeout = env_config.ws.idle_timeout;
        }
        if std::env::var("RUSK_JSONRPC_WS_MAX_EVENTS_PER_SECOND").is_ok() {
            config.ws.max_events_per_second =
                env_config.ws.max_events_per_second;
        }

        // Rate limiting configuration
        if std::env::var("RUSK_JSONRPC_RATE_LIMIT_ENABLED").is_ok() {
            config.rate_limit.enabled = env_config.rate_limit.enabled;
        }
        if std::env::var("RUSK_JSONRPC_RATE_LIMIT_DEFAULT_REQUESTS").is_ok() {
            config.rate_limit.default_limit.requests =
                env_config.rate_limit.default_limit.requests;
        }
        if std::env::var("RUSK_JSONRPC_RATE_LIMIT_DEFAULT_WINDOW_SECS").is_ok()
        {
            config.rate_limit.default_limit.window =
                env_config.rate_limit.default_limit.window;
        }

        // For method-specific rate limits, we would need a more complex
        // approach For now, we'll replace the whole array if any method
        // limit is defined
        if std::env::var("RUSK_JSONRPC_RATE_LIMIT_METHOD_LIMITS").is_ok() {
            config.rate_limit.method_limits =
                env_config.rate_limit.method_limits;
        }

        // WebSocket rate limit
        if std::env::var("RUSK_JSONRPC_RATE_LIMIT_WEBSOCKET_REQUESTS").is_ok() {
            config.rate_limit.websocket_limit.requests =
                env_config.rate_limit.websocket_limit.requests;
        }
        if std::env::var("RUSK_JSONRPC_RATE_LIMIT_WEBSOCKET_WINDOW_SECS")
            .is_ok()
        {
            config.rate_limit.websocket_limit.window =
                env_config.rate_limit.websocket_limit.window;
        }

        // Feature toggles
        if std::env::var("RUSK_JSONRPC_FEATURE_ENABLE_WEBSOCKET").is_ok() {
            config.features.enable_websocket =
                env_config.features.enable_websocket;
        }
        if std::env::var("RUSK_JSONRPC_FEATURE_DETAILED_ERRORS").is_ok() {
            config.features.detailed_errors =
                env_config.features.detailed_errors;
        }
        if std::env::var("RUSK_JSONRPC_FEATURE_METHOD_TIMING").is_ok() {
            config.features.method_timing = env_config.features.method_timing;
        }
        if std::env::var("RUSK_JSONRPC_FEATURE_STRICT_VERSION_CHECKING").is_ok()
        {
            config.features.strict_version_checking =
                env_config.features.strict_version_checking;
        }
        if std::env::var("RUSK_JSONRPC_FEATURE_STRICT_PARAMETER_VALIDATION")
            .is_ok()
        {
            config.features.strict_parameter_validation =
                env_config.features.strict_parameter_validation;
        }
        if std::env::var("RUSK_JSONRPC_FEATURE_MAX_BLOCK_RANGE").is_ok() {
            config.features.max_block_range =
                env_config.features.max_block_range;
        }
        if std::env::var("RUSK_JSONRPC_FEATURE_MAX_BATCH_SIZE").is_ok() {
            config.features.max_batch_size = env_config.features.max_batch_size;
        }

        config.validate()?;
        Ok(config)
    }

    /// Create a configuration for testing
    pub fn test_config() -> Self {
        let mut config = Self::default();
        config.http.bind_address =
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        config.ws.bind_address =
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        config.rate_limit.enabled = false;
        config
    }

    /// Export the current configuration to a TOML string
    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    /// Export the current configuration to a file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let toml = self.to_toml_string()?;
        fs::write(path, toml)?;
        Ok(())
    }

    /// Validates configuration values with additional security checks
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate HTTP configuration
        if self.http.max_body_size == 0 {
            return Err(ConfigError::validation(
                "http.max_body_size must be greater than 0",
            ));
        }
        if self.http.max_connections == 0 {
            return Err(ConfigError::validation(
                "http.max_connections must be greater than 0",
            ));
        }
        if self.http.request_timeout.as_secs() == 0 {
            return Err(ConfigError::validation(
                "http.request_timeout must be greater than 0",
            ));
        }

        // Validate WebSocket configuration
        if self.ws.max_message_size == 0 {
            return Err(ConfigError::validation(
                "ws.max_message_size must be greater than 0",
            ));
        }
        if self.ws.max_connections == 0 {
            return Err(ConfigError::validation(
                "ws.max_connections must be greater than 0",
            ));
        }
        if self.ws.max_subscriptions_per_connection == 0 {
            return Err(ConfigError::validation(
                "ws.max_subscriptions_per_connection must be greater than 0",
            ));
        }
        if self.ws.idle_timeout.as_secs() == 0 {
            return Err(ConfigError::validation(
                "ws.idle_timeout must be greater than 0",
            ));
        }
        if self.ws.max_events_per_second == 0 {
            return Err(ConfigError::validation(
                "ws.max_events_per_second must be greater than 0",
            ));
        }

        // Validate rate limit configuration
        if self.rate_limit.enabled {
            if self.rate_limit.default_limit.requests == 0 {
                return Err(ConfigError::validation(
                    "rate_limit.default_limit.requests must be greater than 0",
                ));
            }
            if self.rate_limit.default_limit.window.as_secs() == 0 {
                return Err(ConfigError::validation(
                    "rate_limit.default_limit.window must be greater than 0",
                ));
            }
            if self.rate_limit.websocket_limit.requests == 0 {
                return Err(ConfigError::validation("rate_limit.websocket_limit.requests must be greater than 0"));
            }
            if self.rate_limit.websocket_limit.window.as_secs() == 0 {
                return Err(ConfigError::validation(
                    "rate_limit.websocket_limit.window must be greater than 0",
                ));
            }

            // Validate method-specific rate limits
            for (i, method_limit) in
                self.rate_limit.method_limits.iter().enumerate()
            {
                if method_limit.method_pattern.is_empty() {
                    return Err(ConfigError::validation(format!("rate_limit.method_limits[{}].method_pattern must not be empty", i)));
                }
                if method_limit.limit.requests == 0 {
                    return Err(ConfigError::validation(format!("rate_limit.method_limits[{}].limit.requests must be greater than 0", i)));
                }
                if method_limit.limit.window.as_secs() == 0 {
                    return Err(ConfigError::validation(format!("rate_limit.method_limits[{}].limit.window must be greater than 0", i)));
                }
            }
        }

        // Validate feature configuration
        if self.features.max_block_range == 0 {
            return Err(ConfigError::validation(
                "features.max_block_range must be greater than 0",
            ));
        }
        if self.features.max_batch_size == 0 {
            return Err(ConfigError::validation(
                "features.max_batch_size must be greater than 0",
            ));
        }

        // --- Additional security validation checks ---

        // 1. Check for insecure bind addresses (public interfaces without rate
        //    limiting)
        if !self.rate_limit.enabled {
            // Check if HTTP server is binding to a public interface
            if is_public_interface(&self.http.bind_address) {
                return Err(ConfigError::security_violation(
                    "Binding to public interface without rate limiting is insecure"
                ));
            }

            // Check if WebSocket server is binding to a public interface
            if is_public_interface(&self.ws.bind_address) {
                return Err(ConfigError::security_violation(
                    "Binding WebSocket to public interface without rate limiting is insecure"
                ));
            }
        }

        // 2. Check CORS configuration (if enabled)
        if self.http.cors.enabled {
            // Check for overly permissive CORS settings
            let has_wildcard_origin = self
                .http
                .cors
                .allowed_origins
                .iter()
                .any(|origin| origin == "*");

            // If wildcard origin and credentials are allowed, this is insecure
            if has_wildcard_origin && self.http.cors.allow_credentials {
                return Err(ConfigError::security_violation(
                    "Allowing wildcard CORS origin (*) with credentials is insecure"
                ));
            }
        }

        // 3. Check for excessively large request limits
        const MAX_SAFE_REQUEST_SIZE: usize = 100 * 1024 * 1024; // 100 MB
        if self.http.max_body_size > MAX_SAFE_REQUEST_SIZE {
            return Err(ConfigError::security_violation(format!(
                "Request body size limit of {} bytes exceeds recommended maximum of {} bytes",
                self.http.max_body_size, MAX_SAFE_REQUEST_SIZE
            )));
        }

        // 4. Check for excessively large WebSocket message size
        const MAX_SAFE_WS_MESSAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MB
        if self.ws.max_message_size > MAX_SAFE_WS_MESSAGE_SIZE {
            return Err(ConfigError::security_violation(format!(
                "WebSocket message size limit of {} bytes exceeds recommended maximum of {} bytes",
                self.ws.max_message_size, MAX_SAFE_WS_MESSAGE_SIZE
            )));
        }

        // 5. Check for disabled security features
        if !self.features.strict_parameter_validation {
            return Err(ConfigError::security_violation(
                "Disabling strict parameter validation is not recommended for production"
            ));
        }

        // 6. Check rate limiting settings
        if self.rate_limit.enabled {
            // Check if rate limits are reasonable (not too high)
            const MAX_SAFE_DEFAULT_RATE: u64 = 1000; // per minute
            if self.rate_limit.default_limit.requests > MAX_SAFE_DEFAULT_RATE
                && self.rate_limit.default_limit.window.as_secs() <= 60
            {
                return Err(ConfigError::security_violation(format!(
                    "Default rate limit of {} requests per minute exceeds recommended maximum of {}",
                    self.rate_limit.default_limit.requests, MAX_SAFE_DEFAULT_RATE
                )));
            }

            // Check for any unlimited methods (extremely high limits)
            for method_limit in &self.rate_limit.method_limits {
                const UNLIMITED_THRESHOLD: u64 = 10000; // per minute
                let requests_per_minute =
                    if method_limit.limit.window.as_secs() == 0 {
                        u64::MAX // Avoid division by zero
                    } else {
                        method_limit.limit.requests * 60
                            / method_limit.limit.window.as_secs()
                    };

                if requests_per_minute > UNLIMITED_THRESHOLD {
                    return Err(ConfigError::security_violation(format!(
                        "Method '{}' has an extremely high rate limit (equivalent to {} requests per minute)",
                        method_limit.method_pattern, requests_per_minute
                    )));
                }
            }
        }

        // 7. Check for secure defaults in block query limits
        const MAX_SAFE_BLOCK_RANGE: u64 = 10000;
        if self.features.max_block_range > MAX_SAFE_BLOCK_RANGE {
            return Err(ConfigError::security_violation(format!(
                "Block range limit of {} exceeds recommended maximum of {}",
                self.features.max_block_range, MAX_SAFE_BLOCK_RANGE
            )));
        }

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
}

impl Default for JsonRpcConfig {
    fn default() -> Self {
        Self {
            http: HttpServerConfig::default(),
            ws: WebSocketServerConfig::default(),
            rate_limit: RateLimitConfig::default(),
            features: FeatureToggles::default(),
        }
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
        }
    }
}

impl Default for WebSocketServerConfig {
    fn default() -> Self {
        Self {
            bind_address: default_ws_address(),
            max_message_size: 1024 * 1024, // 1 MB
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
            allowed_origins: Vec::new(), // Empty means all origins
            allowed_methods: vec!["POST".to_string(), "GET".to_string()],
            allowed_headers: vec![
                "Content-Type".to_string(),
                "Rusk-Version".to_string(),
            ],
            allow_credentials: false,
            max_age_seconds: 86400, // 24 hours
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
            method_pattern: "".to_string(), // Empty string as default pattern
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
    Duration::from_secs(60) // 1 minute
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
