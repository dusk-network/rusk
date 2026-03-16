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
use axum::extract::{FromRequestParts, State, ws::WebSocketUpgrade};
#[cfg(feature = "http-wasm")]
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, Request, Response, StatusCode};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::IntoResponse;
use axum::response::Json;
use axum::routing::any;
use serde_json::json;
use tokio::sync::{RwLock, broadcast, mpsc};
use tower::ServiceExt;
#[cfg(feature = "chain")]
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

#[cfg(feature = "chain")]
use super::MAX_GRAPHQL_REQUEST_BODY_BYTES;
use super::error::ApiError;
#[cfg(feature = "chain")]
use super::graphql;
use super::policy::HttpRequestPolicy;
use super::rues;
use super::rues::SubscriptionAction;
use super::{HandleRequest, RuesEvent, SessionId};

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
pub(super) struct HttpAppState {
    pub(super) sources: Arc<dyn HandleRequest>,
    pub(super) sockets_map:
        Arc<RwLock<HashMap<SessionId, mpsc::Sender<SubscriptionAction>>>>,
    pub(super) events: broadcast::Sender<RuesEvent>,
    pub(super) shutdown: broadcast::Sender<Infallible>,
    pub(super) ws_event_channel_cap: usize,
    pub(super) policy: Arc<HttpRequestPolicy>,
    pub(super) headers: Arc<HeaderMap>,
}

pub(super) fn build_app(state: HttpAppState) -> Router {
    let router = Router::new();
    let router = router
        .route("/on", any(rues_route))
        .route("/on/{*path}", any(rues_route));

    #[cfg(feature = "chain")]
    let router = router.route(
        "/graphql",
        any(graphql_route).layer(RequestBodyLimitLayer::new(
            MAX_GRAPHQL_REQUEST_BODY_BYTES,
        )),
    );

    #[cfg(feature = "http-wasm")]
    let router = router
        .route(WALLET_CORE_ALIAS_PATH, any(wallet_core_alias))
        .route(WALLET_CORE_1_0_1_PATH, any(wallet_core_1_0_1))
        .route(WALLET_CORE_1_3_0_PATH, any(wallet_core_1_3_0))
        .route(WALLET_CORE_1_6_0_PATH, any(wallet_core_1_6_0));

    router
        .fallback(not_found)
        .layer(from_fn_with_state(state.clone(), policy_middleware))
        .layer(from_fn_with_state(
            state.clone(),
            configured_headers_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn policy_middleware(
    State(state): State<HttpAppState>,
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    match state.policy.enforce(&req) {
        Ok(permit) => {
            let response = next.run(req).await;
            drop(permit);
            response
        }
        Err(rejection) => {
            let mut error =
                ApiError::new(rejection.status, rejection.message, "policy");
            if let Some(retry_after) = rejection.retry_after_seconds {
                error = error.with_retry_after(retry_after);
            }
            error.into_response()
        }
    }
}

async fn configured_headers_middleware(
    State(state): State<HttpAppState>,
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let mut response = next.run(req).await;
    response
        .headers_mut()
        .extend(state.headers.as_ref().clone());
    response
}

async fn rues_route(
    State(state): State<HttpAppState>,
    req: Request<Body>,
) -> Result<Response<Body>, ApiError> {
    let events = state.events.subscribe();
    let shutdown = state.shutdown.subscribe();

    let (mut parts, body) = req.into_parts();
    if let Ok(ws) = WebSocketUpgrade::from_request_parts(&mut parts, &()).await
    {
        return Ok(rues::handle_request_rues_ws(
            ws,
            state.sockets_map.clone(),
            events,
            shutdown,
            state.ws_event_channel_cap,
        )
        .await);
    }
    let req = Request::from_parts(parts, body);

    rues::handle_request_rues_http(req, state.sources, state.sockets_map).await
}

#[cfg(feature = "chain")]
async fn graphql_route(
    State(state): State<HttpAppState>,
    req: Request<Body>,
) -> Result<Response<Body>, ApiError> {
    let handler = match state.sources.graphql_handler() {
        Some(handler) => handler,
        None => {
            return Ok(graphql::handle_graphql_http_error(
                StatusCode::NOT_FOUND,
                "GraphQL endpoint not configured",
            )
            .expect("GraphQL error response should be built"));
        }
    };

    graphql::handle_graphql_http(handler, req)
        .await
        .map_err(ApiError::from)
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
    use std::collections::HashMap;
    use std::convert::Infallible;
    use std::sync::Arc;
    use tokio::sync::{RwLock, broadcast, mpsc};
    use tower::ServiceExt;

    use super::rues::SubscriptionAction;
    use super::{HttpAppState, HttpRequestPolicy, build_app};
    use crate::http::{HandleRequest, HttpPolicyConfig};
    use crate::http::{
        HttpResult, ResponseData, RuesDispatchEvent, RuesEvent, SessionId,
    };

    struct NoopHandle;

    #[async_trait::async_trait]
    impl HandleRequest for NoopHandle {
        fn can_handle_rues(&self, _request: &RuesDispatchEvent) -> bool {
            false
        }

        async fn handle_rues(
            &self,
            _request: &RuesDispatchEvent,
        ) -> HttpResult<ResponseData> {
            Err(crate::http::HttpError::Unsupported)
        }
    }

    fn test_state() -> HttpAppState {
        let (events_tx, _events_rx) = broadcast::channel::<RuesEvent>(1);
        let (shutdown_tx, _shutdown_rx) = broadcast::channel::<Infallible>(1);
        HttpAppState {
            sources: Arc::new(NoopHandle),
            sockets_map: Arc::new(RwLock::new(HashMap::<
                SessionId,
                mpsc::Sender<SubscriptionAction>,
            >::new())),
            events: events_tx,
            shutdown: shutdown_tx,
            ws_event_channel_cap: 1,
            policy: Arc::new(HttpRequestPolicy::new(
                HttpPolicyConfig::default(),
            )),
            headers: Arc::new(axum::http::HeaderMap::new()),
        }
    }

    #[tokio::test]
    async fn router_fallback_returns_json_not_found() {
        let app = build_app(test_state());
        let response = app
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
