// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use axum::Router;
use axum::body::{Body, to_bytes};
#[cfg(feature = "http-wasm")]
use axum::http::StatusCode;
#[cfg(feature = "http-wasm")]
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{Request, Response};
#[cfg(feature = "http-wasm")]
use axum::response::IntoResponse;
#[cfg(feature = "http-wasm")]
use axum::routing::any;
use http_body_util::Full;
use hyper::body::Incoming;
use tower::ServiceExt;

use super::ExecutionError;
use super::event::FullOrStreamBody;

#[cfg(feature = "http-wasm")]
const WALLET_CORE_ALIAS_PATH: &str = "/static/drivers/wallet-core.wasm";
#[cfg(feature = "http-wasm")]
const WALLET_CORE_1_0_1_PATH: &str = "/static/drivers/wallet-core-1.0.1.wasm";
#[cfg(feature = "http-wasm")]
const WALLET_CORE_1_3_0_PATH: &str = "/static/drivers/wallet-core-1.3.0.wasm";
#[cfg(feature = "http-wasm")]
const WALLET_CORE_1_6_0_PATH: &str = "/static/drivers/wallet-core-1.6.0.wasm";

#[cfg(feature = "http-wasm")]
const WASM_CONTENT_TYPE: &str = "application/wasm";
#[cfg(feature = "http-wasm")]
const WASM_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";
#[cfg(feature = "chain")]
const GRAPHQL_PATH: &str = "/graphql";
#[cfg(feature = "chain")]
const GRAPHQL_TRAILING_SLASH_PATH: &str = "/graphql/";
const RUES_ROOT_PATH: &str = "/on";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RouteOwner {
    Legacy,
    Axum,
}

#[derive(Clone)]
pub(super) struct RouteDispatchPlan {
    router: Router,
}

impl RouteDispatchPlan {
    pub(super) fn new() -> Self {
        let router = build_router();
        Self { router }
    }

    pub(super) fn owner_for_path(&self, path: &str) -> RouteOwner {
        if is_axum_path(path) {
            RouteOwner::Axum
        } else {
            RouteOwner::Legacy
        }
    }

    pub(super) async fn handle_axum(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<FullOrStreamBody>, ExecutionError> {
        let req = req.map(Body::new);
        let response = match self.router.clone().oneshot(req).await {
            Ok(response) => response,
            Err(err) => match err {},
        };
        axum_response_to_hyper(response).await
    }
}

impl Default for RouteDispatchPlan {
    fn default() -> Self {
        Self::new()
    }
}

fn build_router() -> Router {
    let router = Router::new();

    #[cfg(feature = "http-wasm")]
    let router = router
        .route(WALLET_CORE_ALIAS_PATH, any(wallet_core_alias))
        .route(WALLET_CORE_1_0_1_PATH, any(wallet_core_1_0_1))
        .route(WALLET_CORE_1_3_0_PATH, any(wallet_core_1_3_0))
        .route(WALLET_CORE_1_6_0_PATH, any(wallet_core_1_6_0));

    router
}

fn is_axum_path(path: &str) -> bool {
    if path == RUES_ROOT_PATH || path.starts_with("/on/") {
        return true;
    }

    #[cfg(feature = "chain")]
    if matches!(path, GRAPHQL_PATH | GRAPHQL_TRAILING_SLASH_PATH) {
        return true;
    }

    #[cfg(feature = "http-wasm")]
    if matches!(
        path,
        WALLET_CORE_ALIAS_PATH
            | WALLET_CORE_1_0_1_PATH
            | WALLET_CORE_1_3_0_PATH
            | WALLET_CORE_1_6_0_PATH
    ) {
        return true;
    }

    let _ = path;
    false
}

async fn axum_response_to_hyper(
    response: Response<Body>,
) -> Result<Response<FullOrStreamBody>, ExecutionError> {
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, usize::MAX)
        .await
        .map_err(|err| ExecutionError::Other(err.to_string()))?;
    Ok(Response::from_parts(parts, Full::new(bytes).into()))
}

#[cfg(feature = "http-wasm")]
async fn wallet_core_alias() -> impl IntoResponse {
    wasm_response(include_bytes!("../../assets/wallet_core-1.0.1.wasm"))
}

#[cfg(feature = "http-wasm")]
async fn wallet_core_1_0_1() -> impl IntoResponse {
    wasm_response(include_bytes!("../../assets/wallet_core-1.0.1.wasm"))
}

#[cfg(feature = "http-wasm")]
async fn wallet_core_1_3_0() -> impl IntoResponse {
    wasm_response(include_bytes!("../../assets/wallet_core-1.3.0.wasm"))
}

#[cfg(feature = "http-wasm")]
async fn wallet_core_1_6_0() -> impl IntoResponse {
    wasm_response(include_bytes!("../../assets/wallet_core-1.6.0.wasm"))
}

#[cfg(feature = "http-wasm")]
fn wasm_response(bytes: &'static [u8]) -> impl IntoResponse {
    (
        StatusCode::OK,
        [
            (CONTENT_TYPE, WASM_CONTENT_TYPE),
            (CACHE_CONTROL, WASM_CACHE_CONTROL),
        ],
        bytes,
    )
}

#[cfg(test)]
mod tests {
    use super::{RouteDispatchPlan, RouteOwner};

    #[test]
    fn route_dispatch_defaults_to_legacy_owner() {
        let dispatch = RouteDispatchPlan::new();
        assert_eq!(dispatch.owner_for_path("/legacy/path"), RouteOwner::Legacy);
    }

    #[cfg(feature = "http-wasm")]
    #[test]
    fn static_wasm_paths_map_to_axum_owner() {
        let dispatch = RouteDispatchPlan::new();
        assert_eq!(
            dispatch.owner_for_path("/static/drivers/wallet-core.wasm"),
            RouteOwner::Axum
        );
        assert_eq!(
            dispatch.owner_for_path("/static/drivers/wallet-core-1.0.1.wasm"),
            RouteOwner::Axum
        );
        assert_eq!(
            dispatch.owner_for_path("/static/drivers/wallet-core-1.3.0.wasm"),
            RouteOwner::Axum
        );
        assert_eq!(
            dispatch.owner_for_path("/static/drivers/wallet-core-1.6.0.wasm"),
            RouteOwner::Axum
        );
    }

    #[cfg(feature = "chain")]
    #[test]
    fn graphql_paths_map_to_axum_owner() {
        let dispatch = RouteDispatchPlan::new();
        assert_eq!(dispatch.owner_for_path("/graphql"), RouteOwner::Axum);
        assert_eq!(dispatch.owner_for_path("/graphql/"), RouteOwner::Axum);
    }

    #[test]
    fn rues_paths_map_to_axum_owner() {
        let dispatch = RouteDispatchPlan::new();
        assert_eq!(dispatch.owner_for_path("/on"), RouteOwner::Axum);
        assert_eq!(dispatch.owner_for_path("/on/test/echo"), RouteOwner::Axum);
    }
}
