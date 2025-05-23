// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # Manual Rate Limiting for Specific JSON-RPC Scenarios
//!
//! This module provides the `ManualRateLimiters` struct, responsible for
//! enforcing rate limits for specific scenarios not covered by the main
//! `tower-governor` middleware layer. These include:
//!
//! 1. **WebSocket Connections:** Limiting the rate at which a single client (IP
//!    address) can establish new WebSocket connections.
//! 2. **Method-Specific Limits:** Applying different rate limits to specific
//!    JSON-RPC methods or method patterns (e.g., allowing more `get*` calls
//!    than expensive `transfer` calls).
//!
//! ## Design Rationale: Why Not Middleware for Everything?
//!
//! While using a single middleware layer (like `tower-governor`) for all rate
//! limiting seems appealing for uniformity, it presents significant practical
//! challenges for WebSocket connection and method-specific limits:
//!
//! ### 1. Accessing the JSON-RPC Method Name in Middleware
//!
//! - **Problem:** Standard `tower` middleware operates on raw `http::Request`
//!   objects *before* the request body is typically processed by the
//!   application framework (`jsonrpsee` in this case). The specific JSON-RPC
//!   method name (e.g., `"getTransactionsByMemo"`) is located *inside* the JSON
//!   payload of the request body.
//! - **Inefficiency:** For a middleware layer to access the method name, it
//!   would need to buffer the *entire* request body, parse it as JSON,
//!   deserialize the JSON-RPC request structure, and extract the `"method"`
//!   field. This process would have to occur for **every single incoming HTTP
//!   request**, even those not requiring method-specific limits or not even
//!   being JSON-RPC calls. This adds substantial overhead and complexity,
//!   potentially negating the performance benefits of middleware.
//! - **Lifecycle Mismatch:** `jsonrpsee` internally handles the body parsing
//!   and method dispatch *after* the main middleware stack has run. Performing
//!   the method-specific check within the `jsonrpsee` method handler itself is
//!   far more efficient, as the method name is already known at that point.
//!
//! ### 2. Handling WebSocket Connection Limits in Middleware
//!
//! - **Problem:** WebSocket connections begin as HTTP `Upgrade` requests. While
//!   middleware *could* potentially identify these requests, applying a rate
//!   limit strictly at the HTTP middleware layer is problematic. The limit
//!   needs to be applied based on the *successful establishment* of a WebSocket
//!   session, not just the initial HTTP request.
//! - **Complexity:** Integrating this check reliably into the standard HTTP
//!   middleware flow, before the WebSocket handshake is fully managed and
//!   accepted by `jsonrpsee`, is complex and potentially fragile. It's cleaner
//!   to perform this check within the server's specific WebSocket connection
//!   acceptance logic.
//!
//! ### 3. The Chosen Approach: Separation of Concerns
//!
//! This implementation adopts a two-part strategy for efficiency and
//! correctness:
//!
//! - **`tower-governor` Middleware:** Handles the broad, default per-IP rate
//!   limit for all incoming HTTP requests. This is efficient as it relies only
//!   on readily available IP address information from the connection metadata.
//! - **`ManualRateLimiters` Struct:** This struct manages the state for
//!   WebSocket and method-specific limits. Its checking methods
//!   (`check_websocket_limit`, `check_method_limit`) are designed to be called
//!   **explicitly** from the appropriate points in the application logic:
//!     - `check_websocket_limit`: Called during the WebSocket connection
//!       setup/acceptance phase.
//!     - `check_method_limit`: Called at the beginning of individual JSON-RPC
//!       method handlers where specific limits apply (after `jsonrpsee` has
//!       parsed the request and identified the method).
//!
//! This separation ensures that each type of rate limit is checked at the most
//! appropriate and efficient point in the request/connection lifecycle, using
//! the necessary context (IP address, connection state, method name) when it
//! becomes available.
//!
//! ## Usage
//!
//! An instance of `Arc<ManualRateLimiters>` is expected to be created during
//! server initialization (using the `RateLimitConfig`) and stored within
//! `AppState`. The check methods are then called manually from the relevant
//! code paths.

use crate::jsonrpc::config::{RateLimit, RateLimitConfig};
use crate::jsonrpc::infrastructure::client_info::ClientInfo;
use crate::jsonrpc::infrastructure::error::RateLimitError;
use dashmap::DashMap;
use glob::Pattern;
use governor::{
    clock::DefaultClock, state::keyed::DefaultKeyedStateStore, Quota,
    RateLimiter,
};
use std::{sync::Arc, time::Duration};

