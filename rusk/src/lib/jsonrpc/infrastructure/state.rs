// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # Application State Management for JSON-RPC Server
//!
//! This module defines the `AppState` struct, which serves as the central
//! container for shared resources and configuration required by the JSON-RPC
//! method handlers. It ensures that components like configuration, database
//! access, subscription management, metrics, and rate limiting are accessible
//! in a thread-safe manner throughout the application.
//!
//! ## Design
//!
//! - `AppState` uses `Arc` for shared ownership of immutable or thread-safe
//!   components (like `JsonRpcConfig`, `DatabaseAdapter`, `MetricsCollector`,
//!   `SubscriptionManager`).
//! - `Arc<RwLock>` is used for components requiring mutable access across
//!   threads (like `SubscriptionManager`).
//! - It implements `Clone`, allowing cheap cloning for sharing across tasks or
//!   handlers.
//! - `Send + Sync` are implicitly satisfied due to the use of `Arc` and
//!   `RwLock`.
//!
//! ## Usage
//!
//! An instance of `AppState` is typically created during server initialization
//! and passed to the JSON-RPC framework (e.g., `jsonrpsee`) to be made
//! available within method handlers.

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

/// Central application state shared across JSON-RPC handlers.
///
/// Holds configuration and references to shared infrastructure components.
/// It is designed to be cloneable and thread-safe (`Send + Sync`).
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
        db_adapter: impl DatabaseAdapter + 'static,
        subscription_manager: SubscriptionManager,
        metrics_collector: MetricsCollector,
        manual_rate_limiters: ManualRateLimiters,
    ) -> Self {
        Self {
            config: Arc::new(config),
            db_adapter: Arc::new(db_adapter),
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
