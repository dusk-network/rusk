// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod graphql;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use node::database::rocksdb::Backend;
use node::network::Kadcast;
use node::Network;
use node_data::message::Message;

use graphql::{DBContext, Query};

use async_graphql::{
    EmptyMutation, EmptySubscription, Name, Schema, Variables,
};
use serde_json::json;

use super::event::{
    Event, MessageRequest, MessageResponse, RequestData, ResponseData, Target,
};
use crate::http::RuskNode;

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

impl RuskNode {
    pub(crate) async fn handle_request(
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
            _ => anyhow::bail!("Unsupported"),
        }
    }

    async fn handle_gql(
        &self,
        request: &MessageRequest,
    ) -> anyhow::Result<ResponseData> {
        let gql_query = request.event.data.as_string();

        let schema = Schema::build(Query, EmptyMutation, EmptySubscription)
            .data(self.db())
            .finish();

        if gql_query.trim().is_empty() {
            return Ok(schema.sdl().into());
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
        Ok(data.into())
    }

    async fn propagate_tx(&self, tx: &[u8]) -> anyhow::Result<ResponseData> {
        let tx = phoenix_core::Transaction::from_slice(tx)
            .map_err(|e| anyhow::anyhow!("Invalid Data {e:?}"))?
            .into();
        let tx_message = Message::new_transaction(Box::new(tx));

        let network = self.0.network();
        network.read().await.route_internal(tx_message);

        Ok(ResponseData::None)
    }

    async fn alive_nodes(&self, amount: usize) -> anyhow::Result<ResponseData> {
        let nodes = self.0.network().read().await.alive_nodes(amount).await;
        let nodes: Vec<_> = nodes.iter().map(|n| n.to_string()).collect();
        Ok(serde_json::to_value(nodes)?.into())
    }
}
