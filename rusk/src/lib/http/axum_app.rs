// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(
    not(any(feature = "chain", feature = "prover", test)),
    allow(dead_code)
)]

use std::collections::HashMap;
use std::convert::Infallible;
#[cfg(any(feature = "chain", feature = "prover", test))]
use std::future::Future;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
#[cfg(any(feature = "chain", feature = "prover", test))]
use axum::body::Bytes;
#[cfg(any(feature = "chain", feature = "prover", test))]
use axum::extract::Path;
use axum::extract::State;
#[cfg(feature = "http-wasm")]
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, Request, Response, StatusCode};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Json};
use axum::routing::get;
#[cfg(any(feature = "chain", feature = "prover", test))]
use axum::routing::post;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::{RwLock, broadcast, mpsc};
#[cfg(any(feature = "chain", feature = "prover", test))]
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

#[cfg(feature = "chain")]
use super::MAX_GRAPHQL_REQUEST_BODY_BYTES;
use super::error::ApiError;
#[cfg(feature = "chain")]
use super::graphql;
use super::policy::HttpRequestPolicy;
use super::rues::SubscriptionAction;
#[cfg(any(feature = "chain", feature = "prover", test))]
use super::{HttpError, ResponseData};
use super::{HttpHandlers, RuesEvent, RuesEventUri, SessionId, rues};

#[cfg(feature = "http-wasm")]
const WASM_CONTENT_TYPE: &str = "application/wasm";
#[cfg(feature = "http-wasm")]
const WASM_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

#[derive(Clone)]
pub(super) struct HttpAppState {
    pub(super) services: HttpHandlers,
    pub(super) sockets_map:
        Arc<RwLock<HashMap<SessionId, mpsc::Sender<SubscriptionAction>>>>,
    pub(super) events: broadcast::Sender<RuesEvent>,
    pub(super) shutdown: broadcast::Sender<Infallible>,
    pub(super) ws_event_channel_cap: usize,
    pub(super) policy: Arc<HttpRequestPolicy>,
    pub(super) headers: Arc<HeaderMap>,
}

pub(super) fn build_app(state: HttpAppState) -> Router {
    let router = Router::new()
        .route("/on", get(rues::handle_rues_ws))
        .nest("/on", rues_http_router());

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

#[derive(Deserialize)]
struct TopicPath {
    topic: String,
}

#[derive(Deserialize)]
#[cfg(feature = "chain")]
struct EntityTopicPath {
    entity: String,
    topic: String,
}

fn invalid_rues_path_error() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, "Invalid URL path", "invalid_path")
}

fn component_uri(
    component: &'static str,
    topic: &str,
) -> Result<RuesEventUri, ApiError> {
    RuesEventUri::from_parts(component, None, topic)
        .ok_or_else(invalid_rues_path_error)
}

#[cfg(feature = "chain")]
fn entity_uri(
    component: &'static str,
    entity: &str,
    topic: &str,
) -> Result<RuesEventUri, ApiError> {
    RuesEventUri::from_parts(component, Some(entity.to_string()), topic)
        .ok_or_else(invalid_rues_path_error)
}

#[cfg(any(feature = "chain", feature = "prover", test))]
async fn run_rues_post<F, Fut>(
    uri: Result<RuesEventUri, ApiError>,
    headers: HeaderMap,
    body: Bytes,
    dispatch: F,
) -> Result<Response<Body>, ApiError>
where
    F: FnOnce(super::RuesDispatchEvent, bool) -> Fut,
    Fut: Future<
            Output = (
                super::RuesDispatchEvent,
                bool,
                Result<ResponseData, HttpError>,
            ),
        > + Send,
{
    rues::validate_rusk_version_headers(&headers)?;
    let (event, binary_request) = rues::parse_rues_post(uri?, headers, body)?;
    let (event, binary_request, result) = dispatch(event, binary_request).await;
    rues::finish_rues_post(event, binary_request, result)
}

