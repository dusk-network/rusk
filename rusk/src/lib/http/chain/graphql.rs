// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod block;
mod tx;

use block::*;
use tx::*;

use juniper_codegen::GraphQLObject;
use node::database::rocksdb::Backend;
use node::database::{Ledger, Register, DB};
use node_data::ledger::{Block, SpentTransaction};

use juniper::{graphql_value, FieldError, FieldResult};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Ctx(pub Arc<RwLock<Backend>>);
pub type OptResult<T> = FieldResult<Option<T>>;

// Mark our struct for juniper.
impl juniper::Context for Ctx {}

pub struct Query;

#[juniper::graphql_object(
    Context = Ctx,
)]
impl Query {
    async fn block(
        ctx: &Ctx,
        height: Option<f64>,
        hash: Option<String>,
    ) -> OptResult<Block> {
        match (height, hash) {
            (Some(height), None) => block_by_height(ctx, height).await,
            (None, Some(hash)) => block_by_hash(ctx, hash).await,
            _ => Err(invalid_data("")),
        }
    }
    async fn tx(ctx: &Ctx, hash: String) -> OptResult<SpentTransaction> {
        tx_by_hash(ctx, hash).await
    }

    async fn blocks(
        ctx: &Ctx,
        last: Option<i32>,
        range: Option<Vec<i32>>,
    ) -> FieldResult<Vec<Block>> {
        match (last, range) {
            (Some(count), None) => last_blocks(ctx, count).await,
            (None, Some(range)) => {
                let range: [i32; 2] =
                    range.try_into().map_err(|_| invalid_data("must be 2"))?;
                blocks_range(ctx, range[0], range[1]).await
            }
            _ => Err(invalid_data("")),
        }
    }
}

fn invalid_data(err: &str) -> FieldError {
    FieldError::new("Invalid data", graphql_value!(err))
}
