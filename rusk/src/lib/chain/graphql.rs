// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node::database::rocksdb::Backend;
use node::database::{Ledger, DB};
use node_data::ledger::Block;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct DbContext(pub Arc<RwLock<Backend>>);

// Mark our struct for juniper.
impl juniper::Context for DbContext {}

pub struct Query;

#[juniper::graphql_object(
    Context = DbContext,
)]
impl Query {
    async fn block<P>(context: &DbContext, height: f64) -> Option<Block> {
        context
            .0
            .read()
            .await
            .view(|t| match height > 0f64 {
                true => t.fetch_block_by_height(height as u64),
                false => {
                    let reg = t.get_register().ok().flatten();
                    let mrb = reg.and_then(|reg| {
                        t.fetch_block(&reg.mrb_hash).ok().flatten()
                    });
                    Ok(mrb)
                }
            })
            .ok()
            .flatten() // TODO: FIXME
    }
}