#[cfg(any(feature = "chain", test))]
async fn run_subscribe(
    uri: Result<RuesEventUri, ApiError>,
    headers: HeaderMap,
    session_id: SessionId,
    sockets_map: Arc<
        RwLock<HashMap<SessionId, mpsc::Sender<SubscriptionAction>>>,
    >,
) -> Result<Response<Body>, ApiError> {
    rues::validate_rusk_version_headers(&headers)?;
    rues::dispatch_rues_subscribe(session_id, sockets_map, uri?).await
}

#[cfg(any(feature = "chain", test))]
async fn run_unsubscribe(
    uri: Result<RuesEventUri, ApiError>,
    headers: HeaderMap,
    session_id: SessionId,
    sockets_map: Arc<
        RwLock<HashMap<SessionId, mpsc::Sender<SubscriptionAction>>>,
    >,
) -> Result<Response<Body>, ApiError> {
    rues::validate_rusk_version_headers(&headers)?;
    rues::dispatch_rues_unsubscribe(session_id, sockets_map, uri?).await
}

fn rues_http_router() -> Router<HttpAppState> {
    let router = Router::new();

    #[cfg(feature = "chain")]
    let router = router
        .route(
            "/transactions/propagate",
            post(transactions_propagate_post).layer(
                RequestBodyLimitLayer::new(super::MAX_RUES_REQUEST_BODY_BYTES),
            ),
        )
        .route(
            "/transactions/{topic}",
            post(transactions_post)
                .get(transactions_subscribe)
                .delete(transactions_unsubscribe)
                .layer(RequestBodyLimitLayer::new(
                    super::MAX_RUES_REQUEST_BODY_BYTES,
                )),
        )
        .route(
            "/network/{topic}",
            post(network_post).layer(RequestBodyLimitLayer::new(
                super::MAX_RUES_REQUEST_BODY_BYTES,
            )),
        )
        .route(
            "/node/{topic}",
            post(node_post).layer(RequestBodyLimitLayer::new(
                super::MAX_RUES_REQUEST_BODY_BYTES,
            )),
        )
        .route(
            "/blocks/{topic}",
            post(blocks_post)
                .get(blocks_subscribe)
                .delete(blocks_unsubscribe)
                .layer(RequestBodyLimitLayer::new(
                    super::MAX_RUES_REQUEST_BODY_BYTES,
                )),
        )
        .route(
            "/stats/{topic}",
            post(stats_post).layer(RequestBodyLimitLayer::new(
                super::MAX_RUES_REQUEST_BODY_BYTES,
            )),
        )
        .route(
            "/account:{entity}/{topic}",
            post(account_post).layer(RequestBodyLimitLayer::new(
                super::MAX_RUES_REQUEST_BODY_BYTES,
            )),
        )
        .route(
            "/blobs:{entity}/{topic}",
            post(blobs_post).layer(RequestBodyLimitLayer::new(
                super::MAX_RUES_REQUEST_BODY_BYTES,
            )),
        )
        .route(
            "/contracts:{entity}/{topic}",
            post(contracts_post)
                .get(contracts_subscribe)
                .delete(contracts_unsubscribe)
                .layer(RequestBodyLimitLayer::new(
                    super::MAX_RUES_REQUEST_BODY_BYTES,
                )),
        )
        .route(
            "/driver:{entity}/{topic}",
            post(driver_post).layer(RequestBodyLimitLayer::new(
                super::MAX_RUES_REQUEST_BODY_BYTES,
            )),
        )
        .route(
            "/contract_owner:{entity}/{topic}",
            post(contract_owner_post).layer(RequestBodyLimitLayer::new(
                super::MAX_RUES_REQUEST_BODY_BYTES,
            )),
        )
        .route(
            "/contract:{entity}/upload_driver",
            post(contract_upload_driver_post).layer(
                RequestBodyLimitLayer::new(super::MAX_DRIVER_UPLOAD_BODY_BYTES),
            ),
        )
        .route(
            "/contract:{entity}/{topic}",
            post(contract_post).layer(RequestBodyLimitLayer::new(
                super::MAX_RUES_REQUEST_BODY_BYTES,
            )),
        );

    #[cfg(feature = "prover")]
    let router = router.route(
        "/prover/{topic}",
        post(prover_post).layer(RequestBodyLimitLayer::new(
            super::MAX_RUES_REQUEST_BODY_BYTES,
        )),
    );

    #[cfg(feature = "chain")]
    let router = router.route(
        "/graphql/query",
        post(legacy_rues_graphql_post_route)
            .layer(RequestBodyLimitLayer::new(
                super::MAX_RUES_REQUEST_BODY_BYTES,
            )),
    );

    #[cfg(test)]
    let router = router.route(
        "/test/{topic}",
        post(test_post)
            .get(test_subscribe)
            .delete(test_unsubscribe)
            .layer(RequestBodyLimitLayer::new(
                super::MAX_RUES_REQUEST_BODY_BYTES,
            )),
    );

    router
}
async fn transactions_propagate_post(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_rues_post(
        component_uri("transactions", "propagate"),
        headers,
        body,
        |event, binary_request| async move {
            let result = match state.services.chain_handler() {
                Some(chain) => chain.transactions("propagate", &event).await,
                None => Err(super::HttpError::Unsupported),
            };
            (event, binary_request, result)
        },
    )
    .await
}

