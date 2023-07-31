// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod graphql;

use std::collections::HashMap;
use std::sync::Arc;

use node::database::rocksdb::Backend;
use node::network::Kadcast;

use graphql::{Ctx, Query};

use async_graphql::{
    EmptyMutation, EmptySubscription, Name, Schema, Variables,
};

use super::event::{DataType, Event, MessageRequest, MessageResponse, Target};
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
        request: MessageRequest,
    ) -> MessageResponse {
        match &request.event.target {
            Target::Host(s) if s == "Chain" && request.event.topic == "gql" => {
                self.handle_gql(request).await
            }
            _ => MessageResponse {
                data: DataType::None,
                headers: request.x_headers(),
                error: Some("Unsupported".into()),
            },
        }
    }

    async fn handle_gql(&self, request: MessageRequest) -> MessageResponse {
        let gql_query = match &request.event.data {
            DataType::Text(str) => str.clone(),
            DataType::Binary(data) => {
                String::from_utf8(data.inner.clone()).unwrap_or_default()
            }
            DataType::None => String::default(),
        };

        let schema = Schema::build(Query, EmptyMutation, EmptySubscription)
            .data(self.db())
            .finish();

        let variables = variables_from_request(&request);
        let gql_query =
            async_graphql::Request::new(gql_query).variables(variables);

        let gql_res = schema.execute(gql_query).await;
        let async_graphql::Response { data, errors, .. } = gql_res;

        let data = match serde_json::to_string(&data) {
            Ok(d) => d,
            Err(e) => {
                return MessageResponse {
                    data: data.to_string().into(),
                    headers: request.x_headers(),
                    error: Some("Cannot parse response".into()),
                }
            }
        };

        let errors = (!errors.is_empty()).then(|| format!("{errors:?}"));
        MessageResponse {
            data: data.into(),
            headers: request.x_headers(),
            error: errors,
        }
    }
}
