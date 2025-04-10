// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # Application State Management for JSON-RPC Server
//!
//! This module defines the `AppState` struct, which serves as the central
//! container for shared resources and configuration required by JSON-RPC method
//! handlers and potentially other web framework handlers (like `axum`). It
//! ensures that components like configuration, database access, subscription
//! management, metrics, and rate limiting are accessible in a thread-safe
//! manner throughout the application.
//!
//! ## Design
//!
//! - `AppState` uses `Arc` for shared ownership of immutable or thread-safe
//!   components (like `JsonRpcConfig`, `DatabaseAdapter`, `MetricsCollector`).
//! - `Arc<RwLock>` is used for components requiring mutable access across
//!   threads (like `SubscriptionManager`).
//! - It implements `Clone`, allowing cheap cloning for sharing across tasks or
//!   handlers.
//! - `Send + Sync` are implicitly satisfied due to the use of `Arc`, `RwLock`,
//!   and the `Send + Sync` bounds on `dyn DatabaseAdapter`.
//!
//! ## Integration with Axum
//!
//! When used with the `axum` web framework, an instance of `AppState`
//! (typically wrapped in an `Arc`) is provided to the `axum::Router` using the
//! `.with_state()` method. Handlers can then access the shared state via the
//! `axum::extract::State` extractor.
//!
//! ```rust,ignore
//! // Example: Setting up Axum router with AppState
//! use axum::{routing::get, Router, extract::State};
//! use std::sync::Arc;
//! use dusk_network::jsonrpc::infrastructure::state::{AppState, SubscriptionManager};
//! use dusk_network::jsonrpc::config::JsonRpcConfig;
//! use dusk_network::jsonrpc::infrastructure::db::DatabaseAdapter; // Assuming a mock or real impl
//! use dusk_network::jsonrpc::infrastructure::metrics::MetricsCollector;
//! use dusk_network::jsonrpc::infrastructure::manual_limiter::ManualRateLimiters;
//!
//! // Assume these are initialized appropriately
//! let config = JsonRpcConfig::default();
//! struct MockDb;
//! impl DatabaseAdapter for MockDb { /* ... */ }
//! let db_adapter: Arc<dyn DatabaseAdapter> = Arc::new(MockDb);
//! let subscription_manager = SubscriptionManager::default();
//! let metrics_collector = MetricsCollector::default(); // Assuming default impl
//! let manual_rate_limiters = ManualRateLimiters::default(); // Assuming default impl
//!
//! let app_state = AppState::new(
//!     config,
//!     db_adapter,
//!     subscription_manager,
//!     metrics_collector,
//!     manual_rate_limiters,
//! );
//!
//! // Create the Axum router and provide the state
//! let app = Router::new()
//!     .route("/health", get(health_handler))
//!     // Add other routes...
//!     .with_state(app_state); // Pass AppState to the router
//!
//! // Example handler accessing the state
//! async fn health_handler(State(state): State<AppState>) -> &'static str {
//!     println!("Current config enable_http: {}", state.config().enable_http);
//!     // Access other state components via state.db_adapter(), state.metrics_collector(), etc.
//!     "OK"
//! }
//!
//! // Server launch would follow...
//! // let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
//! // axum::serve(listener, app).await.unwrap();
//! ```

use crate::jsonrpc::config::JsonRpcConfig;
use crate::jsonrpc::infrastructure::db::DatabaseAdapter;
use crate::jsonrpc::infrastructure::manual_limiter::ManualRateLimiters;
use crate::jsonrpc::infrastructure::metrics::MetricsCollector;
use parking_lot::RwLock;
use std::sync::Arc;

// --- Placeholder Types ---

/// Placeholder struct for managing WebSocket subscriptions.
///
/// This struct will handle subscription requests, store active subscriptions,
/// and manage broadcasting events to subscribers.
#[derive(Debug, Default)]
pub struct SubscriptionManager {
    // TODO: Add fields for managing subscriptions later.
}

// --- End Placeholder Types ---