#[cfg(feature = "chain")]
async fn transactions_post(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_rues_post(
        component_uri("transactions", &topic),
        headers,
        body,
        |event, binary_request| async move {
            let result = match state.services.chain_handler() {
                Some(chain) => chain.transactions(&topic, &event).await,
                None => Err(super::HttpError::Unsupported),
            };
            (event, binary_request, result)
        },
    )
    .await
}

#[cfg(feature = "chain")]
async fn transactions_subscribe(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    session_id: SessionId,
) -> Result<Response<Body>, ApiError> {
    run_subscribe(
        component_uri("transactions", &topic),
        headers,
        session_id,
        state.sockets_map,
    )
    .await
}

#[cfg(feature = "chain")]
async fn transactions_unsubscribe(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    session_id: SessionId,
) -> Result<Response<Body>, ApiError> {
    run_unsubscribe(
        component_uri("transactions", &topic),
        headers,
        session_id,
        state.sockets_map,
    )
    .await
}

#[cfg(feature = "chain")]
async fn network_post(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_rues_post(
        component_uri("network", &topic),
        headers,
        body,
        |event, binary_request| async move {
            let result = match state.services.chain_handler() {
                Some(chain) => chain.network(&topic, &event).await,
                None => Err(super::HttpError::Unsupported),
            };
            (event, binary_request, result)
        },
    )
    .await
}

#[cfg(feature = "chain")]
async fn node_post(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_rues_post(
        component_uri("node", &topic),
        headers,
        body,
        |event, binary_request| async move {
            let result = match topic.as_str() {
                "info" => match state.services.chain_handler() {
                    Some(chain) => chain.node(&topic, &event).await,
                    None => Err(super::HttpError::Unsupported),
                },
                "provisioners" | "crs" => match state.services.rusk_handler() {
                    Some(rusk) => rusk.node(&topic, &event).await,
                    None => Err(super::HttpError::Unsupported),
                },
                _ => Err(super::HttpError::Unsupported),
            };
            (event, binary_request, result)
        },
    )
    .await
}

#[cfg(feature = "chain")]
async fn blocks_post(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_rues_post(
        component_uri("blocks", &topic),
        headers,
        body,
        |event, binary_request| async move {
            let result = match state.services.chain_handler() {
                Some(chain) => chain.blocks(&topic, &event).await,
                None => Err(super::HttpError::Unsupported),
            };
            (event, binary_request, result)
        },
    )
    .await
}

#[cfg(feature = "chain")]
async fn blocks_subscribe(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    session_id: SessionId,
) -> Result<Response<Body>, ApiError> {
    run_subscribe(
        component_uri("blocks", &topic),
        headers,
        session_id,
        state.sockets_map,
    )
    .await
}

