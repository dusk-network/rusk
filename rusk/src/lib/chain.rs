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

use crate::chain::graphql::Query;
use crate::http::{ResponseData, WsRequest, WsResponse};
use crate::Rusk;
use graphql::DbContext;

type Schema = juniper::RootNode<
    'static,
    Query,
    EmptyMutation<DbContext>,
    EmptySubscription<DbContext>,
>;

#[derive(Clone)]
pub struct RuskNode(pub node::Node<Kadcast<255>, Backend, Rusk>);
impl RuskNode {
    pub fn db(&self) -> Arc<tokio::sync::RwLock<Backend>> {
        self.0.database() as Arc<tokio::sync::RwLock<Backend>>
    }

    pub(crate) async fn handle_request(
        &self,
        request: WsRequest,
    ) -> WsResponse {
        match request.target_type {
            0x02 if request.target == "chain" => {
                let ctx = DbContext(self.db());

                match juniper::execute(
                    &request.data,
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
                        data: ResponseData::None,
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
                data: ResponseData::None,
                headers: request.x_headers(),
                error: Some("Unsupported".into()),
            },
        }
    }
}