/// Central application state shared across JSON-RPC and web handlers.
///
/// Holds configuration and references to shared infrastructure components like
/// database access, metrics, and subscription management. It is designed to be
/// cloneable and thread-safe (`Send + Sync`).
///
/// ## Design Rationale: Dynamic Dispatch (`Arc<dyn Trait>`)
///
/// This struct uses `Arc<dyn DatabaseAdapter>` for the database component
/// instead of making `AppState` generic like `AppState<DB: DatabaseAdapter>`.
/// This design choice favors **dynamic dispatch** for several reasons:
///
/// 1. **Flexibility & Testability:** It allows `AppState` to hold *any* type
///    implementing `DatabaseAdapter` without the `AppState` type itself
///    changing. This makes it easy to swap implementations, especially for
///    testing where a `MockDbAdapter` can be injected instead of the real one,
///    without altering handler signatures.
/// 2. **Avoids Generic Propagation:** A generic `AppState<DB>` would require
///    the `DB` type parameter to be added to potentially many functions and
///    structs that use `AppState`, increasing code complexity. Dynamic dispatch
///    contains this complexity.
/// 3. **Simplified Usage:** Consumers of `AppState` work with a single,
///    concrete type, simplifying function signatures and usage patterns.
///
/// While dynamic dispatch has a small runtime overhead compared to static
/// dispatch (generics), the flexibility and simplified architecture are
/// generally more beneficial for shared application state, especially when
/// components are already behind an `Arc`.
///
/// ## Example
///
/// ```rust
/// use std::sync::Arc;
/// use parking_lot::RwLock;
/// use dusk_network::jsonrpc::infrastructure::state::{AppState, SubscriptionManager};
/// use dusk_network::jsonrpc::config::JsonRpcConfig;
/// use dusk_network::jsonrpc::infrastructure::db::DatabaseAdapter; // Assuming trait is in scope
/// use dusk_network::jsonrpc::infrastructure::metrics::MetricsCollector;
/// use dusk_network::jsonrpc::infrastructure::manual_limiter::ManualRateLimiters;
///
/// // Define a mock database adapter for the example
/// #[derive(Debug)]
/// struct MockDbAdapter;
/// impl DatabaseAdapter for MockDbAdapter {
///     // Implement trait methods...
/// }
///
/// // Initialize components
/// let config = JsonRpcConfig::default();
/// let db_adapter: Arc<dyn DatabaseAdapter> = Arc::new(MockDbAdapter);
/// let subscription_manager = SubscriptionManager::default();
/// let metrics_collector = MetricsCollector::default(); // Assuming Default impl
/// let manual_rate_limiters = ManualRateLimiters::default(); // Assuming Default impl
///
/// // Create the AppState
/// let state = AppState::new(
///     config,
///     Arc::clone(&db_adapter), // Clone Arc for ownership transfer
///     subscription_manager,
///     metrics_collector,
///     manual_rate_limiters,
/// );
///
/// // Clone the state (cheap Arc clone) for sharing
/// let state_clone = state.clone();
///
/// // Access components
/// println!("Config HTTP enabled: {}", state.config().enable_http);
/// let _db = state.db_adapter(); // Access the trait object
/// let _subs_lock = state.subscription_manager().read(); // Acquire read lock
/// ```
#[derive(Debug, Clone)]
pub struct AppState {
    /// Shared JSON-RPC server configuration.
    config: Arc<JsonRpcConfig>,

    /// Shared database adapter instance.
    /// Provides access to Rusk's backend data.
    db_adapter: Arc<dyn DatabaseAdapter>,

    /// Shared subscription manager for WebSocket event handling.
    /// Needs `RwLock` for managing mutable subscription state.
    subscription_manager: Arc<RwLock<SubscriptionManager>>,

    /// Shared metrics collector instance.
    metrics_collector: Arc<MetricsCollector>,

    /// Shared manual rate limiters for WebSockets and specific methods.
    manual_rate_limiters: Arc<ManualRateLimiters>,
}

impl AppState {
    /// Creates a new `AppState` instance.
    ///
    /// Initializes the shared state container with the provided configuration
    /// and infrastructure components. Components are wrapped in `Arc` or
    /// `Arc<RwLock>` to enable safe sharing across threads.
    ///
    /// # Arguments
    ///
    /// * `config` - The JSON-RPC server configuration.
    /// * `db_adapter` - An implementation of the `DatabaseAdapter` trait.
    /// * `subscription_manager` - The manager for WebSocket subscriptions.
    /// * `metrics_collector` - The collector for server metrics.
    /// * `manual_rate_limiters` - The manager for manual rate limiting.
    ///
    /// # Returns
    ///
    /// A new `AppState` instance ready to be shared.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: JsonRpcConfig,
        db_adapter: Arc<dyn DatabaseAdapter>,
        subscription_manager: SubscriptionManager,
        metrics_collector: MetricsCollector,
        manual_rate_limiters: ManualRateLimiters,
    ) -> Self {
        Self {
            config: Arc::new(config),
            db_adapter,
            subscription_manager: Arc::new(RwLock::new(subscription_manager)),
            metrics_collector: Arc::new(metrics_collector),
            manual_rate_limiters: Arc::new(manual_rate_limiters),
        }
    }

    /// Returns a reference to the shared JSON-RPC configuration.
    ///
    /// The configuration is wrapped in an `Arc`, allowing cheap cloning if
    /// needed.
    pub fn config(&self) -> &Arc<JsonRpcConfig> {
        &self.config
    }

    /// Returns a reference to the shared database adapter.
    ///
    /// The adapter is wrapped in an `Arc<dyn ...>`, allowing shared access to
    /// the database implementation.
    pub fn db_adapter(&self) -> &Arc<dyn DatabaseAdapter> {
        &self.db_adapter
    }

    /// Returns a reference to the shared subscription manager.
    ///
    /// The manager is wrapped in an `Arc<RwLock<...>>`, allowing thread-safe
    /// read and write access to subscription state. Use `.read()` or `.write()`
    /// on the returned `Arc` to acquire the lock.
    pub fn subscription_manager(&self) -> &Arc<RwLock<SubscriptionManager>> {
        &self.subscription_manager
    }

    /// Returns a reference to the shared metrics collector.
    ///
    /// The collector is wrapped in an `Arc`, allowing cheap cloning and shared
    /// access for recording metrics.
    pub fn metrics_collector(&self) -> &Arc<MetricsCollector> {
        &self.metrics_collector
    }

    /// Returns a reference to the shared manual rate limiters.
    ///
    /// The limiters are wrapped in an `Arc`, allowing cheap cloning and shared
    /// access for performing manual rate limit checks.
    pub fn manual_rate_limiters(&self) -> &Arc<ManualRateLimiters> {
        &self.manual_rate_limiters
    }
}
