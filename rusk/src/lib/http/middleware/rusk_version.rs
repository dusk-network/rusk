use axum::body::Body;
use axum::http::{Request, Response};
use axum::middleware::Next;

use crate::http::error::ApiError;
use crate::http::rues;

pub(crate) async fn rusk_version_middleware(
    req: Request<Body>,
    next: Next,
) -> Result<Response<Body>, ApiError> {
    rues::validate_rusk_version_headers(req.headers())?;
    Ok(next.run(req).await)
}
