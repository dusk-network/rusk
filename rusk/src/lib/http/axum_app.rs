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
use axum::extract::State;
#[cfg(feature = "http-wasm")]
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, Request, Response, StatusCode};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Json};
#[cfg(any(feature = "chain", feature = "http-wasm"))]
use axum::routing::get;
use axum::routing::{any, post};
use serde_json::json;
use tokio::sync::{RwLock, broadcast, mpsc};
#[cfg(feature = "chain")]
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

#[cfg(feature = "chain")]
use super::MAX_GRAPHQL_REQUEST_BODY_BYTES;
use super::error::ApiError;
#[cfg(feature = "chain")]
use super::graphql;
use super::policy::HttpRequestPolicy;
use super::rues::SubscriptionAction;
use super::{HandleRequest, RuesEvent, SessionId, rues};

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
    let router = router.route("/on", any(rues::handle_rues_ws)).route(
        "/on/{*path}",
        post(rues::handle_rues_post)
            .get(rues::handle_rues_subscribe)
            .delete(rues::handle_rues_unsubscribe),
    );

    #[cfg(feature = "chain")]
    let router = router.route(
        "/graphql",
        get(graphql_get_route)
            .post(graphql_post_route)
            .layer(RequestBodyLimitLayer::new(MAX_GRAPHQL_REQUEST_BODY_BYTES)),
    );

    #[cfg(feature = "http-wasm")]
    let router = router.route("/static/drivers/{file}", get(wasm_driver_route));

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

#[cfg(feature = "chain")]
async fn graphql_get_route(
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

    graphql::handle_graphql_get(handler, req)
        .await
        .map_err(ApiError::from)
}

#[cfg(feature = "chain")]
async fn graphql_post_route(
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

    graphql::handle_graphql_post(handler, req)
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
async fn wasm_driver_route(
    axum::extract::Path(file): axum::extract::Path<String>,
) -> Response<Body> {
    let bytes: Option<&'static [u8]> = match file.as_str() {
        "wallet-core.wasm" | "wallet-core-1.0.1.wasm" => {
            Some(include_bytes!("../../assets/wallet_core-1.0.1.wasm"))
        }
        "wallet-core-1.3.0.wasm" => {
            Some(include_bytes!("../../assets/wallet_core-1.3.0.wasm"))
        }
        "wallet-core-1.6.0.wasm" => {
            Some(include_bytes!("../../assets/wallet_core-1.6.0.wasm"))
        }
        _ => None,
    };

    match bytes {
        Some(wasm) => (
            StatusCode::OK,
            [
                (CONTENT_TYPE, WASM_CONTENT_TYPE),
                (CACHE_CONTROL, WASM_CACHE_CONTROL),
            ],
            wasm,
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Path not found" })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::convert::Infallible;
    use std::sync::Arc;

    use axum::body::{Body, to_bytes};
    use axum::http::header::CONTENT_TYPE;
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use tokio::sync::{RwLock, broadcast, mpsc};
    use tower::ServiceExt;

    use super::rues::SubscriptionAction;
    use super::{HttpAppState, HttpRequestPolicy, build_app};
    use crate::http::{
        HandleRequest, HttpPolicyConfig, HttpResult, ResponseData,
        RuesDispatchEvent, RuesEvent, SessionId,
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