/// Type alias for the base rate limiter used throughout this module.
///
/// This rate limiter:
/// - Uses `ClientInfo` as the key type, allowing rate limiting by client
///   identity
/// - Uses the default keyed state store from governor for tracking rate limit
///   state
/// - Uses the default clock implementation for time tracking
///
/// This type alias simplifies the code by avoiding repetition of these generic
/// parameters.
type BaseLimiter = RateLimiter<
    ClientInfo,                         // Key type
    DefaultKeyedStateStore<ClientInfo>, // Keyed state store
    DefaultClock,                       // Clock
>;

/// Type alias for the map holding specific limiters for method patterns for a
/// single client. Maps: Method Pattern (String) -> Specific Rate Limiter
/// (Arc<BaseLimiter>)
type PatternLimiterMap = DashMap<String, Arc<BaseLimiter>>;

/// Type alias for the shared, thread-safe map holding method pattern limiters
/// for a single client.
type ClientMethodLimiters = Arc<PatternLimiterMap>;

/// Type alias for the top-level, shared, thread-safe map holding method
/// limiters per client. Maps: Client Info -> Client's Method Limiters Map
/// (ClientMethodLimiters)
type MethodLimiters = Arc<DashMap<ClientInfo, ClientMethodLimiters>>;

/// Manages state for WebSocket connection and method-specific rate limits.
///
/// This struct maintains the state needed for enforcing rate limits on both
/// WebSocket connections and specific JSON-RPC method calls. It uses the
/// `governor` crate for the underlying rate limiting functionality.
///
/// # Structure
///
/// - Stores compiled glob patterns for method-specific rate limits
/// - Maintains separate rate limiters for WebSocket connections and method
///   calls
/// - Uses client information (primarily IP address) as the key for rate
///   limiting
///
/// # Examples
///
/// Creating a new instance with a configuration:
///
/// ```
/// use std::sync::Arc;
/// use std::time::Duration;
/// use rusk::jsonrpc::config::{MethodRateLimit, RateLimit, RateLimitConfig};
/// use rusk::jsonrpc::infrastructure::manual_limiter::ManualRateLimiters;
///
/// let config = Arc::new(RateLimitConfig {
///     enabled: true,
///     websocket_limit: RateLimit {
///         requests: 5,
///         window: Duration::from_secs(60),
///     },
///     method_limits: vec![
///         // Create MethodRateLimit struct directly
///         MethodRateLimit {
///             method_pattern: "getTransactionsBy*".to_string(),
///             limit: RateLimit { requests: 100, window: Duration::from_secs(60) },
///         },
///     ],
///     // Use default_limit field
///     default_limit: RateLimit::default(),
/// });
///
/// let limiters = ManualRateLimiters::new(config).expect("Failed to create rate limiters");
/// ```
///
/// Checking if a client can establish a new WebSocket connection:
///
/// ```
/// use std::net::{IpAddr, Ipv4Addr, SocketAddr};
/// use std::sync::Arc;
/// use std::time::Duration;
/// use rusk::jsonrpc::config::{RateLimit, RateLimitConfig};
/// use rusk::jsonrpc::infrastructure::client_info::ClientInfo;
/// use rusk::jsonrpc::infrastructure::manual_limiter::ManualRateLimiters;
///
/// // Create a minimal valid config for the doctest
/// let config = Arc::new(RateLimitConfig {
///     enabled: true,
///     websocket_limit: RateLimit { requests: 5, window: Duration::from_secs(1) },
///     method_limits: vec![], // No method limits needed for this example
///     default_limit: RateLimit::default(), // Default is needed
/// });
///
/// // Create the limiters instance
/// let limiters = ManualRateLimiters::new(config).expect("Doctest setup failed");
///
/// let client_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12345);
/// let client_info = ClientInfo::new(client_addr);
///
/// match limiters.check_websocket_limit(&client_info) {
///     Ok(()) => {
///         // Allow the WebSocket connection
///         println!("Connection allowed"); // Added print for test verification
///     },
///     Err(err) => {
///         // Reject the connection with appropriate error
///         eprintln!("Connection rejected: {}", err); // Print error for debugging if needed
///     }
/// }
///
/// assert!(limiters.check_websocket_limit(&client_info).is_ok());
/// ```
#[derive(Debug, Clone)]
pub struct ManualRateLimiters {
    /// Shared rate limiting configuration.
    config: Arc<RateLimitConfig>,
    /// Compiled glob patterns for method-specific limits and their associated
    /// quotas.
    method_patterns: Arc<Vec<(Pattern, Arc<Quota>)>>,
    /// Limiters for new WebSocket connections per client. Stores
    /// `Arc<BaseLimiter>`.
    websocket_limiters: Arc<DashMap<ClientInfo, Arc<BaseLimiter>>>,
    /// Nested map for method-specific limiters, using type aliases for
    /// clarity.
    method_limiters: MethodLimiters,
}