#[cfg(feature = "chain")]
async fn blocks_unsubscribe(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    session_id: SessionId,
) -> Result<Response<Body>, ApiError> {
    run_unsubscribe(
        component_uri("blocks", &topic),
        headers,
        session_id,
        state.sockets_map,
    )
    .await
}

#[cfg(feature = "chain")]
async fn stats_post(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_rues_post(
        component_uri("stats", &topic),
        headers,
        body,
        |event, binary_request| async move {
            let result = match state.services.chain_handler() {
                Some(chain) => chain.stats(&topic, &event).await,
                None => Err(super::HttpError::Unsupported),
            };
            (event, binary_request, result)
        },
    )
    .await
}

#[cfg(feature = "prover")]
async fn prover_post(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_rues_post(
        component_uri("prover", &topic),
        headers,
        body,
        |event, binary_request| async move {
            let result = match (topic.as_str(), state.services.prover_handler())
            {
                ("prove", Some(prover)) => prover.prove(&event).await,
                _ => Err(super::HttpError::Unsupported),
            };
            (event, binary_request, result)
        },
    )
    .await
}

#[cfg(feature = "chain")]
async fn account_post(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_rues_post(
        entity_uri("account", &entity, &topic),
        headers,
        body,
        |event, binary_request| async move {
            let result = match state.services.chain_handler() {
                Some(chain) => chain.account(&entity, &topic, &event).await,
                None => Err(super::HttpError::Unsupported),
            };
            (event, binary_request, result)
        },
    )
    .await
}

#[cfg(feature = "chain")]
async fn blobs_post(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_rues_post(
        entity_uri("blobs", &entity, &topic),
        headers,
        body,
        |event, binary_request| async move {
            let result = match state.services.chain_handler() {
                Some(chain) => chain.blobs(&entity, &topic, &event).await,
                None => Err(super::HttpError::Unsupported),
            };
            (event, binary_request, result)
        },
    )
    .await
}

#[cfg(feature = "chain")]
async fn contracts_post(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_rues_post(
        entity_uri("contracts", &entity, &topic),
        headers,
        body,
        |event, binary_request| async move {
            let result = match state.services.rusk_handler() {
                Some(rusk) => rusk.contracts(&entity, &topic, &event).await,
                None => Err(super::HttpError::Unsupported),
            };
            (event, binary_request, result)
        },
    )
    .await
}

#[cfg(feature = "chain")]
async fn contracts_subscribe(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    headers: HeaderMap,
    session_id: SessionId,
) -> Result<Response<Body>, ApiError> {
    run_subscribe(
        entity_uri("contracts", &entity, &topic),
        headers,
        session_id,
        state.sockets_map,
    )
    .await
}

#[cfg(feature = "chain")]
async fn contracts_unsubscribe(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    headers: HeaderMap,
    session_id: SessionId,
) -> Result<Response<Body>, ApiError> {
    run_unsubscribe(
        entity_uri("contracts", &entity, &topic),
        headers,
        session_id,
        state.sockets_map,
    )
    .await
}

#[cfg(feature = "chain")]
async fn driver_post(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_rues_post(
        entity_uri("driver", &entity, &topic),
        headers,
        body,
        |event, binary_request| async move {
            let result = match state.services.rusk_handler() {
                Some(rusk) => rusk.driver(&entity, &topic, &event).await,
                None => Err(super::HttpError::Unsupported),
            };
            (event, binary_request, result)
        },
    )
    .await
}

#[cfg(feature = "chain")]
async fn contract_owner_post(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_rues_post(
        entity_uri("contract_owner", &entity, &topic),
        headers,
        body,
        |event, binary_request| async move {
            let result = match state.services.rusk_handler() {
                Some(rusk) => {
                    rusk.contract_owner(&entity, &topic, &event).await
                }
                None => Err(super::HttpError::Unsupported),
            };
            (event, binary_request, result)
        },
    )
    .await
}

