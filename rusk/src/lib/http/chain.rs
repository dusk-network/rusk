// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod graphql;

use node::database::rocksdb::Backend;
use node::network::Kadcast;

use juniper::EmptyMutation;
use juniper::EmptySubscription;
use juniper::Variables;
use std::sync::Arc;

use graphql::{DbContext, Query};

use crate::chain::RuskNode;
use crate::http::{DataType, WsRequest, WsResponse};

use super::event::WsTarget;

type Schema = juniper::RootNode<
    'static,
    Query,
    EmptyMutation<DbContext>,
    EmptySubscription<DbContext>,
>;

impl RuskNode {
    pub(crate) async fn handle_request(
        &self,
        request: WsRequest,
    ) -> WsResponse {
        match &request.target {
            WsTarget::Host(s) if s == "Chain" && request.topic == "gql" => {
                let ctx = DbContext(self.db());

                let gql_query = match &request.data {
                    DataType::Text(str) => str.clone(),
                    DataType::Binary(data) => {
                        String::from_utf8(data.inner.clone())
                            .unwrap_or_default()
                    }
                    DataType::None => String::default(),
                };
                match juniper::execute(
                    &gql_query,
                    None,
                    &Schema::new(
                        Query,
                        EmptyMutation::new(),
                        EmptySubscription::new(),
                    ),
                    &Variables::new(),
                    &ctx,
                )
                .await
                {
                    Err(e) => WsResponse {
                        data: DataType::None,
                        headers: request.x_headers(),
                        error: format!("{e}").into(),
                    },
                    Ok((res, _errors)) => WsResponse {
                        data: format!("{res}").into(),
                        headers: request.x_headers(),
                        error: None,
                    },
                }
            }
            _ => WsResponse {
                data: DataType::None,
                headers: request.x_headers(),
                error: Some("Unsupported".into()),
            },
        }
    }
}
