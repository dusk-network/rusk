#![cfg_attr(
    not(any(feature = "chain", feature = "prover", test)),
    allow(dead_code)
)]

use axum::middleware::from_fn;
use serde::Deserialize;
use utoipa_axum::router::OpenApiRouter;

use crate::http::HttpAppState;
use crate::http::middleware::rusk_version_middleware;

#[cfg(feature = "chain")]
mod accounts;
#[cfg(feature = "chain")]
mod blocks;
#[cfg(feature = "chain")]
mod contracts;
#[cfg(feature = "chain")]
mod graphql;
#[cfg(feature = "chain")]
mod network;
#[cfg(feature = "prover")]
mod prover;
#[cfg(test)]
mod test;
#[cfg(feature = "chain")]
mod transactions;

#[derive(Deserialize)]
pub(crate) struct TopicPath {
    pub(crate) topic: String,
}

#[derive(Deserialize)]
#[cfg(feature = "chain")]
pub(crate) struct EntityTopicPath {
    pub(crate) entity: String,
    pub(crate) topic: String,
}

/// This is the router for the WebSocket `/on` endpoints that handle RUES
/// exclusively.
///
/// It is nested under the main router at the `/on` path. All routes defined
/// here will be prefixed with `/on`.
pub(crate) fn router() -> OpenApiRouter<HttpAppState> {
    let router = OpenApiRouter::default();

    #[cfg(feature = "chain")]
    let router = transactions::transaction_routes(router);
    #[cfg(feature = "chain")]
    let router = network::network_and_node_routes(router);
    #[cfg(feature = "chain")]
    let router = blocks::block_and_stats_routes(router);
    #[cfg(feature = "chain")]
    let router = accounts::account_and_blob_routes(router);
    #[cfg(feature = "chain")]
    let router = contracts::contract_routes(router);
    #[cfg(feature = "chain")]
    let router = graphql::legacy_graphql_routes(router);

    #[cfg(feature = "prover")]
    let router = prover::prover_routes(router);

    #[cfg(test)]
    let router = test::test_routes(router);

    router.route_layer(from_fn(rusk_version_middleware))
}
