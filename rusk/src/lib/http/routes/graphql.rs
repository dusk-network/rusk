use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, Response, StatusCode};
use tower_http::limit::RequestBodyLimitLayer;
use utoipa_axum::router::{OpenApiRouter, UtoipaMethodRouterExt};
use utoipa_axum::routes;

use crate::http::error::ApiError;
use crate::http::{
    HttpAppState, MAX_GRAPHQL_REQUEST_BODY_BYTES, graphql as graphql_http,
};

pub(crate) fn graphql_routes(
    router: OpenApiRouter<HttpAppState>,
) -> OpenApiRouter<HttpAppState> {
    router.routes(routes!(graphql_get_route)).routes(
        routes!(graphql_post_route)
            .layer(RequestBodyLimitLayer::new(MAX_GRAPHQL_REQUEST_BODY_BYTES)),
    )
}

/// GraphQL HTTP transport endpoint.
///
/// This route documents the standard JSON GraphQL-over-HTTP transport only. It
/// does not attempt to expose the full GraphQL schema through OpenAPI.
#[utoipa::path(
    get,
    path = "/graphql",
    tag = "GraphQL",
    params(crate::http::openapi::GraphqlGetParams),
    responses(
        (status = 200, description = "GraphQL IDE or endpoint information"),
        (
            status = 400,
            description = "Invalid GraphQL request",
            body = crate::http::openapi::GraphqlHttpResponse,
            content_type = "application/json"
        ),
        (
            status = 404,
            description = "GraphQL endpoint not configured",
            body = crate::http::openapi::GraphqlHttpResponse,
            content_type = "application/json"
        )
    )
)]
async fn graphql_get_route(
    State(state): State<HttpAppState>,
    req: Request<Body>,
) -> Result<Response<Body>, ApiError> {
    let handler = match state.services.graphql_handler() {
        Some(handler) => handler,
        None => {
            return Ok(graphql_http::handle_graphql_http_error(
                StatusCode::NOT_FOUND,
                "GraphQL endpoint not configured",
            )
            .expect("GraphQL error response should be built"));
        }
    };

    graphql_http::handle_graphql_get(handler.as_ref(), req)
        .await
        .map_err(ApiError::from)
}

/// GraphQL HTTP transport endpoint.
///
/// Clients send a JSON GraphQL request body to the canonical `/graphql`
/// endpoint. `Rusk-Version` is included in normal server responses; clients may
/// also send `Rusk-Version` and `Rusk-Version-Strict` headers when they need
/// explicit version enforcement.
#[utoipa::path(
    post,
    path = "/graphql",
    tag = "GraphQL",
    request_body(
        content = crate::http::openapi::GraphqlHttpRequest,
        content_type = "application/json"
    ),
    responses(
        (
            status = 200,
            description = "Successful GraphQL response",
            body = crate::http::openapi::GraphqlHttpResponse,
            content_type = "application/json"
        ),
        (
            status = 400,
            description = "Invalid GraphQL request",
            body = crate::http::openapi::GraphqlHttpResponse,
            content_type = "application/json"
        ),
        (
            status = 404,
            description = "GraphQL endpoint not configured",
            body = crate::http::openapi::GraphqlHttpResponse,
            content_type = "application/json"
        ),
        (status = 413, description = "Request body exceeds the GraphQL size limit")
    )
)]
pub(crate) async fn graphql_post_route(
    State(state): State<HttpAppState>,
    req: Request<Body>,
) -> Result<Response<Body>, ApiError> {
    let handler = match state.services.graphql_handler() {
        Some(handler) => handler,
        None => {
            return Ok(graphql_http::handle_graphql_http_error(
                StatusCode::NOT_FOUND,
                "GraphQL endpoint not configured",
            )
            .expect("GraphQL error response should be built"));
        }
    };

    graphql_http::handle_graphql_post(handler.as_ref(), req)
        .await
        .map_err(ApiError::from)
}
