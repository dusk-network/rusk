// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;
use node::database::rocksdb::MD_HASH_KEY;
use node::database::{Mempool, Metadata};

pub async fn tx_by_hash(
    ctx: &Context<'_>,
    hash: String,
) -> OptResult<SpentTransaction> {
    let (db, _) = ctx.data::<DBContext>()?;
    let hash = hex::decode(hash)?;
    let tx = db.read().await.view(|t| t.ledger_tx(&hash))?;
    Ok(tx.map(SpentTransaction))
}

pub async fn last_transactions(
    ctx: &Context<'_>,
    count: usize,
) -> FieldResult<Vec<SpentTransaction>> {
    if count < 1 {
        return Err(FieldError::new("count must be positive"));
    }

    let (db, _) = ctx.data::<DBContext>()?;
    let transactions = db.read().await.view(|t| {
        let mut txs = vec![];
        let mut current_block =
            t.op_read(MD_HASH_KEY).and_then(|res| match res {
                Some(hash) => t.light_block(&hash),
                None => Ok(None),
            })?;

        while let Some(h) = current_block {
            for txs_id in h.transactions_ids {
                let tx = t.ledger_tx(&txs_id)?.ok_or_else(|| {
                    FieldError::new("Cannot find transaction")
                })?;

                txs.push(SpentTransaction(tx));
                if txs.len() >= count {
                    return Ok::<_, async_graphql::Error>(txs);
                }
            }
            current_block = t.light_block(&h.header.prev_block_hash)?;
        }

        Ok::<_, async_graphql::Error>(txs)
    })?;
    Ok(transactions)
}

pub async fn mempool<'a>(
    ctx: &Context<'_>,
) -> FieldResult<Vec<Transaction<'a>>> {
    let (db, _) = ctx.data::<DBContext>()?;
    db.read().await.view(|db| {
        let txs = db.mempool_txs_sorted_by_fee().map(|t| t.into()).collect();
        Ok(txs)
    })
}

pub async fn mempool_by_hash<'a>(
    ctx: &Context<'_>,
    hash: String,
) -> OptResult<Transaction<'a>> {
    let (db, _) = ctx.data::<DBContext>()?;
    let hash = &hex::decode(hash)?[..];
    let hash = hash.try_into()?;
    let tx = db.read().await.view(|db| db.mempool_tx(hash))?;
    Ok(tx.map(|t| t.into()))
}