#[cfg(feature = "chain")]
async fn contract_upload_driver_post(
    State(state): State<HttpAppState>,
    Path(entity): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    let topic = "upload_driver".to_string();
    run_rues_post(
        entity_uri("contract", &entity, &topic),
        headers,
        body,
        |event, binary_request| async move {
            let result = match state.services.rusk_handler() {
                Some(rusk) => rusk.contract(&entity, &topic, &event).await,
                None => Err(super::HttpError::Unsupported),
            };
            (event, binary_request, result)
        },
    )
    .await
}

#[cfg(feature = "chain")]
async fn contract_post(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_rues_post(
        entity_uri("contract", &entity, &topic),
        headers,
        body,
        |event, binary_request| async move {
            let result = match topic.as_str() {
                "status" => match state.services.chain_handler() {
                    Some(chain) => {
                        chain.contract(&entity, &topic, &event).await
                    }
                    None => Err(super::HttpError::Unsupported),
                },
                _ => match state.services.rusk_handler() {
                    Some(rusk) => rusk.contract(&entity, &topic, &event).await,
                    None => Err(super::HttpError::Unsupported),
                },
            };
            (event, binary_request, result)
        },
    )
    .await
}

#[cfg(feature = "chain")]
async fn legacy_rues_graphql_post_route(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_rues_post(
        component_uri("graphql", "query"),
        headers,
        body,
        |event, binary_request| async move {
            let result = match state.services.chain_handler() {
                Some(chain) => chain.graphql_query(&event).await,
                None => Err(super::HttpError::Unsupported),
            };
            (event, binary_request, result)
        },
    )
    .await
}

#[cfg(test)]
async fn test_post(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_rues_post(
        component_uri("test", &topic),
        headers,
        body,
        |event, binary_request| async move {
            let result = match state.services.test_handler() {
                Some(handler) => handler.handle_test(&topic, &event).await,
                None => Err(super::HttpError::Unsupported),
            };
            (event, binary_request, result)
        },
    )
    .await
}

#[cfg(test)]
async fn test_subscribe(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    session_id: SessionId,
) -> Result<Response<Body>, ApiError> {
    run_subscribe(
        component_uri("test", &topic),
        headers,
        session_id,
        state.sockets_map,
    )
    .await
}

#[cfg(test)]
async fn test_unsubscribe(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    session_id: SessionId,
) -> Result<Response<Body>, ApiError> {
    run_unsubscribe(
        component_uri("test", &topic),
        headers,
        session_id,
        state.sockets_map,
    )
    .await
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
    let handler = match state.services.graphql_handler() {
        Some(handler) => handler,
        None => {
            return Ok(graphql::handle_graphql_http_error(
                StatusCode::NOT_FOUND,
                "GraphQL endpoint not configured",
            )
            .expect("GraphQL error response should be built"));
        }
    };

    graphql::handle_graphql_get(handler.as_ref(), req)
        .await
        .map_err(ApiError::from)
}

#[cfg(feature = "chain")]
async fn graphql_post_route(
    State(state): State<HttpAppState>,
    req: Request<Body>,
) -> Result<Response<Body>, ApiError> {
    let handler = match state.services.graphql_handler() {
        Some(handler) => handler,
        None => {
            return Ok(graphql::handle_graphql_http_error(
                StatusCode::NOT_FOUND,
                "GraphQL endpoint not configured",
            )
            .expect("GraphQL error response should be built"));
        }
    };

    graphql::handle_graphql_post(handler.as_ref(), req)
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
    use crate::http::{HttpHandlers, HttpPolicyConfig, RuesEvent, SessionId};

    fn test_state() -> HttpAppState {
        let (events_tx, _events_rx) = broadcast::channel::<RuesEvent>(1);
        let (shutdown_tx, _shutdown_rx) = broadcast::channel::<Infallible>(1);
        HttpAppState {
            services: HttpHandlers::default(),
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
