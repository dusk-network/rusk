use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Response};
use tower_http::limit::RequestBodyLimitLayer;
use utoipa_axum::router::{OpenApiRouter, UtoipaMethodRouterExt};
use utoipa_axum::routes;

use crate::http::error::ApiError;
use crate::http::routes::on::TopicPath;
use crate::http::{HttpAppState, HttpError, MAX_RUES_REQUEST_BODY_BYTES, rues};

pub(crate) fn prover_routes(
    router: OpenApiRouter<HttpAppState>,
) -> OpenApiRouter<HttpAppState> {
    router.routes(
        routes!(prover_post)
            .layer(RequestBodyLimitLayer::new(MAX_RUES_REQUEST_BODY_BYTES)),
    )
}

/// Zero-knowledge prover operations and proof generation.
///
/// Submit binary-encoded prover requests. The currently supported topic is
/// `prove`.
#[utoipa::path(
    post,
    path = "/prover/{topic}",
    tag = "RUES / Dispatch",
    params(
        crate::http::openapi::VersionHeaders,
        ("topic" = String, Path, description = "Prover topic: prove")
    ),
    request_body(
        content = String,
        content_type = "application/octet-stream",
        description = "Binary-encoded proof input."
    ),
    responses(
        (status = 200, description = "Generated proof bytes", body = String, content_type = "application/octet-stream"),
        (status = 400, description = "Invalid prover request or version headers", body = crate::http::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Internal prover failure", body = crate::http::openapi::RuesErrorResponse),
        (status = 501, description = "Prover handler not configured or topic unsupported", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn prover_post(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    let request =
        rues::ParsedRuesRequest::component("prover", &topic, headers, body)?;
    let result = match (topic.as_str(), state.services.prover_handler()) {
        ("prove", Some(prover)) => prover.prove(request.event()).await,
        _ => Err(HttpError::Unsupported),
    };
    request.into_response(result)
}
