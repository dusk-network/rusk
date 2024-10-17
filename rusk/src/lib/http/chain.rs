// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub(super) mod error;
mod geo;
pub mod graphql;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use execution_core::transfer::Transaction as ProtocolTransaction;
use node::database::rocksdb::{Backend, DBTransaction};
use node::database::{Mempool, DB};
use node::mempool::MempoolSrv;
use node::network::Kadcast;
use node::Network;
use node_data::ledger::Transaction;
use node_data::message::Message;
use thiserror::Error;

use error::{ChainError, ChainResult};
use graphql::{DBContext, Query};

use async_graphql::{
    EmptyMutation, EmptySubscription, Name, Schema, ServerError,
    Value as GqlValue, Variables,
};
use serde_json::{json, Map, Value};

use super::*;
use crate::node::RuskNode;
use crate::{VERSION, VERSION_BUILD};

const GQL_VAR_PREFIX: &str = "rusk-gqlvar-";

fn variables_from_headers(headers: &Map<String, Value>) -> Variables {
    let mut var = Variables::default();
    headers
        .iter()
        .filter_map(|(h, v)| {
            let h = h.to_lowercase();
            h.starts_with(GQL_VAR_PREFIX).then(|| {
                (h.replacen(GQL_VAR_PREFIX, "", 1), async_graphql::value!(v))
            })
        })
        .for_each(|(k, v)| {
            var.insert(Name::new(k), v);
        });

    var
}

#[async_trait]
impl HandleRequest for RuskNode {
    fn can_handle(&self, request: &MessageRequest) -> bool {
        let route = request.event.to_route();
        if matches!(route, (Target::Host(_), "rusk", "preverify")) {
            return true;
        }
        matches!(route, (Target::Host(_), "Chain", _))
    }

    fn can_handle_rues(&self, request: &RuesDispatchEvent) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match request.uri.inner() {
            ("graphql", _, "query") => true,
            ("transactions", _, "preverify") => true,
            ("transactions", _, "propagate") => true,
            ("network", _, "peers") => true,
            ("network", _, "peers_location") => true,
            ("node", _, "info") => true,
            ("blocks", _, "gas-price") => true,
            _ => false,
        }
    }
    async fn handle_rues(
        &self,
        request: &RuesDispatchEvent,
    ) -> RequestResult<ResponseData> {
        match request.uri.inner() {
            ("graphql", _, "query") => {
                Ok(self.handle_gql(&request.data, &request.headers).await?)
                // ToDo: this should not be this deep in the node
            }
            ("transactions", _, "preverify") => {
                Ok(self.handle_preverify(request.data.as_bytes()).await?)
            }
            ("transactions", _, "propagate") => {
                Ok(self.propagate_tx(request.data.as_bytes()).await?)
            }
            ("network", _, "peers") => {
                let amount = request.data.as_string().trim().parse()?;
                Ok(self.alive_nodes(amount).await?)
            }

            ("network", _, "peers_location") => {
                Ok(self.peers_location().await?)
            }
            ("node", _, "info") => Ok(self.get_info().await?),
            ("blocks", _, "gas-price") => {
                let max_transactions = request
                    .data
                    .as_string()
                    .trim()
                    .parse::<usize>()
                    .unwrap_or(usize::MAX);
                Ok(self.get_gas_price(max_transactions).await?)
            }
            _ => return Err(RequestError::UnsupportedRuesEventUri),
        }
    }
    async fn handle(
        &self,
        request: &MessageRequest,
    ) -> RequestResult<ResponseData> {
        match &request.event.to_route() {
            (Target::Host(_), "Chain", "gql") => Ok(self
                .handle_gql(&request.event.data, &request.headers)
                .await?),
            (Target::Host(_), "rusk", "preverify") => {
                Ok(self.handle_preverify(request.event_data()).await?)
            }
            (Target::Host(_), "Chain", "propagate_tx") => {
                Ok(self.propagate_tx(request.event_data()).await?)
            }
            (Target::Host(_), "Chain", "alive_nodes") => {
                let amount = request.event.data.as_string().trim().parse()?;
                Ok(self.alive_nodes(amount).await?)
            }
            (Target::Host(_), "Chain", "info") => Ok(self.get_info().await?),
            (Target::Host(_), "Chain", "gas") => {
                let max_transactions = request
                    .event
                    .data
                    .as_string()
                    .trim()
                    .parse::<usize>()
                    .unwrap_or(usize::MAX);
                Ok(self.get_gas_price(max_transactions).await?)
            }
            _ => return Err(RequestError::UnsupportedRuesEventTopic),
        }
    }
}

