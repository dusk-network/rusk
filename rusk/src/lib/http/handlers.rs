// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! HTTP request handlers for various routes. Mainly for RUES.

#![cfg_attr(
    not(any(feature = "chain", feature = "prover", test)),
    allow(dead_code, unused_imports)
)]

use std::sync::Arc;

#[cfg(feature = "chain")]
use async_graphql::{BatchRequest, BatchResponse};
#[cfg(any(feature = "chain", feature = "prover", test))]
use async_trait::async_trait;

use crate::http::rues::event::ResponseData;
use crate::http::{HttpResult, RuesDispatchEvent};

/// Registry of optional handler implementations.
#[derive(Default, Clone)]
pub struct HttpHandlers {
    #[cfg(feature = "chain")]
    /// Chain-owned RUES handlers for transaction, network, account, block,
    /// blob, and chain-state contract routes.
    chain: Option<Arc<dyn ChainRequestHandler>>,
    #[cfg(feature = "chain")]
    /// Rusk-owned RUES handlers for provisioner/CRS, contract query, driver,
    /// and contract metadata routes.
    rusk: Option<Arc<dyn RuskRequestHandler>>,
    #[cfg(feature = "chain")]
    /// Handler for the dedicated `/graphql` HTTP endpoint.
    graphql: Option<Arc<dyn GraphqlHandler>>,
    #[cfg(feature = "prover")]
    /// Handler for proof-generation routes under `/on/prover/*`.
    prover: Option<Arc<dyn ProverRequestHandler>>,
    #[cfg(test)]
    /// Test-only handler surface used by `/on/test/*` routes.
    test: Option<Arc<dyn TestRequestHandler>>,
}

#[cfg(feature = "chain")]
#[async_trait]
pub trait GraphqlHandler: Send + Sync + 'static {
    /// Execute a single or batch request received on the standalone `/graphql`
    /// route.
    async fn execute_graphql(&self, request: BatchRequest) -> BatchResponse;
}

#[cfg(feature = "chain")]
#[async_trait]
pub trait ChainRequestHandler: Send + Sync + 'static {
    /// Handle legacy GraphQL requests routed through `/on/graphql/query`.
    async fn graphql_query(
        &self,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle `/on/transactions/{topic}` requests such as `preverify`,
    /// `propagate`, and `simulate`.
    async fn transactions(
        &self,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle `/on/network/{topic}` requests for peer and network state.
    async fn network(
        &self,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle chain node routes such as `/on/node/info`.
    /// This is node runtime/state information, not Rusk-owned provisioner or
    /// CRS data.
    async fn node(
        &self,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle `/on/account:{entity}/{topic}` requests against chain state.
    async fn account(
        &self,
        entity: &str,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle chain-owned `/on/contract:{entity}/{topic}` topics that expose
    /// chain status for a contract. Currently this is only the `status` topic
    /// (which is the contract balance).
    async fn contract(
        &self,
        entity: &str,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle `/on/blocks/{topic}` requests.
    async fn blocks(
        &self,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle `/on/blobs:{entity}/{topic}` requests.
    async fn blobs(
        &self,
        entity: &str,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle `/on/stats/{topic}` requests for chain-derived statistics.
    async fn stats(
        &self,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
}

#[cfg(feature = "chain")]
#[async_trait]
pub trait RuskRequestHandler: Send + Sync + 'static {
    /// Handle Rusk-owned `/on/node/{topic}` routes such as `provisioners` and
    /// `crs`.
    /// This is auxiliary node data exposed by Rusk, not general node info.
    async fn node(
        &self,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle `/on/contracts:{entity}/{topic}` contract query and call routes.
    /// Unlike `contract`, this route dispatches contract-facing queries
    /// rather than single-contract metadata or driver operations.
    async fn contracts(
        &self,
        entity: &str,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle `/on/driver:{entity}/{topic}` data-driver routes.
    async fn driver(
        &self,
        entity: &str,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle `/on/contract_owner:{entity}/{topic}` owner lookup routes.
    async fn contract_owner(
        &self,
        entity: &str,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
    /// Handle Rusk-owned `/on/contract:{entity}/{topic}` management and
    /// metadata topics such as `upload_driver`, `download_driver`, and
    /// `metadata`. This is operational contract management, not chain-state
    /// status.
    async fn contract(
        &self,
        entity: &str,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
}

#[cfg(feature = "prover")]
#[async_trait]
pub trait ProverRequestHandler: Send + Sync + 'static {
    /// Handle `/on/prover/prove` proof-generation requests.
    async fn prove(
        &self,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
}

#[cfg(test)]
#[async_trait]
pub trait TestRequestHandler: Send + Sync + 'static {
    /// Handle test-only `/on/test/{topic}` requests.
    async fn handle_test(
        &self,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData>;
}

impl HttpHandlers {
    #[cfg(feature = "chain")]
    pub fn set_chain_handler<T>(&mut self, handler: T)
    where
        T: ChainRequestHandler,
    {
        self.chain = Some(Arc::new(handler));
    }

    #[cfg(feature = "chain")]
    pub(crate) fn chain_handler(&self) -> Option<Arc<dyn ChainRequestHandler>> {
        self.chain.clone()
    }

    #[cfg(feature = "chain")]
    pub fn set_rusk_handler<T>(&mut self, handler: T)
    where
        T: RuskRequestHandler,
    {
        self.rusk = Some(Arc::new(handler));
    }

    #[cfg(feature = "chain")]
    pub(crate) fn rusk_handler(&self) -> Option<Arc<dyn RuskRequestHandler>> {
        self.rusk.clone()
    }

    #[cfg(feature = "chain")]
    pub fn set_graphql_handler<T>(&mut self, handler: T)
    where
        T: GraphqlHandler,
    {
        self.graphql = Some(Arc::new(handler));
    }

    #[cfg(feature = "chain")]
    pub(crate) fn graphql_handler(&self) -> Option<Arc<dyn GraphqlHandler>> {
        self.graphql.clone()
    }

    #[cfg(feature = "prover")]
    pub(crate) fn set_prover_handler<T>(&mut self, handler: T)
    where
        T: ProverRequestHandler,
    {
        self.prover = Some(Arc::new(handler));
    }

    #[cfg(feature = "prover")]
    pub(crate) fn prover_handler(
        &self,
    ) -> Option<Arc<dyn ProverRequestHandler>> {
        self.prover.clone()
    }

    #[cfg(test)]
    pub(crate) fn set_test_handler<T>(&mut self, handler: T)
    where
        T: TestRequestHandler,
    {
        self.test = Some(Arc::new(handler));
    }

    #[cfg(test)]
    pub(crate) fn test_handler(&self) -> Option<Arc<dyn TestRequestHandler>> {
        self.test.clone()
    }
}
