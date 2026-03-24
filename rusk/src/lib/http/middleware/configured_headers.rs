use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, Response};
use axum::middleware::Next;

use crate::http::axum_app::HttpAppState;

pub(crate) async fn configured_headers_middleware(
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
