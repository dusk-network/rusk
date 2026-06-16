use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Response};
use tower_http::limit::RequestBodyLimitLayer;
use utoipa_axum::router::{OpenApiRouter, UtoipaMethodRouterExt};
use utoipa_axum::routes;

use crate::http::error::ApiError;
use crate::http::routes::on::TopicPath;
use crate::http::{HttpAppState, HttpError, MAX_RUES_REQUEST_BODY_BYTES, rues};

pub(crate) fn network_and_node_routes(
    router: OpenApiRouter<HttpAppState>,
) -> OpenApiRouter<HttpAppState> {
    router
        .routes(
            routes!(network_post)
                .layer(RequestBodyLimitLayer::new(MAX_RUES_REQUEST_BODY_BYTES)),
        )
        .routes(
            routes!(node_post)
                .layer(RequestBodyLimitLayer::new(MAX_RUES_REQUEST_BODY_BYTES)),
        )
}

/// Network topology and peer management operations.
///
/// Submit binary-encoded network queries. Supported topics are `peers` and
/// `peers_location`.
#[utoipa::path(
    post,
    path = "/network/{topic}",
    tag = "RUES / Dispatch",
    params(
        crate::http::openapi::VersionHeaders,
        ("topic" = String, Path, description = "Network topic: peers | peers_location")
    ),
    request_body(
        content = String,
        content_type = "text/plain",
        description = "Optional peer count for the `peers` topic."
    ),
    responses(
        (status = 200, description = "Network query response", body = serde_json::Value, content_type = "application/json"),
        (status = 400, description = "Invalid network request or version headers", body = crate::http::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while resolving the network query", body = crate::http::openapi::RuesErrorResponse),
        (status = 501, description = "Network handler not configured or topic unsupported", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn network_post(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    let request =
        rues::ParsedRuesRequest::component("network", &topic, headers, body)?;
    let result = match state.services.chain_handler() {
        Some(chain) => chain.network(&topic, request.event()).await,
        None => Err(HttpError::Unsupported),
    };
    request.into_response(result)
}

/// Node information and synchronization state queries.
///
/// Submit binary-encoded node queries. Supported topics are `info`
/// (node and chain runtime info), `provisioners` (stake table), and `crs`
/// (common reference string).
#[utoipa::path(
    post,
    path = "/node/{topic}",
    tag = "RUES / Dispatch",
    params(
        crate::http::openapi::VersionHeaders,
        ("topic" = String, Path, description = "Node topic: info | provisioners | crs")
    ),
    responses(
        (status = 200, description = "Node information response", content(
            (serde_json::Value = "application/json"),
            (String = "application/octet-stream")
        )),
        (status = 400, description = "Invalid node request or version headers", body = crate::http::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while resolving the node query", body = crate::http::openapi::RuesErrorResponse),
        (status = 501, description = "Node handler not configured or topic unsupported", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn node_post(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    let request =
        rues::ParsedRuesRequest::component("node", &topic, headers, body)?;
    let result = match topic.as_str() {
        "info" => match state.services.chain_handler() {
            Some(chain) => chain.node(&topic, request.event()).await,
            None => Err(HttpError::Unsupported),
        },
        "provisioners" | "crs" => match state.services.rusk_handler() {
            Some(rusk) => rusk.node(&topic, request.event()).await,
            None => Err(HttpError::Unsupported),
        },
        _ => Err(HttpError::Unsupported),
    };
    request.into_response(result)
}
