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

pub(crate) fn transaction_routes(
    router: OpenApiRouter<HttpAppState>,
) -> OpenApiRouter<HttpAppState> {
    router
        .routes(
            routes!(transactions_propagate_post)
                .layer(RequestBodyLimitLayer::new(MAX_RUES_REQUEST_BODY_BYTES)),
        )
        .routes(
            routes!(transactions_post)
                .layer(RequestBodyLimitLayer::new(MAX_RUES_REQUEST_BODY_BYTES)),
        )
        .routes(routes!(transactions_subscribe))
        .routes(routes!(transactions_unsubscribe))
        .routes(routes!(transactions_entity_subscribe))
        .routes(routes!(transactions_entity_unsubscribe))
}

/// Propagate a new transaction to the network.
///
/// This endpoint pre-validates transactions before broadcasting them to peers,
/// returning `202 Accepted` if the transaction passed preverification. The
/// request body contains binary-encoded transaction bytes.
#[utoipa::path(
    post,
    path = "/transactions/propagate",
    tag = "RUES / Dispatch",
    params(crate::http::openapi::VersionHeaders),
    request_body(
        content = String,
        content_type = "application/octet-stream",
        description = "Binary-encoded transaction bytes."
    ),
    responses(
        (status = 202, description = "Transaction accepted for broadcasting to peers"),
        (status = 400, description = "Invalid transaction bytes or version headers", body = crate::http::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error during transaction preverification or broadcast", body = crate::http::openapi::RuesErrorResponse),
        (status = 501, description = "Transactions handler not configured", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn transactions_propagate_post(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    let request = rues::ParsedRuesRequest::component(
        "transactions",
        "propagate",
        headers,
        body,
    )?;
    let result = match state.services.chain_handler() {
        Some(chain) => chain.transactions("propagate", request.event()).await,
        None => Err(HttpError::Unsupported),
    };
    request.into_response(result)
}

/// Account and payment transaction queries.
///
/// Submit binary-encoded transaction requests. Supported topics are
/// `preverify`, `propagate`, and `simulate`.
#[utoipa::path(
    post,
    path = "/transactions/{topic}",
    tag = "RUES / Dispatch",
    params(
        crate::http::openapi::VersionHeaders,
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
        (status = 400, description = "Invalid transaction request or version headers", body = crate::http::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error during transaction handling", body = crate::http::openapi::RuesErrorResponse),
        (status = 501, description = "Transactions handler not configured or topic unsupported", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn transactions_post(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    let request = rues::ParsedRuesRequest::component(
        "transactions",
        &topic,
        headers,
        body,
    )?;
    let result = match state.services.chain_handler() {
        Some(chain) => chain.transactions(&topic, request.event()).await,
        None => Err(HttpError::Unsupported),
    };
    request.into_response(result)
}

/// Subscribe to account and transaction state changes.
///
/// The `Rusk-Session-Id` header must contain the session identifier emitted by
/// the `/on` WebSocket handshake for the subscriptions to be routed correctly.
#[utoipa::path(
    get,
    path = "/transactions/{topic}",
    tag = "RUES / Events",
    params(
        crate::http::openapi::VersionHeaders,
        crate::http::openapi::SessionHeader,
        ("topic" = String, Path, description = "Transaction event topic to monitor")
    ),
    responses(
        (status = 200, description = "Subscription registered"),
        (status = 424, description = "Session ID missing or invalid", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Failed to register the subscription", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn transactions_subscribe(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    session_id: SessionId,
) -> Result<Response<Body>, ApiError> {
    rues::subscribe("transactions", None, &topic, session_id, state.sockets_map)
        .await
}

/// Subscribe to transaction events for a specific transaction identifier.
///
/// This preserves the legacy RUES `transactions:{entity}/{topic}` route shape
/// used by clients subscribing to a single transaction lifecycle.
#[utoipa::path(
    get,
    path = "/transactions:{entity}/{topic}",
    tag = "RUES / Events",
    params(
        crate::http::openapi::VersionHeaders,
        crate::http::openapi::SessionHeader,
        ("entity" = String, Path, description = "Transaction identifier to monitor"),
        ("topic" = String, Path, description = "Transaction event topic to monitor")
    ),
    responses(
        (status = 200, description = "Transaction subscription registered"),
        (status = 424, description = "Session ID missing or invalid", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Failed to register the subscription", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn transactions_entity_subscribe(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    session_id: SessionId,
) -> Result<Response<Body>, ApiError> {
    rues::subscribe(
        "transactions",
        Some(&entity),
        &topic,
        session_id,
        state.sockets_map,
    )
    .await
}

/// Unsubscribe from account and transaction updates.
///
/// The `Rusk-Session-Id` header must contain the session identifier emitted by
/// the `/on` WebSocket handshake.
#[utoipa::path(
    delete,
    path = "/transactions/{topic}",
    tag = "RUES / Events",
    params(
        crate::http::openapi::VersionHeaders,
        crate::http::openapi::SessionHeader,
        ("topic" = String, Path, description = "Transaction event topic subscription to remove")
    ),
    responses(
        (status = 200, description = "Subscription removed"),
        (status = 404, description = "Subscription not found", body = crate::http::openapi::RuesErrorResponse),
        (status = 424, description = "Session ID missing or invalid", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Failed to remove the subscription", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn transactions_unsubscribe(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    session_id: SessionId,
) -> Result<Response<Body>, ApiError> {
    rues::unsubscribe(
        "transactions",
        None,
        &topic,
        session_id,
        state.sockets_map,
    )
    .await
}

/// Unsubscribe from transaction events for a specific transaction identifier.
#[utoipa::path(
    delete,
    path = "/transactions:{entity}/{topic}",
    tag = "RUES / Events",
    params(
        crate::http::openapi::VersionHeaders,
        crate::http::openapi::SessionHeader,
        ("entity" = String, Path, description = "Transaction identifier being monitored"),
        ("topic" = String, Path, description = "Transaction event subscription to remove")
    ),
    responses(
        (status = 200, description = "Transaction subscription removed"),
        (status = 404, description = "Subscription not found", body = crate::http::openapi::RuesErrorResponse),
        (status = 424, description = "Session ID missing or invalid", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Failed to remove the subscription", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn transactions_entity_unsubscribe(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    session_id: SessionId,
) -> Result<Response<Body>, ApiError> {
    rues::unsubscribe(
        "transactions",
        Some(&entity),
        &topic,
        session_id,
        state.sockets_map,
    )
    .await
}
