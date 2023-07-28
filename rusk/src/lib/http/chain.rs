// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod graphql;

use juniper::graphql_value;
use juniper::InputValue;
use node::database::rocksdb::Backend;
use node::network::Kadcast;

use juniper::EmptyMutation;
use juniper::EmptySubscription;
use juniper::Variables;
use std::collections::HashMap;
use std::sync::Arc;

use graphql::{Ctx, Query};

use super::event::{DataType, Request, Response, Target};
use crate::chain::RuskNode;

type Schema = juniper::RootNode<
    'static,
    Query,
    EmptyMutation<Ctx>,
    EmptySubscription<Ctx>,
>;

const GQL_VAR_PREFIX: &str = "gqlvar-";

fn variables_from_request(request: &Request) -> HashMap<String, InputValue> {
    let a = request
        .headers
        .iter()
        .filter_map(|(h, v)| {
            h.starts_with(GQL_VAR_PREFIX).then(|| {
                (
                    h.replacen(GQL_VAR_PREFIX, "", 1),
                    InputValue::variable(v.to_string()),
                )
            })
        })
        .collect();

    println!("{a:?}");
    a
}

impl RuskNode {
    pub(crate) async fn handle_request(&self, request: Request) -> Response {
        match &request.target {
            Target::Host(s) if s == "Chain" && request.topic == "gql" => {
                self.handle_gql(request).await
            }
            _ => Response {
                data: DataType::None,
                headers: request.x_headers(),
                error: Some("Unsupported".into()),
            },
        }
    }

    async fn handle_gql(&self, request: Request) -> Response {
        let ctx = Ctx(self.db());

        let gql_query = match &request.data {
            DataType::Text(str) => str.clone(),
            DataType::Binary(data) => {
                String::from_utf8(data.inner.clone()).unwrap_or_default()
            }
            DataType::None => String::default(),
        };

        match juniper::execute(
            &gql_query,
            None,
            &Schema::new(Query, EmptyMutation::new(), EmptySubscription::new()),
            &variables_from_request(&request),
            &ctx,
        )
        .await
        {
            Err(e) => Response {
                data: DataType::None,
                headers: request.x_headers(),
                error: format!("{e}").into(),
            },
            Ok((res, errors)) => Response {
                data: format!("{res}").into(),
                headers: request.x_headers(),
                error: Some(format!("{errors:?}")),
            },
        }
    }
}
