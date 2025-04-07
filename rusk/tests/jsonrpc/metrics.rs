// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Tests for the JSON-RPC metrics infrastructure.

use metrics::{counter, gauge, histogram};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusk::jsonrpc::infrastructure::metrics::{
    init_metrics_recorder, register_jsonrpc_metrics,
    JSONRPC_ACTIVE_CONNECTIONS, JSONRPC_REQUESTS_TOTAL,
    JSONRPC_REQUEST_DURATION_SECONDS,
};
use std::time::Duration;

static METRICS_TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn ensure_metrics_initialized() {
    let _guard = METRICS_TEST_MUTEX.lock();
    match init_metrics_recorder() {
        Ok(_) => {}
        Err(
            metrics_exporter_prometheus::BuildError::FailedToSetGlobalRecorder(
                _,
            ),
        ) => {}
        Err(e) => {
            panic!("Unexpected error during metrics initialization: {:?}", e);
        }
    }
}

#[test]
fn test_init_metrics_recorder_idempotency() {
    let _guard = METRICS_TEST_MUTEX.lock();

    let _ = init_metrics_recorder();

    let result2 = init_metrics_recorder();

    assert!(
        matches!(
            result2,
            Err(metrics_exporter_prometheus::BuildError::FailedToSetGlobalRecorder(_))
        ),
        "Second call to init_metrics_recorder should always return FailedToSetGlobalRecorder, but got: {:?}",
        result2.err().map(|e| e.to_string())
    );
}

#[test]
fn test_register_jsonrpc_metrics_runs() {
    ensure_metrics_initialized();
    register_jsonrpc_metrics();
    register_jsonrpc_metrics();
}

#[test]
fn test_basic_metric_usage() {
    ensure_metrics_initialized();

    counter!(JSONRPC_REQUESTS_TOTAL, "method" => "test_method", "status" => "success")
        .increment(1);

    histogram!(JSONRPC_REQUEST_DURATION_SECONDS, "method" => "test_method")
        .record(Duration::from_millis(150).as_secs_f64());

    gauge!(JSONRPC_ACTIVE_CONNECTIONS).set(5.0);
}
