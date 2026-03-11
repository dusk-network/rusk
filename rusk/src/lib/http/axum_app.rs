// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::{FromRequestParts, ws::WebSocketUpgrade};
use axum::http::StatusCode;
#[cfg(feature = "http-wasm")]
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{Request, Response};
use axum::response::IntoResponse;
use axum::response::Json;
use axum::routing::any;
#[cfg(feature = "chain")]
use hyper::StatusCode as HyperStatusCode;
use hyper::body::Incoming;
use serde_json::json;
use tokio::sync::{Mutex, RwLock, broadcast, mpsc};
use tower::ServiceExt;

use super::SubscriptionAction;
use super::error::map_execution_error;
use super::event::FullOrStreamBody;
#[cfg(feature = "chain")]
use super::graphql;
use super::responses;
use super::rues;
use super::{ExecutionError, HandleRequest, RuesEvent, SessionId};

#[cfg(feature = "http-wasm")]
const WALLET_CORE_ALIAS_PATH: &str = "/static/drivers/wallet-core.wasm";
#[cfg(feature = "http-wasm")]
const WALLET_CORE_1_0_1_PATH: &str = "/static/drivers/wallet-core-1.0.1.wasm";
#[cfg(feature = "http-wasm")]
const WALLET_CORE_1_3_0_PATH: &str = "/static/drivers/wallet-core-1.3.0.wasm";
#[cfg(feature = "http-wasm")]
const WALLET_CORE_1_6_0_PATH: &str = "/static/drivers/wallet-core-1.6.0.wasm";

#[cfg(feature = "http-wasm")]
const WASM_CONTENT_TYPE: &str = "application/wasm";
#[cfg(feature = "http-wasm")]
const WASM_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

#[derive(Clone)]
pub(super) struct AxumRequestContext {
    pub(super) sources: Arc<dyn HandleRequest>,
    pub(super) sockets_map:
        Arc<RwLock<HashMap<SessionId, mpsc::Sender<SubscriptionAction>>>>,
    pub(super) events: Arc<Mutex<broadcast::Receiver<RuesEvent>>>,
    pub(super) shutdown: Arc<Mutex<broadcast::Receiver<Infallible>>>,
    pub(super) ws_event_channel_cap: usize,
}

#[derive(Clone)]
pub(super) struct RouteDispatchPlan {
    router: Router,
}

impl RouteDispatchPlan {
    pub(super) fn new() -> Self {
        let router = build_router();
        Self { router }
    }

    pub(super) async fn handle_axum(
        &self,
        req: Request<Incoming>,
        context: AxumRequestContext,
    ) -> Result<Response<Body>, ExecutionError> {
        let mut req = req.map(Body::new);
        req.extensions_mut().insert(context);
        Ok(match self.router.clone().oneshot(req).await {
            Ok(response) => response,
            Err(err) => match err {},
        })
    }
}

impl Default for RouteDispatchPlan {
    fn default() -> Self {
        Self::new()
    }
}

fn build_router() -> Router {
    let router = Router::new();
    let router = router
        .route("/on", any(rues_route))
        .route("/on/{*path}", any(rues_route));

    #[cfg(feature = "chain")]
    let router = router
        .route("/graphql", any(graphql_route))
        .route("/graphql/", any(graphql_route));

    #[cfg(feature = "http-wasm")]
    let router = router
        .route(WALLET_CORE_ALIAS_PATH, any(wallet_core_alias))
        .route(WALLET_CORE_1_0_1_PATH, any(wallet_core_1_0_1))
        .route(WALLET_CORE_1_3_0_PATH, any(wallet_core_1_3_0))
        .route(WALLET_CORE_1_6_0_PATH, any(wallet_core_1_6_0));

    router.fallback(not_found)
}

fn execution_error_to_axum_response(error: &ExecutionError) -> Response<Body> {
    let (status, message, _category) = map_execution_error(error);
    let response = responses::api_error_response(status, message)
        .expect("Failed to build execution error response");
    full_or_stream_to_axum(response)
}

fn full_or_stream_to_axum(
    response: Response<FullOrStreamBody>,
) -> Response<Body> {
    let (parts, body) = response.into_parts();
    Response::from_parts(parts, Body::new(body))
}

async fn rues_route(mut req: Request<Body>) -> Response<Body> {
    let context = req
        .extensions_mut()
        .remove::<AxumRequestContext>()
        .expect("axum request context should be present");

    let events = context.events.lock().await.resubscribe();
    let shutdown = context.shutdown.lock().await.resubscribe();

    let (mut parts, body) = req.into_parts();
    if let Ok(ws) = WebSocketUpgrade::from_request_parts(&mut parts, &()).await
    {
        return rues::handle_request_rues_ws(
            ws,
            context.sockets_map,
            events,
            shutdown,
            context.ws_event_channel_cap,
        )
        .await;
    }
    let req = Request::from_parts(parts, body);

    match rues::handle_request_rues_http(
        req,
        context.sources,
        context.sockets_map,
    )
    .await
    {
        Ok(response) => response,
        Err(error) => execution_error_to_axum_response(&error),
    }
}

#[cfg(feature = "chain")]
async fn graphql_route(mut req: Request<Body>) -> Response<Body> {
    let context = req
        .extensions_mut()
        .remove::<AxumRequestContext>()
        .expect("axum request context should be present");

    let handler = match context.sources.graphql_handler() {
        Some(handler) => handler,
        None => {
            return graphql::handle_graphql_http_error(
                HyperStatusCode::NOT_FOUND,
                "GraphQL endpoint not configured",
            )
            .expect("GraphQL error response should be built");
        }
    };

    match graphql::handle_graphql_http(handler, req).await {
        Ok(response) => response,
        Err(error) => execution_error_to_axum_response(&error),
    }
}

async fn not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": "Path not found" })),
    )
}

#[cfg(feature = "http-wasm")]
async fn wallet_core_alias() -> impl IntoResponse {
    wasm_response(include_bytes!("../../assets/wallet_core-1.0.1.wasm"))
}

#[cfg(feature = "http-wasm")]
async fn wallet_core_1_0_1() -> impl IntoResponse {
    wasm_response(include_bytes!("../../assets/wallet_core-1.0.1.wasm"))
}

#[cfg(feature = "http-wasm")]
async fn wallet_core_1_3_0() -> impl IntoResponse {
    wasm_response(include_bytes!("../../assets/wallet_core-1.3.0.wasm"))
}

#[cfg(feature = "http-wasm")]
async fn wallet_core_1_6_0() -> impl IntoResponse {
    wasm_response(include_bytes!("../../assets/wallet_core-1.6.0.wasm"))
}

#[cfg(feature = "http-wasm")]
fn wasm_response(bytes: &'static [u8]) -> impl IntoResponse {
    (
        StatusCode::OK,
        [
            (CONTENT_TYPE, WASM_CONTENT_TYPE),
            (CACHE_CONTROL, WASM_CACHE_CONTROL),
        ],
        bytes,
    )
}

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::http::header::CONTENT_TYPE;
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use tower::ServiceExt;

    use super::RouteDispatchPlan;

    #[tokio::test]
    async fn router_fallback_returns_json_not_found() {
        let dispatch = RouteDispatchPlan::new();
        let response = dispatch
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/unsupported")
                    .body(Body::empty())
                    .expect("Request should be built"),
            )
            .await
            .expect("Fallback response should be produced");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        assert_eq!(content_type, "application/json");

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Body should be readable");
        let payload: Value =
            serde_json::from_slice(&body).expect("Body should be JSON");
        assert_eq!(payload["error"], "Path not found");
    }
}
