// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node::database::rocksdb::MD_HASH_KEY;
use node::database::{Mempool, Metadata};
#[cfg(feature = "archive")]
use {
    dusk_bytes::Serializable,
    execution_core::signatures::bls::PublicKey as AccountPublicKey,
    node::archive::MoonlightGroup,
};

use super::*;

pub async fn tx_by_hash(
    ctx: &Context<'_>,
    hash: String,
) -> OptResult<SpentTransaction> {
    let (db, _) = ctx.data::<DBContext>()?;
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

    let (db, _) = ctx.data::<DBContext>()?;
    let transactions = db.read().await.view(|t| {
        let mut txs = vec![];
        let mut current_block =
            t.op_read(MD_HASH_KEY).and_then(|res| match res {
                Some(hash) => t.fetch_light_block(&hash),
                None => Ok(None),
            })?;

        while let Some(h) = current_block {
            for txs_id in h.transactions_ids {
                let tx =
                    t.get_ledger_tx_by_hash(&txs_id)?.ok_or_else(|| {
                        FieldError::new("Cannot find transaction")
                    })?;

                txs.push(SpentTransaction(tx));
                if txs.len() >= count {
                    return Ok::<_, async_graphql::Error>(txs);
                }
            }
            current_block = t.fetch_light_block(&h.header.prev_block_hash)?;
        }

        Ok::<_, async_graphql::Error>(txs)
    })?;
    Ok(transactions)
}

pub async fn mempool<'a>(
    ctx: &Context<'_>,
) -> FieldResult<Vec<Transaction<'a>>> {
    let (db, _) = ctx.data::<DBContext>()?;
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
    let (db, _) = ctx.data::<DBContext>()?;
    let hash = &hex::decode(hash)?[..];
    let hash = hash.try_into()?;
    let tx = db.read().await.view(|t| t.get_tx(hash))?;
    Ok(tx.map(|t| t.into()))
}

#[cfg(feature = "archive")]
pub(super) async fn full_moonlight_history(
    ctx: &Context<'_>,
    address: String,
) -> OptResult<MoonlightTransactions> {
    use dusk_bytes::ParseHexStr;

    let (_, archive) = ctx.data::<DBContext>()?;
    let v = bs58::decode(address).into_vec()?;

    let pk_bytes: [u8; 96] = v
        .try_into()
        .map_err(|_| FieldError::new("Invalid public key length"))?;

    let pk = AccountPublicKey::from_bytes(&pk_bytes)
        .map_err(|_| anyhow::anyhow!("Failed to serialize given public key"))?;

    let moonlight_events = archive.full_moonlight_history(pk)?;

    if let Some(moonlight_events) = moonlight_events {
        Ok(Some(MoonlightTransactions(moonlight_events)))
    } else {
        Ok(None)
    }
}

#[cfg(feature = "archive")]
pub(super) async fn moonlight_transactions(
    ctx: &Context<'_>,
    sender: Option<String>,
    receiver: Option<String>,
    from_block: Option<u64>,
    to_block: Option<u64>,
    max_count: Option<usize>,
    page_count: Option<usize>,
) -> OptResult<MoonlightTransactions> {
    let (_, archive) = ctx.data::<DBContext>()?;

    let sender: Option<AccountPublicKey> = sender
        .map(|s| s.try_into())
        .transpose()?
        .map(|s: NewAccountPublicKey| s.0);
    let receiver: Option<AccountPublicKey> = receiver
        .map(|r| r.try_into())
        .transpose()?
        .map(|s: NewAccountPublicKey| s.0);

    if let Some(moonlight_events) = archive.fetch_moonlight_history(
        sender, receiver, from_block, to_block, max_count, page_count,
    )? {
        Ok(Some(MoonlightTransactions(moonlight_events)))
    } else {
        Ok(None)
    }
}

#[cfg(feature = "archive")]
pub(super) async fn moonlight_tx_by_memo(
    ctx: &Context<'_>,
    memo: Vec<u8>,
) -> OptResult<MoonlightTransactions> {
    let (_, archive) = ctx.data::<DBContext>()?;

    let moonlight_events = archive.moonlight_txs_by_memo(memo)?;

    if let Some(moonlight_events) = moonlight_events {
        Ok(Some(MoonlightTransactions(moonlight_events)))
    } else {
        Ok(None)
    }
}
