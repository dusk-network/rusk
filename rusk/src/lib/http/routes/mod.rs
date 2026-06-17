use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::http::{HttpAppState, openapi, rues};

#[cfg(feature = "chain")]
pub(crate) mod graphql;
pub(crate) mod on;
#[cfg(feature = "http-wasm")]
mod static_assets;

/// This is the main Axum application router for Rusk's HTTP API, including both
/// the WebSocket `/on` routes and the RESTful API routes. The OpenAPI
/// documentation is generated from this router, so all API endpoints must be
/// defined here to be included in the docs.
pub(crate) fn router() -> OpenApiRouter<HttpAppState> {
    let router = openapi::router()
        .routes(routes!(rues::ws::handle_rues_ws))
        .nest("/on", on::router());

    // /graphql
    // Canonical GraphQL over HTTP endpoint
    #[cfg(feature = "chain")]
    let router = graphql::graphql_routes(router);

    // /static/drivers/{file}
    // Static file serving for WASM drivers
    #[cfg(feature = "http-wasm")]
    let router = static_assets::static_routes(router);

    router
}
