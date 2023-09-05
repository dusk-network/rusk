// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node::database::Mempool;

use super::*;

pub async fn tx_by_hash(
    ctx: &Context<'_>,
    hash: String,
) -> OptResult<SpentTransaction> {
    let db = ctx.data::<DBContext>()?;
    let hash = hex::decode(hash)?;
    let tx = db.read().await.view(|t| t.get_ledger_tx_by_hash(&hash))?;
    Ok(tx.map(SpentTransaction))
}

pub async fn last_transactions(
    ctx: &Context<'_>,
    count: usize,
) -> FieldResult<Vec<SpentTransaction>> {
    if (count < 1) {
        return Err(FieldError::new("count must be positive"));
    }

    let db = ctx.data::<DBContext>()?;
    let transactions = db.read().await.view(|t| {
        let mut txs = vec![];
        let mut current_block = t.get_register().and_then(|reg| match reg {
            Some(Register { mrb_hash, .. }) => t.fetch_block_header(&mrb_hash),
            None => Ok(None),
        })?;

        while let Some((header, block_txs)) = current_block {
            for txs_id in block_txs {
                let tx =
                    t.get_ledger_tx_by_hash(&txs_id)?.ok_or_else(|| {
                        FieldError::new("Cannot find transaction")
                    })?;

                txs.push(SpentTransaction(tx));
                if txs.len() >= count {
                    break;
                }
            }
            current_block = t.fetch_block_header(&header.prev_block_hash)?;
        }

        Ok::<_, async_graphql::Error>(txs)
    })?;
    Ok(transactions)
}

pub async fn mempool<'a>(
    ctx: &Context<'_>,
) -> FieldResult<Vec<Transaction<'a>>> {
    let db = ctx.data::<DBContext>()?;
    let transactions = db.read().await.view(|t| {
        let txs = t.get_txs_sorted_by_fee()?.map(|t| t.into()).collect();
        Ok::<_, async_graphql::Error>(txs)
    })?;
    Ok(transactions)
}

pub async fn mempool_by_hash<'a>(
    ctx: &Context<'_>,
    hash: String,
) -> OptResult<Transaction<'a>> {
    let db = ctx.data::<DBContext>()?;
    let hash = &hex::decode(hash)?[..];
    let hash = hash.try_into()?;
    let tx = db.read().await.view(|t| t.get_tx(hash))?;
    Ok(tx.map(|t| t.into()))
}
