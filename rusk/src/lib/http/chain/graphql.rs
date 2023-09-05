// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod block;
mod data;
mod tx;

use block::*;
use data::*;
use tx::*;

use async_graphql::{Context, FieldError, FieldResult, Object};
use node::database::rocksdb::Backend;
use node::database::{Ledger, Register, DB};

use std::sync::Arc;
use tokio::sync::RwLock;

pub type DBContext = Arc<RwLock<Backend>>;
pub type OptResult<T> = FieldResult<Option<T>>;

pub struct Query;

#[Object]
impl Query {
    async fn block(
        &self,
        ctx: &Context<'_>,
        height: Option<f64>,
        hash: Option<String>,
    ) -> OptResult<data::Block> {
        let block = match (height, hash) {
            (Some(height), None) => block_by_height(ctx, height).await,
            (None, Some(hash)) => block_by_hash(ctx, hash).await,
            _ => Err(FieldError::new("Specify heigth or hash")),
        };
        Ok(block?)
    }

    async fn tx(
        &self,
        ctx: &Context<'_>,
        hash: String,
    ) -> OptResult<SpentTransaction> {
        tx_by_hash(ctx, hash).await
    }

    async fn transactions(
        &self,
        ctx: &Context<'_>,
        last: u64,
    ) -> FieldResult<Vec<SpentTransaction>> {
        last_transactions(ctx, last as usize).await
    }

    async fn block_txs(
        &self,
        ctx: &Context<'_>,
        last: Option<u64>,
        range: Option<[u64; 2]>,
        contract: Option<String>,
    ) -> FieldResult<Vec<SpentTransaction>> {
        let blocks = self.blocks(ctx, last, range).await?;

        let contract = match contract {
            Some(contract) => Some(hex::decode(contract)?),
            _ => None,
        };

        let mut txs = vec![];
        for b in blocks.iter() {
            let mut block_txs = b.transactions(ctx).await?;
            match contract.as_ref() {
                None => txs.append(&mut block_txs),
                Some(contract) => {
                    let mut txs_to_add = block_txs
                        .into_iter()
                        .filter(|t| {
                            let tx_contract =
                                t.0.inner
                                    .inner
                                    .call
                                    .as_ref()
                                    .map(|(c, ..)| *c)
                                    .unwrap_or(
                                        rusk_abi::TRANSFER_CONTRACT.to_bytes(),
                                    );

                            tx_contract == contract[..]
                        })
                        .collect();
                    txs.append(&mut txs_to_add);
                }
            }
        }

        Ok(txs)
    }

    async fn blocks(
        &self,
        ctx: &Context<'_>,
        last: Option<u64>,
        range: Option<[u64; 2]>,
    ) -> FieldResult<Vec<Block>> {
        match (last, range) {
            (Some(count), None) => last_blocks(ctx, count).await,
            (None, Some([from, to])) => blocks_range(ctx, from, to).await,
            _ => Err(FieldError::new("")),
        }
    }

    async fn mempool_txs(
        &self,
        ctx: &Context<'_>,
    ) -> FieldResult<Vec<Transaction>> {
        mempool(ctx).await
    }

    async fn mempool_tx(
        &self,
        ctx: &Context<'_>,
        hash: String,
    ) -> OptResult<Transaction> {
        mempool_by_hash(ctx, hash).await
    }
}
