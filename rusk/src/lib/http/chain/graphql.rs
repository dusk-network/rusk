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
use node_data::ledger::{Block, SpentTransaction, Transaction};

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

    async fn block_txs(
        &self,
        ctx: &Context<'_>,
        last: Option<i32>,
        range: Option<[i32; 2]>,
        contract: Option<String>,
    ) -> FieldResult<Vec<Transaction>> {
        let blocks = self.blocks(ctx, last, range).await?;
        let txs = blocks.into_iter().flat_map(|b| b.txs().clone());

        let txs =
            match contract {
                None => txs.collect(),
                Some(contract) => {
                    let contract = hex::decode(contract)?;
                    txs.filter(|t| {
                        let tx_contract =
                            t.inner.call.as_ref().map(|(c, ..)| *c).unwrap_or(
                                rusk_abi::TRANSFER_CONTRACT.to_bytes(),
                            );

                        tx_contract == contract[..]
                    })
                    .collect()
                }
            };

        Ok(txs)
    }

    async fn blocks(
        &self,
        ctx: &Context<'_>,
        last: Option<i32>,
        range: Option<[i32; 2]>,
    ) -> FieldResult<Vec<Block>> {
        let ctx = ctx.data::<Ctx>()?;

        match (last, range) {
            (Some(count), None) => last_blocks(ctx, count).await,
            (None, Some([from, to])) => blocks_range(ctx, from, to).await,
            _ => Err(FieldError::new("")),
        }
    }
}
