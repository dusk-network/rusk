use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{HeaderMap, Response};
use tower_http::limit::RequestBodyLimitLayer;
use utoipa_axum::router::{OpenApiRouter, UtoipaMethodRouterExt};
use utoipa_axum::routes;

use crate::http::error::ApiError;
use crate::http::{HttpAppState, HttpError, MAX_RUES_REQUEST_BODY_BYTES, rues};

pub(crate) fn legacy_graphql_routes(
    router: OpenApiRouter<HttpAppState>,
) -> OpenApiRouter<HttpAppState> {
    router.routes(
        routes!(legacy_rues_graphql_post_route)
            .layer(RequestBodyLimitLayer::new(MAX_RUES_REQUEST_BODY_BYTES)),
    )
}

/// Legacy GraphQL endpoint under `/on`.
///
/// This preserves the raw RUES GraphQL behavior:
/// an empty body returns the schema SDL, and non-empty bodies are treated as
/// legacy GraphQL query text rather than GraphQL-over-HTTP JSON.
#[utoipa::path(
    post,
    path = "/graphql/query",
    tag = "RUES / Dispatch",
    params(crate::http::openapi::VersionHeaders),
    request_body(
        content = String,
        content_type = "text/plain",
        description = "Raw GraphQL query text. An empty body returns the schema SDL."
    ),
    responses(
        (
            status = 200,
            description = "Legacy GraphQL response body or schema SDL",
            body = crate::http::openapi::GraphqlHttpResponse,
            content_type = "application/json"
        ),
        (status = 400, description = "Invalid legacy GraphQL request or version headers", body = crate::http::openapi::RuesErrorResponse),
        (status = 413, description = "Request body exceeds the default RUES size limit"),
        (status = 422, description = "Malformed request headers or payload encoding", body = crate::http::openapi::RuesErrorResponse),
        (status = 500, description = "Internal error while executing the legacy GraphQL request", body = crate::http::openapi::RuesErrorResponse),
        (status = 501, description = "Legacy GraphQL handler not configured", body = crate::http::openapi::RuesErrorResponse)
    )
)]
async fn legacy_rues_graphql_post_route(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, ApiError> {
    let request =
        rues::ParsedRuesRequest::component("graphql", "query", headers, body)?;
    let result = match state.services.chain_handler() {
        Some(chain) => chain.graphql_query(request.event()).await,
        None => Err(HttpError::Unsupported),
    };
    request.into_response(result)
}
