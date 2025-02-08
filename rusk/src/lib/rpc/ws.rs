use std::sync::Arc;
use axum::{routing::get, extract::ws::WebSocketUpgrade, Router, response::Response, Extension};
use yerpc::{RpcClient, RpcSession};
use yerpc::axum::handle_ws_rpc;
use crate::rpc::api::Api;

pub fn router() -> Router {
    Router::new().route("/ws", get(ws_handler))
}

async fn ws_handler(
    ws: WebSocketUpgrade, 
    Extension(api): Extension<Arc<Api>>
) -> Response {
    let (client, out_channel) = RpcClient::new();
    let session = RpcSession::new(client, (*api).clone());
    handle_ws_rpc(ws, out_channel, session).await
}
