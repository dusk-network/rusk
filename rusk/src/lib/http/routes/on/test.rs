use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Response};
use axum::routing::post;
use tower_http::limit::RequestBodyLimitLayer;
use utoipa_axum::router::OpenApiRouter;

use crate::http::error::ApiError;
use crate::http::routes::on::TopicPath;
use crate::http::{
    HttpAppState, HttpError, MAX_RUES_REQUEST_BODY_BYTES, SessionId, rues,
};

pub(crate) fn test_routes(
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

async fn test_post(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Response<Body>, ApiError> {
    let request =
        rues::ParsedRuesRequest::component("test", &topic, headers, body)?;
    let result = match state.services.test_handler() {
        Some(handler) => handler.handle_test(&topic, request.event()).await,
        None => Err(HttpError::Unsupported),
    };
    request.into_response(result)
}

async fn test_subscribe(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    session_id: SessionId,
) -> Result<Response<Body>, ApiError> {
    rues::subscribe("test", None, &topic, session_id, state.sockets_map).await
}

async fn test_unsubscribe(
    State(state): State<HttpAppState>,
    Path(TopicPath { topic }): Path<TopicPath>,
    session_id: SessionId,
) -> Result<Response<Body>, ApiError> {
    rues::unsubscribe("test", None, &topic, session_id, state.sockets_map).await
}
