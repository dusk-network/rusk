use axum::body::Body;
use axum::http::header::{HeaderName, HeaderValue};
use axum::http::{Request, Response};
use axum::middleware::Next;

const DEPRECATION_HEADER: &str = "deprecation";
const DEPRECATION_NOTE_HEADER: &str = "deprecation-note";
const DEPRECATION_NOTE: &str =
    "This endpoint is deprecated and scheduled for removal";

pub(crate) async fn deprecation_notice_middleware(
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert(
        HeaderName::from_static(DEPRECATION_HEADER),
        HeaderValue::from_static("true"),
    );
    headers.insert(
        HeaderName::from_static(DEPRECATION_NOTE_HEADER),
        HeaderValue::from_static(DEPRECATION_NOTE),
    );

    response
}