impl ManualRateLimiters {
    /// Creates a new `ManualRateLimiters` instance.
    ///
    /// Reads configuration for WebSocket and method limits, pre-compiles
    /// patterns.
    pub fn new(
        config: Arc<RateLimitConfig>,
    ) -> Result<Self, crate::jsonrpc::config::ConfigError> {
        tracing::debug!("Initializing ManualRateLimiters...");

        let mut compiled_patterns = Vec::new();
        if config.enabled {
            // Validate websocket limit first
            rate_limit_to_quota(&config.websocket_limit)?;
            tracing::debug!("WebSocket rate limit validated.");

            tracing::debug!(
                "Compiling {} method-specific rate limit patterns...",
                config.method_limits.len()
            );
            for method_limit in &config.method_limits {
                let pattern = Pattern::new(&method_limit.method_pattern).map_err(|e| {
                    let msg = format!(
                        "Invalid glob pattern '{}': {}",
                        method_limit.method_pattern, e
                    );
                    tracing::warn!(error = %msg, pattern = %method_limit.method_pattern, "Failed to compile rate limit pattern");
                    crate::jsonrpc::config::ConfigError::validation(msg)
                })?;

                let quota = rate_limit_to_quota(&method_limit.limit)?;
                tracing::trace!(pattern = %method_limit.method_pattern, quota = ?quota, "Compiled method pattern");
                compiled_patterns.push((pattern, Arc::new(quota)));
            }
            tracing::debug!("Finished compiling method patterns.");
        } else {
            tracing::debug!("Rate limiting is disabled globally, skipping pattern compilation for manual limiter.");
        }

        Ok(Self {
            config,
            method_patterns: Arc::new(compiled_patterns),
            websocket_limiters: Arc::new(DashMap::new()),
            method_limiters: Arc::new(DashMap::new()),
        })
    }

    /// Checks if a new WebSocket connection from the client is allowed.
    ///
    /// Applies the limit defined in `config.websocket_limit`.
    ///
    /// # Arguments
    ///
    /// * `client_info` - Information about the client attempting to connect.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the connection is allowed.
    /// * `Err(RateLimitError::ManualWebSocketLimitExceeded)` - If the client
    ///   has exceeded the limit.
    pub fn check_websocket_limit(
        &self,
        client_info: &ClientInfo,
    ) -> Result<(), RateLimitError> {
        if !self.config.enabled {
            tracing::trace!(%client_info, "Rate limiting disabled globally, allowing WebSocket connection.");
            return Ok(());
        }

        tracing::trace!(%client_info, "Checking WebSocket connection rate limit");

        // Get or create the limiter for this client
        let limiter = self
            .websocket_limiters
            .entry(client_info.clone())
            .or_insert_with(|| {
                // .unwrap() is safe here because we validated this quota
                // successfully in `new()`. If `new()` failed
                // due to invalid quota, this `ManualRateLimiters` instance
                // would not have been created.
                let quota =
                    rate_limit_to_quota(&self.config.websocket_limit).unwrap();
                Arc::new(RateLimiter::keyed(quota))
            });

        // Check using the client as the key
        match limiter.check_key(client_info) {
            Ok(_) => {
                tracing::trace!(%client_info, "WebSocket connection rate limit check passed.");
                Ok(())
            }
            Err(_) => {
                // Convert governor's NotUntil error to our specific variant
                tracing::debug!(%client_info, "WebSocket connection rate limit exceeded");
                Err(RateLimitError::ManualWebSocketLimitExceeded(format!(
                    "Client {} exceeded WebSocket connection rate limit",
                    client_info
                )))
            }
        }
    }

