// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # JSON-RPC Metrics Collection
//!
//! This module provides the infrastructure for collecting and exposing metrics
//! related to the JSON-RPC server's operation. It uses the `metrics` crate
//! for defining and recording metrics and `metrics-exporter-prometheus` for
//! exposing them in a Prometheus-compatible format.
//!
//! ## Core Components
//!
//! - **`MetricsCollector`**: A struct (currently a placeholder) that might
//!   later hold handles or configurations related to metrics collection.
//! - **Metric Constants**: Defines standardized keys for JSON-RPC related
//!   metrics.
//! - **Registration Functions**: Functions to register the defined metrics with
//!   the global `metrics` registry.
//! - **Initialization**: A function (`init_metrics_recorder`) to set up the
//!   Prometheus exporter and install it as the global recorder.
//!
//! ## Usage
//!
//! 1. Call `init_metrics_recorder` once during application startup to set up
//!    the metrics system.
//! 2. Store the returned `PrometheusHandle` (likely within `AppState`) to
//!    render the metrics endpoint later.
//! 3. Use the `metrics` crate macros (e.g., `metrics::increment_counter!`,
//!    `metrics::histogram!`) throughout the JSON-RPC request handling logic,
//!    using the constants defined in this module (e.g.,
//!    `JSONRPC_REQUESTS_TOTAL`).
//!
//! ## Metrics Defined
//!
//! - `jsonrpc_requests_total` (Counter): Total number of JSON-RPC requests
//!   processed, labeled by `method` and `status` (`success`/`error`).
//! - `jsonrpc_request_duration_seconds` (Histogram): Latency of JSON-RPC
//!   requests, labeled by `method`.
//! - `jsonrpc_active_connections` (Gauge): Number of currently active HTTP/WS
//!   connections. (Registration only, update logic is external).
//! - `jsonrpc_active_subscriptions` (Gauge): Number of active WebSocket
//!   subscriptions. (Registration only, update logic is external).

use metrics::{describe_counter, describe_gauge, describe_histogram, Unit};
use metrics_exporter_prometheus::{
    BuildError,
    PrometheusBuilder,
    PrometheusHandle, // Removed PrometheusRecorder import
};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use tracing::info;

// --- Metric Keys ---

/// Counter for total number of JSON-RPC requests processed.
/// Labels: `method`, `status` (`success`/`error`).
pub const JSONRPC_REQUESTS_TOTAL: &str = "jsonrpc_requests_total";

/// Histogram for JSON-RPC request duration in seconds.
/// Labels: `method`.
pub const JSONRPC_REQUEST_DURATION_SECONDS: &str =
    "jsonrpc_request_duration_seconds";

/// Gauge for the number of currently active HTTP/WebSocket connections.
/// Labels: None.
pub const JSONRPC_ACTIVE_CONNECTIONS: &str = "jsonrpc_active_connections";

/// Gauge for the number of currently active WebSocket subscriptions.
/// Labels: None.
pub const JSONRPC_ACTIVE_SUBSCRIPTIONS: &str = "jsonrpc_active_subscriptions";

// --- Metrics Collector Struct ---

/// Manages metrics collection state and potentially holds handles.
///
/// Currently, this struct is primarily a placeholder. It might later hold
/// the `PrometheusHandle` or other metrics-related state if needed for more
/// complex scenarios. It needs to be `Clone` and `Debug` to be stored in
/// `AppState`.
#[derive(Debug, Clone, Default)]
pub struct MetricsCollector {}

// --- Metric Registration ---

/// Registers all core JSON-RPC metrics with the `metrics` crate.
///
/// This function should be called once during initialization, typically after
/// the metrics recorder has been installed. It uses `describe_` functions
/// to add descriptions and units to the metrics.
pub fn register_jsonrpc_metrics() {
    info!("Registering JSON-RPC metrics");

    describe_counter!(
        JSONRPC_REQUESTS_TOTAL,
        Unit::Count,
        "Total number of JSON-RPC requests processed"
    );

    describe_histogram!(
        JSONRPC_REQUEST_DURATION_SECONDS,
        Unit::Seconds,
        "Histogram of JSON-RPC request latency in seconds"
    );

    describe_gauge!(
        JSONRPC_ACTIVE_CONNECTIONS,
        Unit::Count,
        "Number of currently active HTTP/WebSocket connections"
    );
    // Initialize gauge to 0
    // Registration is implicit on first use/describe
    metrics::gauge!(JSONRPC_ACTIVE_CONNECTIONS).set(0.0);

    describe_gauge!(
        JSONRPC_ACTIVE_SUBSCRIPTIONS,
        Unit::Count,
        "Number of currently active WebSocket subscriptions"
    );
    metrics::gauge!(JSONRPC_ACTIVE_SUBSCRIPTIONS).set(0.0);

    info!("JSON-RPC metrics registered");
}

// --- Initialization Guard ---

static METRICS_INITIALIZED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

/// Initializes the metrics system with a Prometheus recorder.
///
/// This function sets up the `metrics-exporter-prometheus` recorder and
/// installs it as the global recorder for the `metrics` facade. It ensures that
/// this initialization happens only once using a mutex guard. After successful
/// installation, it registers the core JSON-RPC metrics.
///
/// # Returns
///
/// * `Ok(PrometheusHandle)` containing the handle to render the Prometheus
///   metrics page.
/// * `Err(BuildError)` if building or installing the recorder fails (e.g., if
///   already installed).
///
/// # Panics
///
/// This function might panic if the mutex protecting the initialization flag
/// is poisoned, which indicates a bug in concurrent access during
/// initialization.
pub fn init_metrics_recorder() -> Result<PrometheusHandle, BuildError> {
    let mut initialized = METRICS_INITIALIZED.lock();

    if *initialized {
        info!("Metrics recorder already initialized.");
        let dummy_prometheus_recorder =
            PrometheusBuilder::new().build_recorder();
        return Err(BuildError::FailedToSetGlobalRecorder(
            metrics::SetRecorderError(dummy_prometheus_recorder),
        ));
    }

    info!("Initializing Prometheus metrics recorder.");
    let builder = PrometheusBuilder::new();

    let handle = builder.install_recorder()?;
    register_jsonrpc_metrics();

    *initialized = true;
    info!("Prometheus metrics recorder installed and metrics registered.");

    Ok(handle)
}
