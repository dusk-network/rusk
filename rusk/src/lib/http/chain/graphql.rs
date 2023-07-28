// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod block;
mod tx;

use block::*;
use tx::*;

use async_graphql::{Context, FieldError, FieldResult, Object};
use node::database::rocksdb::Backend;
use node::database::{Ledger, Register, DB};
use node_data::ledger::{Block, SpentTransaction};

use std::sync::Arc;
use tokio::sync::RwLock;

pub type Ctx = Arc<RwLock<Backend>>;
pub type OptResult<T> = FieldResult<Option<T>>;

pub struct Query;

#[Object]
impl Query {
    async fn block(
        &self,
        ctx: &Context<'_>,
        height: Option<f64>,
        hash: Option<String>,
    ) -> OptResult<Block> {
        let ctx = ctx.data::<Ctx>()?;

        match (height, hash) {
            (Some(height), None) => block_by_height(ctx, height).await,
            (None, Some(hash)) => block_by_hash(ctx, hash).await,
            _ => Err(FieldError::new("Specify heigth or hash")),
        }
    }
    async fn tx(
        &self,
        ctx: &Context<'_>,
        hash: String,
    ) -> OptResult<SpentTransaction> {
        let ctx = ctx.data::<Ctx>()?;

        tx_by_hash(ctx, hash).await
    }

    async fn blocks(
        &self,
        ctx: &Context<'_>,
        last: Option<i32>,
        range: Option<Vec<i32>>,
    ) -> FieldResult<Vec<Block>> {
        let ctx = ctx.data::<Ctx>()?;

        match (last, range) {
            (Some(count), None) => last_blocks(ctx, count).await,
            (None, Some(range)) => {
                let range: [i32; 2] = range.try_into().map_err(|_| {
                    FieldError::new("You have to specify a range")
                })?;
                blocks_range(ctx, range[0], range[1]).await
            }
            _ => Err(FieldError::new("")),
        }
    }
}
