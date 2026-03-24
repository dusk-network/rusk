use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Response};
use tower_http::limit::RequestBodyLimitLayer;
use utoipa_axum::router::{OpenApiRouter, UtoipaMethodRouterExt};
use utoipa_axum::routes;

use crate::http::error::ApiError;
use crate::http::routes::on::EntityTopicPath;
use crate::http::{HttpAppState, HttpError, MAX_RUES_REQUEST_BODY_BYTES, rues};

pub(crate) fn account_and_blob_routes(
    router: OpenApiRouter<HttpAppState>,
) -> OpenApiRouter<HttpAppState> {
    router
        .routes(
            routes!(account_post)
                .layer(RequestBodyLimitLayer::new(MAX_RUES_REQUEST_BODY_BYTES)),
        )
        .routes(
            routes!(blobs_post)
                .layer(RequestBodyLimitLayer::new(MAX_RUES_REQUEST_BODY_BYTES)),
        )
}

/// Account state and balance queries for a specific account.
///
/// Query account status (balance, nonce, next nonce). The supported topic is
/// `status`. The entity parameter is a base58-encoded BLS public key.
#[utoipa::path(
    post,
    path = "/account:{entity}/{topic}",
    tag = "RUES / Dispatch",
    params(
        crate::http::openapi::VersionHeaders,
        ("entity" = String, Path, description = "Base58-encoded BLS public key"),
        ("topic" = String, Path, description = "Account topic: status")
    ),
    responses(
        (status = 200, description = "Account status response", body = serde_json::Value, content_type = "application/json"),
        (status = 400, description = "Invalid account query or version headers", body = crate::http::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while resolving the account query", body = crate::http::openapi::RuesErrorResponse),
        (status = 501, description = "Account handler not configured or topic unsupported", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn account_post(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    let request = rues::ParsedRuesRequest::entity(
        "account", &entity, &topic, headers, body,
    )?;
    let result = match state.services.chain_handler() {
        Some(chain) => chain.account(&entity, &topic, request.event()).await,
        None => Err(HttpError::Unsupported),
    };
    request.into_response(result)
}

/// Binary large object storage and retrieval operations.
///
/// Query blob data by commitment or by hash. Supported topics are `commitment`
/// and `hash`. The entity parameter is a hex-encoded commitment or hash value.
#[utoipa::path(
    post,
    path = "/blobs:{entity}/{topic}",
    tag = "RUES / Dispatch",
    params(
        crate::http::openapi::VersionHeaders,
        ("entity" = String, Path, description = "Hex-encoded blob commitment or hash"),
        ("topic" = String, Path, description = "BLOB topic: commitment | hash")
    ),
    responses(
        (status = 200, description = "Blob response", content(
            (serde_json::Value = "application/json"),
            (String = "application/octet-stream"),
            (String = "text/plain")
        )),
        (status = 400, description = "Invalid blob query or version headers", body = crate::http::openapi::RuesErrorResponse),
        (status = 404, description = "Blob not found", body = crate::http::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while resolving the blob query", body = crate::http::openapi::RuesErrorResponse),
        (status = 501, description = "Blob handler not configured or topic unsupported", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn blobs_post(
    State(state): State<HttpAppState>,
    Path(EntityTopicPath { entity, topic }): Path<EntityTopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    let request = rues::ParsedRuesRequest::entity(
        "blobs", &entity, &topic, headers, body,
    )?;
    let result = match state.services.chain_handler() {
        Some(chain) => chain.blobs(&entity, &topic, request.event()).await,
        None => Err(HttpError::Unsupported),
    };
    request.into_response(result)
}
