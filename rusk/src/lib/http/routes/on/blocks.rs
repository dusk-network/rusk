use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Response};
use tower_http::limit::RequestBodyLimitLayer;
use utoipa_axum::router::{OpenApiRouter, UtoipaMethodRouterExt};
use utoipa_axum::routes;

use crate::http::error::ApiError;
use crate::http::routes::on::{EntityTopicPath, TopicPath};
use crate::http::{
    HttpAppState, HttpError, MAX_RUES_REQUEST_BODY_BYTES, SessionId, rues,
};

pub(crate) fn block_and_stats_routes(
    router: OpenApiRouter<HttpAppState>,
) -> OpenApiRouter<HttpAppState> {
    router
        .routes(
            routes!(blocks_post)
                .layer(RequestBodyLimitLayer::new(MAX_RUES_REQUEST_BODY_BYTES)),
        )
        .routes(routes!(blocks_subscribe))
        .routes(routes!(blocks_unsubscribe))
        .routes(routes!(blocks_entity_subscribe))
        .routes(routes!(blocks_entity_unsubscribe))
        .routes(
            routes!(stats_post)
                .layer(RequestBodyLimitLayer::new(MAX_RUES_REQUEST_BODY_BYTES)),
        )
}

/// Block chain queries.
///
/// `POST` currently supports `gas-price`, which returns mempool gas-price
/// statistics.
#[utoipa::path(
    post,
    path = "/blocks/{topic}",
    tag = "RUES / Dispatch",
    params(
        crate::http::openapi::VersionHeaders,
        ("topic" = String, Path, description = "Blocks topic: gas-price (POST); event topic for GET/DELETE")
    ),
    request_body(
        content = String,
        content_type = "text/plain",
        description = "Optional transaction count limit for `gas-price`."
    ),
    responses(
        (status = 200, description = "Block query response", body = serde_json::Value, content_type = "application/json"),
        (status = 400, description = "Invalid block request or version headers", body = crate::http::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while resolving the block query", body = crate::http::openapi::RuesErrorResponse),
        (status = 501, description = "Blocks handler not configured or topic unsupported", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn blocks_post(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    let request =
        rues::ParsedRuesRequest::component("blocks", &topic, headers, body)?;
    let result = match state.services.chain_handler() {
        Some(chain) => chain.blocks(&topic, request.event()).await,
        None => Err(HttpError::Unsupported),
    };
    request.into_response(result)
}

/// Subscribe to new block finalization events.
///
/// The `Rusk-Session-Id` header must contain the session identifier emitted by
/// the `/on` WebSocket handshake.
#[utoipa::path(
    get,
    path = "/blocks/{topic}",
    tag = "RUES / Events",
    params(
        crate::http::openapi::VersionHeaders,
        crate::http::openapi::SessionHeader,
        ("topic" = String, Path, description = "Block event type to monitor")
    ),
    responses(
        (status = 200, description = "Block subscription registered"),
        (status = 424, description = "Session ID missing or invalid", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Failed to register the subscription", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn blocks_subscribe(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    session_id: SessionId,
) -> Result<Response<Body>, ApiError> {
    rues::subscribe("blocks", None, &topic, session_id, state.sockets_map).await
}

/// Subscribe to block events for a specific block identifier.
///
/// This preserves the legacy RUES `blocks:{entity}/{topic}` route shape used
/// by clients subscribing to a single block state transition.
#[utoipa::path(
    get,
    path = "/blocks:{entity}/{topic}",
    tag = "RUES / Events",
    params(
        crate::http::openapi::VersionHeaders,
        crate::http::openapi::SessionHeader,
        ("entity" = String, Path, description = "Block identifier to monitor"),
        ("topic" = String, Path, description = "Block event type to monitor")
    ),
    responses(
        (status = 200, description = "Block subscription registered"),
        (status = 424, description = "Session ID missing or invalid", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Failed to register the subscription", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn blocks_entity_subscribe(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    session_id: SessionId,
) -> Result<Response<Body>, ApiError> {
    rues::subscribe(
        "blocks",
        Some(&entity),
        &topic,
        session_id,
        state.sockets_map,
    )
    .await
}

/// Unsubscribe from block event notifications.
///
/// The `Rusk-Session-Id` header must contain the session identifier emitted by
/// the `/on` WebSocket handshake.
#[utoipa::path(
    delete,
    path = "/blocks/{topic}",
    tag = "RUES / Events",
    params(
        crate::http::openapi::VersionHeaders,
        crate::http::openapi::SessionHeader,
        ("topic" = String, Path, description = "Block event subscription to remove")
    ),
    responses(
        (status = 200, description = "Block subscription removed"),
        (status = 404, description = "Subscription not found", body = crate::http::openapi::RuesErrorResponse),
        (status = 424, description = "Session ID missing or invalid", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Failed to remove the subscription", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn blocks_unsubscribe(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    session_id: SessionId,
) -> Result<Response<Body>, ApiError> {
    rues::unsubscribe("blocks", None, &topic, session_id, state.sockets_map)
        .await
}

/// Unsubscribe from block events for a specific block identifier.
#[utoipa::path(
    delete,
    path = "/blocks:{entity}/{topic}",
    tag = "RUES / Events",
    params(
        crate::http::openapi::VersionHeaders,
        crate::http::openapi::SessionHeader,
        ("entity" = String, Path, description = "Block identifier being monitored"),
        ("topic" = String, Path, description = "Block event subscription to remove")
    ),
    responses(
        (status = 200, description = "Block subscription removed"),
        (status = 404, description = "Subscription not found", body = crate::http::openapi::RuesErrorResponse),
        (status = 424, description = "Session ID missing or invalid", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Failed to remove the subscription", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn blocks_entity_unsubscribe(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    session_id: SessionId,
) -> Result<Response<Body>, ApiError> {
    rues::unsubscribe(
        "blocks",
        Some(&entity),
        &topic,
        session_id,
        state.sockets_map,
    )
    .await
}

/// Node statistics and performance metrics queries.
///
/// Submit binary-encoded archive statistics queries. Supported topics are
/// `account_count` and `tx_count`.
#[utoipa::path(
    post,
    path = "/stats/{topic}",
    tag = "RUES / Dispatch",
    params(
        crate::http::openapi::VersionHeaders,
        ("topic" = String, Path, description = "Stats topic: account_count | tx_count")
    ),
    responses(
        (status = 200, description = "Statistics response", body = serde_json::Value, content_type = "application/json"),
        (status = 400, description = "Invalid statistics request or version headers", body = crate::http::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while resolving the statistics query", body = crate::http::openapi::RuesErrorResponse),
        (status = 501, description = "Stats handler not configured or topic unsupported", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn stats_post(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    let request =
        rues::ParsedRuesRequest::component("stats", &topic, headers, body)?;
    let result = match state.services.chain_handler() {
        Some(chain) => chain.stats(&topic, request.event()).await,
        None => Err(HttpError::Unsupported),
    };
    request.into_response(result)
}