    /// Checks if a call to a specific method from the client is allowed based
    /// on method-specific rate limits.
    ///
    /// This function:
    /// 1. Checks if rate limiting is globally enabled
    /// 2. Finds the first matching method pattern for the given method name
    /// 3. Creates or retrieves a rate limiter for the client and pattern
    ///    combination
    /// 4. Applies the rate limit check
    ///
    /// # Arguments
    ///
    /// * `client_info` - Information about the client making the request
    ///   (typically IP address)
    /// * `method_name` - The name of the JSON-RPC method being called
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the method call is allowed
    /// * `Err(RateLimitError::ManualMethodLimitExceeded)` - If the client has
    ///   exceeded the rate limit for the matching method pattern.
    ///
    /// # Rate Limit Behavior
    ///
    /// - If rate limiting is disabled globally, all method calls are allowed
    /// - If no method pattern matches the given method name, the call is
    ///   allowed (default limits are handled by middleware)
    /// - Each client has separate rate limiters for each method pattern
    /// - The first matching pattern is used for rate limiting
    ///
    /// # Examples
    ///
    /// ```
    /// use rusk::jsonrpc::infrastructure::manual_limiter::{ManualRateLimiters};
    /// use rusk::jsonrpc::infrastructure::client_info::ClientInfo;
    /// use rusk::jsonrpc::config::{RateLimitConfig, MethodRateLimit, RateLimit};
    /// use std::sync::Arc;
    /// use std::time::Duration;
    /// use std::net::{IpAddr, Ipv4Addr};
    ///
    /// // Create a rate limit config with a method-specific limit
    /// let mut config = RateLimitConfig::default();
    /// config.enabled = true;
    /// config.method_limits.push(MethodRateLimit {
    ///     method_pattern: "get*".to_string(),
    ///     limit: RateLimit {
    ///         requests: 5,
    ///         window: Duration::from_secs(60),
    ///     },
    /// });
    ///
    /// // Create the manual rate limiters
    /// let limiters = ManualRateLimiters::new(Arc::new(config)).unwrap();
    ///
    /// // Create a client info (typically from the client's IP)
    /// let client_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    /// let client_info = ClientInfo::from_ip(client_ip, 0);
    ///
    /// // Check if a method call is allowed
    /// match limiters.check_method_limit(&client_info, "getBalance") {
    ///     Ok(()) => println!("Method call allowed"),
    ///     Err(e) => println!("Method call denied: {}", e),
    /// }
    ///
    /// // Multiple calls within the rate limit window will eventually be denied
    /// for _ in 0..10 {
    ///     let result = limiters.check_method_limit(&client_info, "getBalance");
    ///     if result.is_err() {
    ///         println!("Rate limit exceeded after multiple calls");
    ///         break;
    ///     }
    /// }
    /// ```
    pub fn check_method_limit(
        &self,
        client_info: &ClientInfo,
        method_name: &str,
    ) -> Result<(), RateLimitError> {
        if !self.config.enabled {
            tracing::trace!(%client_info, %method_name, "Rate limiting disabled globally, allowing method call.");
            return Ok(());
        }

        tracing::trace!(%client_info, %method_name, "Checking method-specific rate limit");

        // Find the first matching pattern
        for (pattern, quota) in self.method_patterns.iter() {
            if pattern.matches(method_name) {
                tracing::trace!(%client_info, %method_name, pattern = %pattern.as_str(), "Found matching method pattern");

                // Get or create the client's map of method limiters
                let client_method_limiters = self.method_limiters.entry(client_info.clone()).or_insert_with(|| {
                    tracing::debug!(%client_info, "Creating new method limiter map for client");
                    Arc::new(DashMap::new())
                });

                // Get or create the specific limiter for this method pattern
                let pattern_limiter = client_method_limiters
                    .entry(pattern.as_str().to_string())
                    .or_insert_with(|| {
                        tracing::debug!(%client_info, pattern = %pattern.as_str(), ?quota, "Creating new method-specific rate limiter for client/pattern");
                        // Clone the quota Arc from the pre-compiled list
                        Arc::new(RateLimiter::keyed(**quota))
                    });

                // Perform the check
                return match pattern_limiter.check_key(client_info) {
                    Ok(_) => {
                        tracing::trace!(%client_info, %method_name, pattern = %pattern.as_str(), "Method-specific rate limit check passed.");
                        Ok(())
                    }
                    Err(_) => {
                        tracing::debug!(%client_info, %method_name, pattern = %pattern.as_str(), "Method-specific rate limit exceeded");
                        Err(RateLimitError::ManualMethodLimitExceeded(format!(
                            "Client {} exceeded rate limit for method pattern '{}'",
                            client_info, pattern.as_str()
                        )))
                    }
                };
            }
        }

        // If no specific pattern matched, the request is allowed by default
        // (default limit handled by middleware)
        tracing::trace!(%client_info, %method_name, "No specific method pattern matched, allowing call.");
        Ok(())
    }
}

// Helper function
fn rate_limit_to_quota(
    rate_limit: &RateLimit,
) -> Result<Quota, crate::jsonrpc::config::ConfigError> {
    let requests = std::num::NonZeroU32::new(rate_limit.requests as u32)
        .ok_or_else(|| {
            crate::jsonrpc::config::ConfigError::validation(format!(
                "Rate limit requests must be non-zero, got {}",
                rate_limit.requests
            ))
        })?;

    if rate_limit.window == Duration::ZERO {
        return Err(crate::jsonrpc::config::ConfigError::validation(
            "Rate limit window must be non-zero",
        ));
    }

    // Create the initial Quota Result
    let quota_result = Quota::with_period(rate_limit.window).ok_or_else(|| {
        crate::jsonrpc::config::ConfigError::validation(format!(
            "Invalid rate limit window: {:?}",
            rate_limit.window
        ))
    });

    // Unwrap the result OR propagate the error
    let base_quota = quota_result?;

    // Apply the burst size and wrap the final Quota in Ok
    Ok(base_quota.allow_burst(requests))
}
