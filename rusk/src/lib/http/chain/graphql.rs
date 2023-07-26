// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node::database::rocksdb::Backend;
use node::database::{Ledger, Register, DB};
use node_data::ledger::Block;

use juniper::{FieldError, FieldResult};
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
    async fn block<P>(
        context: &DbContext,
        height: f64,
    ) -> FieldResult<Option<Block>> {
        let block = context.0.read().await.view(|t| match height > 0f64 {
            true => t.fetch_block_by_height(height as u64),
            false => t.get_register().and_then(|reg| match reg {
                Some(Register { mrb_hash, .. }) => t.fetch_block(&mrb_hash),
                None => Ok(None),
            }),
        })?;
        Ok(block)
    }
}
