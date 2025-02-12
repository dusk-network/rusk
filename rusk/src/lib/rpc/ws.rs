// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::rpc::api::Api;
use axum::{
    extract::ws::WebSocketUpgrade, response::Response, routing::get, Extension,
    Router,
};
use std::sync::Arc;
use yerpc::axum::handle_ws_rpc;
use yerpc::{RpcClient, RpcSession};

pub fn router() -> Router {
    Router::new().route("/ws", get(ws_handler))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(api): Extension<Arc<Api>>,
) -> Response {
    let (client, out_channel) = RpcClient::new();
    let session = RpcSession::new(client, (*api).clone());
    handle_ws_rpc(ws, out_channel, session).await
}
