// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod graphql;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use node::database::rocksdb::{Backend, DBTransaction};
use node::database::{Mempool, DB};
use node::network::Kadcast;
use node::Network;
use node_data::ledger::Transaction;
use node_data::message::Message;

use graphql::{DBContext, Query};

use async_graphql::{
    EmptyMutation, EmptySubscription, Name, Schema, Variables,
};
use serde_json::json;

use super::*;
use crate::http::RuskNode;
use crate::{VERSION, VERSION_BUILD};

const GQL_VAR_PREFIX: &str = "rusk-gqlvar-";

fn variables_from_request(request: &MessageRequest) -> Variables {
    let mut var = Variables::default();
    request
        .headers
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
    async fn handle(
        &self,
        request: &MessageRequest,
    ) -> anyhow::Result<ResponseData> {
        match &request.event.to_route() {
            (Target::Host(_), "Chain", "gql") => self.handle_gql(request).await,
            (Target::Host(_), "Chain", "propagate_tx") => {
                self.propagate_tx(request.event_data()).await
            }
            (Target::Host(_), "Chain", "alive_nodes") => {
                let amount = request.event.data.as_string().trim().parse()?;
                self.alive_nodes(amount).await
            }
            (Target::Host(_), "Chain", "info") => self.get_info().await,
            (Target::Host(_), "Chain", "gas") => self.get_gas_price().await,
            _ => anyhow::bail!("Unsupported"),
        }
    }
}
impl RuskNode {
    async fn handle_gql(
        &self,
        request: &MessageRequest,
    ) -> anyhow::Result<ResponseData> {
        let gql_query = request.event.data.as_string();

        let schema = Schema::build(Query, EmptyMutation, EmptySubscription)
            .data(self.db())
            .finish();

        if gql_query.trim().is_empty() {
            return Ok(ResponseData::new(schema.sdl()));
        }

        let variables = variables_from_request(request);
        let gql_query =
            async_graphql::Request::new(gql_query).variables(variables);

        let gql_res = schema.execute(gql_query).await;
        let async_graphql::Response { data, errors, .. } = gql_res;
        if !errors.is_empty() {
            return Err(anyhow::anyhow!("{errors:?}"));
        }
        let data = serde_json::to_value(&data)
            .map_err(|e| anyhow::anyhow!("Cannot parse response {e}"))?;
        Ok(ResponseData::new(data))
    }

    async fn propagate_tx(&self, tx: &[u8]) -> anyhow::Result<ResponseData> {
        let tx = phoenix_core::Transaction::from_slice(tx)
            .map_err(|e| anyhow::anyhow!("Invalid Data {e:?}"))?
            .into();
        let tx_message = Message::new_transaction(Box::new(tx));

        let network = self.0.network();
        network.read().await.route_internal(tx_message);

        Ok(ResponseData::new(DataType::None))
    }

    async fn alive_nodes(&self, amount: usize) -> anyhow::Result<ResponseData> {
        let nodes = self.0.network().read().await.alive_nodes(amount).await;
        let nodes: Vec<_> = nodes.iter().map(|n| n.to_string()).collect();
        Ok(ResponseData::new(serde_json::to_value(nodes)?))
    }

    async fn get_info(&self) -> anyhow::Result<ResponseData> {
        let mut info: HashMap<&str, serde_json::Value> = HashMap::new();
        info.insert("version", VERSION.as_str().into());
        info.insert("version_build", VERSION_BUILD.as_str().into());

        let n_conf = self.network().read().await.conf().clone();
        info.insert("bootstrapping_nodes", n_conf.bootstrapping_nodes.into());
        info.insert("chain_id", n_conf.kadcast_id.into());
        info.insert("kadcast_address", n_conf.public_address.into());

        Ok(ResponseData::new(serde_json::to_value(&info)?))
    }

    /// Calculates the average gas price of transactions in the mempool.
    ///
    /// It retrieves, at maximum, the top 100 transactions sorted by descending
    /// order by the gas price, and calculates the average gas price. If there
    /// are no transactions available in the mempool, it defaults to a gas price
    /// of 1.
    async fn get_gas_price(&self) -> anyhow::Result<ResponseData> {
        let average_gas_price =
            self.db().read().await.view(|t| -> anyhow::Result<u64> {
                let (total, count) = t
                    .get_txs_sorted_by_fee()?
                    .take(100) // TODO: Figure out a sane default
                    .map(|tx| tx.inner.fee().gas_price)
                    .fold((0u64, 0u64), |(total, count), gas_price| {
                        (total + gas_price, count + 1)
                    });

                match count {
                    0 => Ok(1),
                    _ => Ok(total / count), // TODO: Proper rounding up
                }
            })?;
        // TODO: Consider returning more than a singular value
        Ok(ResponseData::new(serde_json::to_value(average_gas_price)?))
    }
}
