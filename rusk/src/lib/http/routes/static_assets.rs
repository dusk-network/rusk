use axum::body::Body;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{Response, StatusCode};
use axum::response::{IntoResponse, Json};
use serde_json::json;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::http::HttpAppState;

const WASM_CONTENT_TYPE: &str = "application/wasm";
const WASM_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

pub(crate) fn static_routes(
    router: OpenApiRouter<HttpAppState>,
) -> OpenApiRouter<HttpAppState> {
    router.routes(routes!(wasm_driver_route))
}

/// Fetch static WASM driver assets.
#[utoipa::path(
    get,
    path = "/static/drivers/{file}",
    tag = "Static",
    params(("file" = String, Path, description = "Driver filename (e.g., transfer.wasm)")),
    responses(
        (status = 200, description = "Driver WASM bytecode", content_type = "application/wasm"),
        (status = 404, description = "Driver file not found", body = crate::http::openapi::ErrorEnvelope, content_type = "application/json")
    )
)]
async fn wasm_driver_route(
    axum::extract::Path(file): axum::extract::Path<String>,
) -> Response<Body> {
    let bytes: Option<&'static [u8]> = match file.as_str() {
        "wallet-core.wasm" | "wallet-core-1.0.1.wasm" => {
            Some(include_bytes!("../../../assets/wallet_core-1.0.1.wasm"))
        }
        "wallet-core-1.3.0.wasm" => {
            Some(include_bytes!("../../../assets/wallet_core-1.3.0.wasm"))
        }
        "wallet-core-1.6.0.wasm" => {
            Some(include_bytes!("../../../assets/wallet_core-1.6.0.wasm"))
        }
        _ => None,
    };

    match bytes {
        Some(wasm) => (
            StatusCode::OK,
            [
                (CONTENT_TYPE, WASM_CONTENT_TYPE),
                (CACHE_CONTROL, WASM_CACHE_CONTROL),
            ],
            wasm,
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Path not found" })),
        )
            .into_response(),
    }
}
