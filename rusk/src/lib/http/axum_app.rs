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
use serde::Deserialize;
use serde_json::json;
use tokio::sync::{RwLock, broadcast, mpsc};
#[cfg(any(feature = "chain", feature = "prover", test))]
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;
use utoipa_axum::router::OpenApiRouter;
#[cfg(feature = "chain")]
use utoipa_axum::router::UtoipaMethodRouterExt;
use utoipa_axum::routes;

#[cfg(feature = "chain")]
use super::MAX_GRAPHQL_REQUEST_BODY_BYTES;
use super::error::ApiError;
#[cfg(feature = "chain")]
use super::graphql;
use super::policy::HttpRequestPolicy;
use super::rues::SubscriptionAction;
#[cfg(any(feature = "chain", feature = "prover", test))]
use super::{HttpError, ResponseData};
use super::{HttpHandlers, RuesEvent, RuesEventUri, SessionId, openapi, rues};

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
    pub(super) enable_docs: bool,
    pub(super) policy: Arc<HttpRequestPolicy>,
    pub(super) headers: Arc<HeaderMap>,
}

pub(super) fn build_app(state: HttpAppState) -> Router {
    let enable_docs = state.enable_docs;
    let router = router()
        .fallback(not_found)
        .layer(from_fn_with_state(state.clone(), policy_middleware))
        .layer(from_fn_with_state(
            state.clone(),
            configured_headers_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state::<()>(state);

    if enable_docs {
        let (router, api) = router.split_for_parts();
        openapi::with_docs_routes(router, api)
    } else {
        router.into()
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn generated_openapi() -> utoipa::openapi::OpenApi {
    router().into_openapi()
}

/// This is the main Axum application router for Rusk's HTTP API, including both
/// the WebSocket `/on` routes and the RESTful API routes. The OpenAPI
/// documentation is generated from this router, so all API endpoints must be
/// defined here to be included in the docs.
fn router() -> OpenApiRouter<HttpAppState> {
    let router = openapi::router()
        .routes(routes!(rues::handle_rues_ws))
        .nest("/on", rues_router());

    // /graphql
    // Canonical GraphQL over HTTP endpoint
    #[cfg(feature = "chain")]
    let router = with_graphql_routes(router);

    // /static/drivers/{file}
    // Static file serving for WASM drivers
    #[cfg(feature = "http-wasm")]
    let router = with_static_routes(router);

    router
}

#[cfg(feature = "chain")]
fn with_graphql_routes(
    router: OpenApiRouter<HttpAppState>,
) -> OpenApiRouter<HttpAppState> {
    router.routes(routes!(graphql_get_route)).routes(
        routes!(graphql_post_route)
            .layer(RequestBodyLimitLayer::new(MAX_GRAPHQL_REQUEST_BODY_BYTES)),
    )
}

#[cfg(feature = "http-wasm")]
fn with_static_routes(
    router: OpenApiRouter<HttpAppState>,
) -> OpenApiRouter<HttpAppState> {
    router.routes(routes!(wasm_driver_route))
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
async fn run_dispatch<F, Fut>(
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

/// This is the router for the WebSocket `/on` endpoints that handle RUES
/// exclusively.
///
/// It is nested under the main router at the `/on` path. All routes defined
/// here will be prefixed with `/on`.
fn rues_router() -> OpenApiRouter<HttpAppState> {
    let router = OpenApiRouter::default();

    #[cfg(feature = "chain")]
    let router = {
        let router = with_transactions(router);
        let router = with_network_and_node(router);
        let router = with_block_and_stats(router);
        let router = with_account_and_blob(router);
        let router = with_contract(router);
        with_legacy_graphql_routes(router)
    };

    #[cfg(feature = "prover")]
    let router = with_prover(router);

    #[cfg(test)]
    let router = tests::with_test_routes(router);

    router
}
#[cfg(any(feature = "chain", feature = "prover"))]
fn with_default_rues_body_limit(
    router: OpenApiRouter<HttpAppState>,
) -> OpenApiRouter<HttpAppState> {
    router.route_layer(RequestBodyLimitLayer::new(
        super::MAX_RUES_REQUEST_BODY_BYTES,
    ))
}

#[cfg(feature = "chain")]
fn with_transactions(
    router: OpenApiRouter<HttpAppState>,
) -> OpenApiRouter<HttpAppState> {
    router
        .merge(with_default_rues_body_limit(
            OpenApiRouter::default()
                .routes(routes!(transactions_propagate_post))
                .routes(routes!(transactions_post)),
        ))
        .routes(routes!(transactions_subscribe))
        .routes(routes!(transactions_unsubscribe))
}

#[cfg(feature = "chain")]
fn with_network_and_node(
    router: OpenApiRouter<HttpAppState>,
) -> OpenApiRouter<HttpAppState> {
    router.merge(with_default_rues_body_limit(
        OpenApiRouter::default()
            .routes(routes!(network_post))
            .routes(routes!(node_post)),
    ))
}

#[cfg(feature = "chain")]
fn with_block_and_stats(
    router: OpenApiRouter<HttpAppState>,
) -> OpenApiRouter<HttpAppState> {
    router
        .merge(with_default_rues_body_limit(
            OpenApiRouter::default()
                .routes(routes!(blocks_post))
                .routes(routes!(stats_post)),
        ))
        .routes(routes!(blocks_subscribe))
        .routes(routes!(blocks_unsubscribe))
}

#[cfg(feature = "chain")]
fn with_account_and_blob(
    router: OpenApiRouter<HttpAppState>,
) -> OpenApiRouter<HttpAppState> {
    router.merge(with_default_rues_body_limit(
        OpenApiRouter::default()
            .routes(routes!(account_post))
            .routes(routes!(blobs_post)),
    ))
}

#[cfg(feature = "chain")]
fn with_contract(
    router: OpenApiRouter<HttpAppState>,
) -> OpenApiRouter<HttpAppState> {
    router
        .merge(with_default_rues_body_limit(
            OpenApiRouter::default()
                .routes(routes!(contracts_post))
                .routes(routes!(driver_post))
                .routes(routes!(contract_owner_post))
                .routes(routes!(contract_post)),
        ))
        .routes(routes!(contracts_subscribe))
        .routes(routes!(contracts_unsubscribe))
        .routes(routes!(contract_upload_driver_post).layer(
            RequestBodyLimitLayer::new(super::MAX_DRIVER_UPLOAD_BODY_BYTES),
        ))
}

#[cfg(feature = "prover")]
fn with_prover(
    router: OpenApiRouter<HttpAppState>,
) -> OpenApiRouter<HttpAppState> {
    router.merge(with_default_rues_body_limit(
        OpenApiRouter::default().routes(routes!(prover_post)),
    ))
}

#[cfg(feature = "chain")]
fn with_legacy_graphql_routes(
    router: OpenApiRouter<HttpAppState>,
) -> OpenApiRouter<HttpAppState> {
    router.merge(with_default_rues_body_limit(
        OpenApiRouter::default()
            .routes(routes!(legacy_rues_graphql_post_route)),
    ))
}

/// Propagate a new transaction to the network.
///
/// This endpoint pre-validates transactions before broadcasting them to peers,
/// returning `202 Accepted` if the transaction passed preverification. The
/// request body contains binary-encoded transaction bytes.
#[cfg(feature = "chain")]
#[utoipa::path(
    post,
    path = "/transactions/propagate",
    tag = "RUES / Dispatch",
    params(super::openapi::VersionHeaders),
    request_body(
        content = String,
        content_type = "application/octet-stream",
        description = "Binary-encoded transaction bytes."
    ),
    responses(
        (status = 202, description = "Transaction accepted for broadcasting to peers"),
        (status = 400, description = "Invalid transaction bytes or version headers", body = super::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error during transaction preverification or broadcast", body = super::openapi::RuesErrorResponse),
        (status = 501, description = "Transactions handler not configured", body = super::openapi::RuesErrorResponse)
    )
)]
async fn transactions_propagate_post(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_dispatch(
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

/// Transaction execution endpoints.
///
/// Supported topics are `preverify`, `propagate`, and `simulate`. The request
/// body contains binary-encoded transaction bytes.
#[cfg(feature = "chain")]
#[utoipa::path(
    post,
    path = "/transactions/{topic}",
    tag = "RUES / Dispatch",
    params(
        super::openapi::VersionHeaders,
        ("topic" = String, Path, description = "Transaction topic: preverify | propagate | simulate")
    ),
    request_body(
        content = String,
        content_type = "application/octet-stream",
        description = "Binary-encoded transaction bytes."
    ),
    responses(
        (status = 200, description = "Transaction simulation result", body = serde_json::Value, content_type = "application/json"),
        (status = 202, description = "Request accepted with no immediate response"),
        (status = 400, description = "Invalid transaction request or version headers", body = super::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error during transaction handling", body = super::openapi::RuesErrorResponse),
        (status = 501, description = "Transactions handler not configured or topic unsupported", body = super::openapi::RuesErrorResponse)
    )
)]
async fn transactions_post(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_dispatch(
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

/// Subscribe to transaction event streams.
///
/// The `Rusk-Session-Id` header must contain the session identifier emitted by
/// the `/on` WebSocket handshake for the subscriptions to be routed correctly.
#[cfg(feature = "chain")]
#[utoipa::path(
    get,
    path = "/transactions/{topic}",
    tag = "RUES / Events",
    params(
        super::openapi::VersionHeaders,
        super::openapi::SessionHeader,
        ("topic" = String, Path, description = "Transaction event topic to monitor")
    ),
    responses(
        (status = 200, description = "Subscription registered"),
        (status = 424, description = "Session ID missing or invalid", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Failed to register the subscription", body = super::openapi::RuesErrorResponse)
    )
)]
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

/// Unsubscribe from transaction event streams.
///
/// The `Rusk-Session-Id` header must contain the session identifier emitted by
/// the `/on` WebSocket handshake.
#[cfg(feature = "chain")]
#[utoipa::path(
    delete,
    path = "/transactions/{topic}",
    tag = "RUES / Events",
    params(
        super::openapi::VersionHeaders,
        super::openapi::SessionHeader,
        ("topic" = String, Path, description = "Transaction event topic subscription to remove")
    ),
    responses(
        (status = 200, description = "Subscription removed"),
        (status = 404, description = "Subscription not found", body = super::openapi::RuesErrorResponse),
        (status = 424, description = "Session ID missing or invalid", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Failed to remove the subscription", body = super::openapi::RuesErrorResponse)
    )
)]
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

/// Network topology and peer visibility queries.
///
/// Supported topics are `peers` and `peers_location`. The request body may
/// contain an optional plain-text peer count for the `peers` topic.
#[cfg(feature = "chain")]
#[utoipa::path(
    post,
    path = "/network/{topic}",
    tag = "RUES / Dispatch",
    params(
        super::openapi::VersionHeaders,
        ("topic" = String, Path, description = "Network topic: peers | peers_location")
    ),
    request_body(
        content = String,
        content_type = "text/plain",
        description = "Optional peer count for the `peers` topic."
    ),
    responses(
        (status = 200, description = "Network query response", body = serde_json::Value, content_type = "application/json"),
        (status = 400, description = "Invalid network request or version headers", body = super::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while resolving the network query", body = super::openapi::RuesErrorResponse),
        (status = 501, description = "Network handler not configured or topic unsupported", body = super::openapi::RuesErrorResponse)
    )
)]
async fn network_post(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_dispatch(
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

/// Node information and synchronization state queries.
///
/// Supported topics are `info` (node and chain runtime info), `provisioners`
/// (stake table), and `crs` (common reference string).
#[cfg(feature = "chain")]
#[utoipa::path(
    post,
    path = "/node/{topic}",
    tag = "RUES / Dispatch",
    params(
        super::openapi::VersionHeaders,
        ("topic" = String, Path, description = "Node topic: info | provisioners | crs")
    ),
    responses(
        (status = 200, description = "Node information response", content(
            (serde_json::Value = "application/json"),
            (String = "application/octet-stream")
        )),
        (status = 400, description = "Invalid node request or version headers", body = super::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while resolving the node query", body = super::openapi::RuesErrorResponse),
        (status = 501, description = "Node handler not configured or topic unsupported", body = super::openapi::RuesErrorResponse)
    )
)]
async fn node_post(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_dispatch(
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

/// Block-related queries.
///
/// `POST` currently supports `gas-price`, which returns mempool gas-price
/// statistics. The request body may contain an optional plain-text transaction
/// count limit.
#[cfg(feature = "chain")]
#[utoipa::path(
    post,
    path = "/blocks/{topic}",
    tag = "RUES / Dispatch",
    params(
        super::openapi::VersionHeaders,
        ("topic" = String, Path, description = "Blocks topic: gas-price (POST); event topic for GET/DELETE")
    ),
    request_body(
        content = String,
        content_type = "text/plain",
        description = "Optional transaction count limit for `gas-price`."
    ),
    responses(
        (status = 200, description = "Block query response", body = serde_json::Value, content_type = "application/json"),
        (status = 400, description = "Invalid block request or version headers", body = super::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while resolving the block query", body = super::openapi::RuesErrorResponse),
        (status = 501, description = "Blocks handler not configured or topic unsupported", body = super::openapi::RuesErrorResponse)
    )
)]
async fn blocks_post(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_dispatch(
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

/// Subscribe to new block finalization events.
///
/// The `Rusk-Session-Id` header must contain the session identifier emitted by
/// the `/on` WebSocket handshake.
#[cfg(feature = "chain")]
#[utoipa::path(
    get,
    path = "/blocks/{topic}",
    tag = "RUES / Events",
    params(
        super::openapi::VersionHeaders,
        super::openapi::SessionHeader,
        ("topic" = String, Path, description = "Block event type to monitor")
    ),
    responses(
        (status = 200, description = "Block subscription registered"),
        (status = 424, description = "Session ID missing or invalid", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Failed to register the subscription", body = super::openapi::RuesErrorResponse)
    )
)]
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

/// Unsubscribe from block event notifications.
///
/// The `Rusk-Session-Id` header must contain the session identifier emitted by
/// the `/on` WebSocket handshake.
#[cfg(feature = "chain")]
#[utoipa::path(
    delete,
    path = "/blocks/{topic}",
    tag = "RUES / Events",
    params(
        super::openapi::VersionHeaders,
        super::openapi::SessionHeader,
        ("topic" = String, Path, description = "Block event subscription to remove")
    ),
    responses(
        (status = 200, description = "Block subscription removed"),
        (status = 404, description = "Subscription not found", body = super::openapi::RuesErrorResponse),
        (status = 424, description = "Session ID missing or invalid", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Failed to remove the subscription", body = super::openapi::RuesErrorResponse)
    )
)]
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

/// Archive-backed node statistics queries.
///
/// Supported topics are `account_count` and `tx_count`.
#[cfg(feature = "chain")]
#[utoipa::path(
    post,
    path = "/stats/{topic}",
    tag = "RUES / Dispatch",
    params(
        super::openapi::VersionHeaders,
        ("topic" = String, Path, description = "Stats topic: account_count | tx_count")
    ),
    responses(
        (status = 200, description = "Statistics response", body = serde_json::Value, content_type = "application/json"),
        (status = 400, description = "Invalid statistics request or version headers", body = super::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while resolving the statistics query", body = super::openapi::RuesErrorResponse),
        (status = 501, description = "Stats handler not configured or topic unsupported", body = super::openapi::RuesErrorResponse)
    )
)]
async fn stats_post(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_dispatch(
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

/// Zero-knowledge proof generation.
///
/// Submit binary-encoded prover requests. The currently supported topic is
/// `prove`.
#[cfg(feature = "prover")]
#[utoipa::path(
    post,
    path = "/prover/{topic}",
    tag = "RUES / Dispatch",
    params(
        super::openapi::VersionHeaders,
        ("topic" = String, Path, description = "Prover topic: prove")
    ),
    request_body(
        content = String,
        content_type = "application/octet-stream",
        description = "Binary-encoded proof input."
    ),
    responses(
        (status = 200, description = "Generated proof bytes", body = String, content_type = "application/octet-stream"),
        (status = 400, description = "Invalid prover request or version headers", body = super::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Internal prover failure", body = super::openapi::RuesErrorResponse),
        (status = 501, description = "Prover handler not configured or topic unsupported", body = super::openapi::RuesErrorResponse)
    )
)]
async fn prover_post(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_dispatch(
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

/// Account state queries for a specific account.
///
/// Query account status (balance, nonce, next nonce). The supported topic is
/// `status`. The entity parameter is a base58-encoded BLS public key.
#[cfg(feature = "chain")]
#[utoipa::path(
    post,
    path = "/account:{entity}/{topic}",
    tag = "RUES / Dispatch",
    params(
        super::openapi::VersionHeaders,
        ("entity" = String, Path, description = "Base58-encoded BLS public key"),
        ("topic" = String, Path, description = "Account topic: status")
    ),
    responses(
        (status = 200, description = "Account status response", body = serde_json::Value, content_type = "application/json"),
        (status = 400, description = "Invalid account query or version headers", body = super::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while resolving the account query", body = super::openapi::RuesErrorResponse),
        (status = 501, description = "Account handler not configured or topic unsupported", body = super::openapi::RuesErrorResponse)
    )
)]
async fn account_post(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_dispatch(
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

/// Blob retrieval by commitment or versioned hash.
///
/// Query blob data by commitment or by hash. Supported topics are `commitment`
/// and `hash`. The entity parameter is a hex-encoded commitment or hash value.
#[cfg(feature = "chain")]
#[utoipa::path(
    post,
    path = "/blobs:{entity}/{topic}",
    tag = "RUES / Dispatch",
    params(
        super::openapi::VersionHeaders,
        ("entity" = String, Path, description = "Hex-encoded blob commitment or hash"),
        ("topic" = String, Path, description = "BLOB topic: commitment | hash")
    ),
    responses(
        (status = 200, description = "Blob response", content(
            (serde_json::Value = "application/json"),
            (String = "application/octet-stream"),
            (String = "text/plain")
        )),
        (status = 400, description = "Invalid blob query or version headers", body = super::openapi::RuesErrorResponse),
        (status = 404, description = "Blob not found", body = super::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while resolving the blob query", body = super::openapi::RuesErrorResponse),
        (status = 501, description = "Blob handler not configured or topic unsupported", body = super::openapi::RuesErrorResponse)
    )
)]
async fn blobs_post(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_dispatch(
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

/// Contract query and call endpoint.
///
/// This plural `/contracts` route is the contract execution surface. It accepts
/// either raw binary call arguments or JSON that is encoded through the
/// contract's data driver. When `Rusk-Feeder` is present, streaming/feeder
/// responses are enabled for contracts that require them.
#[cfg(feature = "chain")]
#[utoipa::path(
    post,
    path = "/contracts:{entity}/{topic}",
    tag = "RUES / Dispatch",
    params(
        super::openapi::VersionHeaders,
        super::openapi::FeederHeader,
        ("entity" = String, Path, description = "Smart contract address or ID (hex)"),
        ("topic" = String, Path, description = "Contract function or query topic")
    ),
    request_body(
        content(
            (String = "application/octet-stream"),
            (serde_json::Value = "application/json")
        ),
        description = "Binary contract arguments or JSON that will be encoded by the contract data driver."
    ),
    responses(
        (status = 200, description = "Contract query response", content(
            (serde_json::Value = "application/json"),
            (String = "application/octet-stream"),
            (String = "text/plain")
        )),
        (status = 400, description = "Invalid contract query, feeder usage, or version headers", body = super::openapi::RuesErrorResponse),
        (status = 404, description = "Contract or contract data driver not found", body = super::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while executing the contract query", body = super::openapi::RuesErrorResponse),
        (status = 501, description = "Contracts handler not configured", body = super::openapi::RuesErrorResponse)
    )
)]
async fn contracts_post(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_dispatch(
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

/// Subscribe to contract events and state changes.
///
/// The `Rusk-Session-Id` header must contain the session identifier emitted by
/// the `/on` WebSocket handshake for event delivery.
#[cfg(feature = "chain")]
#[utoipa::path(
    get,
    path = "/contracts:{entity}/{topic}",
    tag = "RUES / Events",
    params(
        super::openapi::VersionHeaders,
        super::openapi::SessionHeader,
        ("entity" = String, Path, description = "Smart contract address or ID (hex)"),
        ("topic" = String, Path, description = "Contract event type to monitor")
    ),
    responses(
        (status = 200, description = "Contract event subscription registered"),
        (status = 424, description = "Session ID missing or invalid", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Failed to register the subscription", body = super::openapi::RuesErrorResponse)
    )
)]
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

/// Unsubscribe from contract event notifications.
///
/// The `Rusk-Session-Id` header must contain the session identifier emitted by
/// the `/on` WebSocket handshake.
#[cfg(feature = "chain")]
#[utoipa::path(
    delete,
    path = "/contracts:{entity}/{topic}",
    tag = "RUES / Events",
    params(
        super::openapi::VersionHeaders,
        super::openapi::SessionHeader,
        ("entity" = String, Path, description = "Smart contract address or ID (hex)"),
        ("topic" = String, Path, description = "Contract event subscription to remove")
    ),
    responses(
        (status = 200, description = "Contract event subscription removed"),
        (status = 404, description = "Subscription not found", body = super::openapi::RuesErrorResponse),
        (status = 424, description = "Session ID missing or invalid", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Failed to remove the subscription", body = super::openapi::RuesErrorResponse)
    )
)]
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

/// Contract data-driver encode, decode, schema, and version operations.
///
/// Supported topics are `decode_event[:name]`, `decode_input_fn[:name]`,
/// `decode_output_fn[:name]`, `encode_input_fn[:name]`, `get_schema`, and
/// `get_version`.
#[cfg(feature = "chain")]
#[utoipa::path(
    post,
    path = "/driver:{entity}/{topic}",
    tag = "RUES / Dispatch",
    params(
        super::openapi::VersionHeaders,
        ("entity" = String, Path, description = "Hex-encoded contract ID"),
        ("topic" = String, Path, description = "Driver topic: decode_event[:name] | decode_input_fn[:name] | decode_output_fn[:name] | encode_input_fn[:name] | get_schema | get_version")
    ),
    request_body(
        content(
            (String = "application/octet-stream"),
            (serde_json::Value = "application/json")
        ),
        description = "Binary payload for decode operations or JSON input for `encode_input_fn`."
    ),
    responses(
        (status = 200, description = "Driver operation response", content(
            (serde_json::Value = "application/json"),
            (String = "application/octet-stream"),
            (String = "text/plain")
        )),
        (status = 400, description = "Invalid driver request or version headers", body = super::openapi::RuesErrorResponse),
        (status = 404, description = "Driver or contract not found", body = super::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while executing the driver operation", body = super::openapi::RuesErrorResponse),
        (status = 501, description = "Driver handler not configured", body = super::openapi::RuesErrorResponse)
    )
)]
async fn driver_post(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_dispatch(
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

/// Contract owner lookup.
///
/// The current handler ignores `topic` and resolves owner by contract entity.
/// The topic segment is preserved for legacy route compatibility.
#[cfg(feature = "chain")]
#[utoipa::path(
    post,
    path = "/contract_owner:{entity}/{topic}",
    tag = "RUES / Dispatch",
    params(
        super::openapi::VersionHeaders,
        ("entity" = String, Path, description = "Hex-encoded contract ID"),
        ("topic" = String, Path, description = "Legacy placeholder (currently ignored)")
    ),
    responses(
        (status = 200, description = "Contract owner bytes or hex-encoded owner", content(
            (String = "application/octet-stream"),
            (String = "text/plain")
        )),
        (status = 400, description = "Invalid contract owner query or version headers", body = super::openapi::RuesErrorResponse),
        (status = 404, description = "Contract owner not found", body = super::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while resolving contract ownership", body = super::openapi::RuesErrorResponse),
        (status = 501, description = "Contract-owner handler not configured", body = super::openapi::RuesErrorResponse)
    )
)]
async fn contract_owner_post(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_dispatch(
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

/// Upload a contract WASM driver.
///
/// The request body contains binary WASM bytecode. The `sign` header is
/// required and must contain the owner's BLS signature over the uploaded
/// bytecode hash.
#[cfg(feature = "chain")]
#[utoipa::path(
    post,
    path = "/contract:{entity}/upload_driver",
    tag = "RUES / Dispatch",
    params(
        super::openapi::VersionHeaders,
        super::openapi::UploadDriverHeader,
        ("entity" = String, Path, description = "Target contract address or ID (hex)")
    ),
    request_body(
        content = String,
        content_type = "application/octet-stream",
        description = "WASM driver bytecode."
    ),
    responses(
        (status = 200, description = "Driver uploaded and verified", body = String, content_type = "text/plain"),
        (status = 400, description = "Invalid driver bytecode, signature, or version headers", body = super::openapi::RuesErrorResponse),
        (status = 404, description = "Contract owner or contract metadata not found", body = super::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the upload-driver size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while storing the uploaded driver", body = super::openapi::RuesErrorResponse),
        (status = 501, description = "Contract handler not configured", body = super::openapi::RuesErrorResponse)
    )
)]
async fn contract_upload_driver_post(
    State(state): State<HttpAppState>,
    Path(entity): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    let topic = "upload_driver".to_string();
    run_dispatch(
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

/// Single-contract status, metadata, and driver download endpoint.
///
/// The singular `/contract` route is reserved for contract status and
/// management topics. `status` is handled by the chain view, while `metadata`
/// and `download_driver` are handled by the Rusk contract-management surface.
#[cfg(feature = "chain")]
#[utoipa::path(
    post,
    path = "/contract:{entity}/{topic}",
    tag = "RUES / Dispatch",
    params(
        super::openapi::VersionHeaders,
        ("entity" = String, Path, description = "Contract address or ID (hex)"),
        ("topic" = String, Path, description = "Contract topic: status | metadata | download_driver")
    ),
    responses(
        (status = 200, description = "Contract status or management response", content(
            (serde_json::Value = "application/json"),
            (String = "application/wasm")
        )),
        (status = 400, description = "Invalid contract request or version headers", body = super::openapi::RuesErrorResponse),
        (status = 404, description = "Driver or contract resource not found", body = super::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while resolving the contract request", body = super::openapi::RuesErrorResponse),
        (status = 501, description = "Contract handler not configured or topic unsupported", body = super::openapi::RuesErrorResponse)
    )
)]
async fn contract_post(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_dispatch(
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

/// Legacy RUES GraphQL query endpoint under `/on`.
///
/// This route preserves the historical RUES behavior: the request body is a
/// raw GraphQL query string, and an empty body returns the schema SDL.
#[cfg(feature = "chain")]
#[utoipa::path(
    post,
    path = "/graphql/query",
    tag = "RUES / Dispatch",
    request_body(
        content(
            (String = "text/plain"),
            (String = "application/octet-stream")
        ),
        description = "Raw GraphQL query text. An empty body returns the schema SDL."
    ),
    responses(
        (
            status = 200,
            description = "Schema SDL, GraphQL query response, or GraphQL error response",
            content(
                (serde_json::Value = "application/json"),
                (String = "text/plain")
            )
        ),
        (status = 400, description = "Invalid RUES version headers", body = super::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed payload or headers", body = super::openapi::RuesErrorResponse),
        (status = 500, description = "Internal legacy GraphQL handling failure", body = super::openapi::RuesErrorResponse),
        (status = 501, description = "Legacy GraphQL handler not configured", body = super::openapi::RuesErrorResponse)
    )
)]
async fn legacy_rues_graphql_post_route(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    run_dispatch(
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

/// GraphQL HTTP GET endpoint.
///
/// This route documents the standard GraphQL-over-HTTP query-string transport.
/// The `query` parameter is required.
#[cfg(feature = "chain")]
#[utoipa::path(
    get,
    path = "/graphql",
    tag = "GraphQL",
    params(super::openapi::GraphqlGetParams),
    responses(
        (status = 200, description = "Successful GraphQL response", body = super::openapi::GraphqlHttpResponse, content_type = "application/json"),
        (status = 400, description = "Missing or invalid GraphQL query parameters", body = super::openapi::GraphqlHttpResponse, content_type = "application/json"),
        (status = 404, description = "GraphQL endpoint not configured", body = super::openapi::GraphqlHttpResponse, content_type = "application/json")
    )
)]
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

/// GraphQL HTTP POST endpoint.
///
/// Clients send a standard GraphQL-over-HTTP JSON body to the canonical
/// `/graphql` endpoint.
#[cfg(feature = "chain")]
#[utoipa::path(
    post,
    path = "/graphql",
    tag = "GraphQL",
    request_body(
        content = super::openapi::GraphqlHttpRequest,
        content_type = "application/json"
    ),
    responses(
        (
            status = 200,
            description = "Successful GraphQL response",
            body = super::openapi::GraphqlHttpResponse,
            content_type = "application/json"
        ),
        (status = 400, description = "Invalid GraphQL request", body = super::openapi::GraphqlHttpResponse, content_type = "application/json"),
        (status = 404, description = "GraphQL endpoint not configured", body = super::openapi::GraphqlHttpResponse, content_type = "application/json"),
        (status = 413, description = "Request body exceeds the GraphQL size limit")
    )
)]
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

/// Fetch static WASM driver assets.
#[cfg(feature = "http-wasm")]
#[utoipa::path(
    get,
    path = "/static/drivers/{file}",
    tag = "Static",
    params(("file" = String, Path, description = "Driver filename (e.g., transfer.wasm)")),
    responses(
        (status = 200, description = "Driver WASM bytecode", content_type = "application/wasm"),
        (status = 404, description = "Driver file not found", body = super::openapi::ErrorEnvelope, content_type = "application/json")
    )
)]
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
    use axum::extract::{Path, State};
    use axum::http::header::CONTENT_TYPE;
    use axum::http::{HeaderMap, Request, StatusCode};
    use axum::response::Response;
    use axum::routing::post;
    use serde_json::Value;
    use tokio::sync::{RwLock, broadcast, mpsc};
    use tower::ServiceExt;
    use tower_http::limit::RequestBodyLimitLayer;
    use utoipa_axum::router::OpenApiRouter;

    use super::rues::SubscriptionAction;
    use super::{
        ApiError, HttpAppState, HttpRequestPolicy, TopicPath, build_app,
        component_uri, run_dispatch, run_subscribe, run_unsubscribe,
    };
    use crate::http::{
        HttpHandlers, HttpPolicyConfig, MAX_RUES_REQUEST_BODY_BYTES, RuesEvent,
        SessionId,
    };

    pub(super) fn with_test_routes(
        router: OpenApiRouter<HttpAppState>,
    ) -> OpenApiRouter<HttpAppState> {
        router.route(
            "/test/{topic}",
            post(test_post)
                .get(test_subscribe)
                .delete(test_unsubscribe)
                .layer(RequestBodyLimitLayer::new(MAX_RUES_REQUEST_BODY_BYTES)),
        )
    }

    pub(super) async fn test_post(
        State(state): State<HttpAppState>,
        Path(TopicPath { topic }): Path<TopicPath>,
        headers: HeaderMap,
        body: axum::body::Bytes,
    ) -> Result<Response<Body>, ApiError> {
        run_dispatch(
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

    pub(super) async fn test_subscribe(
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

    pub(super) async fn test_unsubscribe(
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

    fn test_state(enable_docs: bool) -> HttpAppState {
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
            enable_docs,
            policy: Arc::new(HttpRequestPolicy::new(
                HttpPolicyConfig::default(),
            )),
            headers: Arc::new(axum::http::HeaderMap::new()),
        }
    }

    #[tokio::test]
    async fn router_fallback_returns_json_not_found() {
        let app = build_app(test_state(false));
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

    #[tokio::test]
    async fn docs_routes_are_available_when_enabled() {
        let app = build_app(test_state(true));

        let openapi_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api-docs/openapi.json")
                    .body(Body::empty())
                    .expect("Request should be built"),
            )
            .await
            .expect("OpenAPI route should respond");

        assert_eq!(openapi_response.status(), StatusCode::OK);
        let openapi_body = to_bytes(openapi_response.into_body(), usize::MAX)
            .await
            .expect("OpenAPI body should be readable");
        let openapi_json: Value = serde_json::from_slice(&openapi_body)
            .expect("OpenAPI body should be JSON");
        let paths = openapi_json
            .get("paths")
            .and_then(Value::as_object)
            .expect("OpenAPI paths should be present");
        assert!(
            paths.contains_key("/graphql"),
            "OpenAPI document should include /graphql"
        );

        let swagger_response = app
            .oneshot(
                Request::builder()
                    .uri("/swagger-ui/index.html")
                    .body(Body::empty())
                    .expect("Request should be built"),
            )
            .await
            .expect("Swagger route should respond");

        assert_eq!(swagger_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn docs_routes_are_not_available_when_disabled() {
        let app = build_app(test_state(false));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api-docs/openapi.json")
                    .body(Body::empty())
                    .expect("Request should be built"),
            )
            .await
            .expect("Response should be produced");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