impl RuskNode {
    async fn handle_gql(
        &self,
        data: &RequestData,
        headers: &serde_json::Map<String, Value>,
    ) -> ChainResult<ResponseData> {
        let gql_query = data.as_string();

        // ToDo: do we want to build the schema every time?
        #[cfg(feature = "archive")]
        let schema = Schema::build(Query, EmptyMutation, EmptySubscription)
            .data((self.db(), self.archive()))
            .finish();
        #[cfg(not(feature = "archive"))]
        let schema = Schema::build(Query, EmptyMutation, EmptySubscription)
            .data((self.db(), ()))
            .finish();

        // If the gql query is empty, return the schema early
        if gql_query.trim().is_empty() {
            return Ok(ResponseData::new(schema.sdl()));
        }

        let variables = variables_from_headers(headers);
        let gql_request =
            async_graphql::Request::new(gql_query).variables(variables);

        let data: GqlValue = graphql::gql_execute(gql_request, schema).await?;

        let data = serde_json::to_value(&data)?;
        Ok(ResponseData::new(data))
    }

    async fn handle_preverify(&self, data: &[u8]) -> ChainResult<ResponseData> {
        let tx = execution_core::transfer::Transaction::from_slice(data)
            .map_err(|_| ChainError::Bytes)?;
        let db = self.inner().database();
        let vm = self.inner().vm_handler();
        MempoolSrv::check_tx(&db, &vm, &tx.into(), true, usize::MAX).await?;
        Ok(ResponseData::new(DataType::None))
    }

    async fn propagate_tx(&self, tx: &[u8]) -> ChainResult<ResponseData> {
        let tx: Transaction = ProtocolTransaction::from_slice(tx)
            .map_err(|_| ChainError::Bytes)?
            .into();
        let tx_message = tx.into();

        let network = self.network();
        network.read().await.route_internal(tx_message);

        Ok(ResponseData::new(DataType::None))
    }

    async fn alive_nodes(&self, amount: usize) -> ChainResult<ResponseData> {
        let nodes = self.network().read().await.alive_nodes(amount).await;
        let nodes: Vec<_> = nodes.iter().map(|n| n.to_string()).collect();
        let data = serde_json::to_value(nodes)?;
        Ok(ResponseData::new(data))
    }

    async fn get_info(&self) -> ChainResult<ResponseData> {
        let mut info: HashMap<&str, serde_json::Value> = HashMap::new();
        info.insert("version", VERSION.as_str().into());
        info.insert("version_build", VERSION_BUILD.as_str().into());

        let n_conf = self.network().read().await.conf().clone();
        info.insert("bootstrapping_nodes", n_conf.bootstrapping_nodes.into());
        info.insert("chain_id", n_conf.kadcast_id.into());
        info.insert("kadcast_address", n_conf.public_address.into());

        Ok(ResponseData::new(serde_json::to_value(&info)?))
    }

    /// Calculates various statistics for gas prices of transactions in the
    /// mempool.
    ///
    /// It retrieves a specified number of transactions, sorted by descending
    /// gas price, and calculates the average, maximum, minimum and median
    /// prices. In the absence of transactions, will
    /// default to a gas price of 1.
    ///
    /// # Arguments
    /// * `max_transactions` - Maximum number of transactions to consider.
    ///
    /// # Returns
    /// A JSON object encapsulating the statistics, or an error if processing
    /// fails.
    async fn get_gas_price(
        &self,
        max_transactions: usize,
    ) -> ChainResult<ResponseData> {
        let gas_prices: Vec<u64> =
            self.db()
                .read()
                .await
                .view(|t| -> anyhow::Result<Vec<u64>> {
                    Ok(t.mempool_txs_ids_sorted_by_fee()?
                        .take(max_transactions)
                        .map(|(gas_price, _)| gas_price)
                        .collect())
                })?;

        if gas_prices.is_empty() {
            let stats = serde_json::json!({ "average": 1, "max": 1, "median": 1, "min": 1 });
            return Ok(ResponseData::new(serde_json::to_value(stats)?));
        }

        let mean_gas_price = {
            let total: u64 = gas_prices.iter().sum();
            let count = gas_prices.len() as u64;
            // ceiling division to round up
            (total + count - 1) / count
        };

        let max_gas_price = *gas_prices.iter().max().unwrap();

        let median_gas_price = {
            let mid = gas_prices.len() / 2;
            if gas_prices.len() % 2 == 0 {
                (gas_prices[mid - 1] + gas_prices[mid]) / 2
            } else {
                gas_prices[mid]
            }
        };

        let min_gas_price = *gas_prices.iter().min().unwrap();

        let stats = serde_json::json!({
            "average": mean_gas_price,
            "max": max_gas_price,
            "median": median_gas_price,
            "min": min_gas_price
        });

        Ok(ResponseData::new(serde_json::to_value(stats)?))
    }
}
