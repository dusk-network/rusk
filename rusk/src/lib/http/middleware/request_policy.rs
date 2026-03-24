use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, Response};
use axum::middleware::Next;
use axum::response::IntoResponse;

use crate::http::app_state::HttpAppState;
use crate::http::error::ApiError;

pub(crate) async fn request_policy_middleware(
    State(state): State<HttpAppState>,
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    match state.policy.enforce(&req) {
        Ok(permit) => {
            let response = next.run(req).await;
            drop(permit);
            response
        }
        Err(rejection) => {
            let mut error =
                ApiError::new(rejection.status, rejection.message, "policy");
            if let Some(retry_after) = rejection.retry_after_seconds {
                error = error.with_retry_after(retry_after);
            }
            error.into_response()
        }
    }
}
