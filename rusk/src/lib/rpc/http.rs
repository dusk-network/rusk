// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::rpc::api::Api;
use axum::{response::IntoResponse, routing::post, Extension, Json, Router};
use serde_json::Value;
use std::sync::Arc;

pub fn router() -> Router {
    Router::new().route("/rpc", post(handle_request))
}

/// Matches on the called JSON-RPC method
async fn handle_request(
    Extension(api): Extension<Arc<Api>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let answer_id = payload.get("id").cloned().unwrap_or(Value::Null);
    let method = payload.get("method").and_then(|m| m.as_str()).unwrap_or("");

    let response = match method {
        "get_block_height" => handle_rpc_call(api.get_block_height().await),

        "get_account" => {
            let address = extract_param(&payload, "address");
            handle_rpc_call(api.get_account(address).await)
        }

        "get_balance" => {
            let address = extract_param(&payload, "address");
            handle_rpc_call(api.get_balance(address).await)
        }

        "get_block" => {
            let block_hash = extract_param(&payload, "block_hash");
            handle_rpc_call(api.get_block(block_hash).await)
        }

        "get_transaction" => {
            let tx_hash = extract_param(&payload, "tx_hash");
            handle_rpc_call(api.get_transaction(tx_hash).await)
        }

        _ => json_rpc_error(-32601, "Method not found", answer_id.clone()),
    };

    Json(serde_json::json!({
        "jsonrpc": "2.0",
        "result": response,
        "id": answer_id
    }))
}

/// Extracts a parameter from the JSON payload
fn extract_param(payload: &Value, key: &str) -> String {
    payload
        .get("params")
        .and_then(|p| p.get(key))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Handles successful and error responses
fn handle_rpc_call<T: serde::Serialize>(
    result: Result<T, anyhow::Error>,
) -> Value {
    match result {
        Ok(data) => serde_json::to_value(data).unwrap(),
        Err(e) => json_rpc_error(-32000, &e.to_string(), Value::Null),
    }
}

/// Returns a formatted error response
fn json_rpc_error(code: i32, message: &str, id: Value) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "error": {
            "code": code,
            "message": message
        },
        "id": id
    })
}
