// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;
use node::database::rocksdb::MD_HASH_KEY;
use node::database::{into_array, Metadata};

pub async fn block_by_height(
    ctx: &Context<'_>,
    height: f64,
) -> OptResult<Block> {
    let db = ctx.data::<DBContext>()?;
    let block_hash = db.read().await.view(|t| {
        if height >= 0f64 {
            t.fetch_block_hash_by_height(height as u64)
        } else {
            Ok(t.op_read(MD_HASH_KEY)?.map(|hash| into_array(&hash[..])))
        }
    })?;

    if let Some(hash) = block_hash {
        return block_by_hash(ctx, hex::encode(hash)).await;
    };

    Ok(None)
}

pub async fn last_block(ctx: &Context<'_>) -> FieldResult<Block> {
    let db = ctx.data::<DBContext>()?;
    let block = db.read().await.view(|t| {
        let hash = t.op_read(MD_HASH_KEY)?;
        match hash {
            None => Ok(None),
            Some(hash) => t.fetch_block_header(&hash),
        }
    })?;

    block
        .map(|(header, txs_id)| Block::new(header, txs_id))
        .ok_or_else(|| FieldError::new("Cannot find last block"))
}

pub async fn block_by_hash(
    ctx: &Context<'_>,
    hash: String,
) -> OptResult<Block> {
    let db = ctx.data::<DBContext>()?;
    let hash = hex::decode(hash)?;
    let block = db.read().await.view(|t| t.fetch_block_header(&hash))?;
    Ok(block.map(|(header, txs_id)| Block::new(header, txs_id)))
}

pub async fn last_blocks(
    ctx: &Context<'_>,
    count: u64,
) -> FieldResult<Vec<Block>> {
    if (count < 1) {
        return Err(FieldError::new("count must be positive"));
    }
    let db = ctx.data::<DBContext>()?;
    let last_block = last_block(ctx).await?;
    let mut hash_to_search = last_block.header().prev_block_hash;
    let blocks = db.read().await.view(|t| {
        let mut blocks = vec![last_block];
        let mut count = count - 1;
        while (count > 0) {
            match t.fetch_block_header(&hash_to_search)? {
                None => break,
                Some((header, txs_id)) => {
                    hash_to_search = header.prev_block_hash;
                    blocks.push(Block::new(header, txs_id));
                    count -= 1;
                }
            }
        }
        Ok::<_, anyhow::Error>(blocks)
    })?;
    Ok(blocks)
}

pub async fn blocks_range(
    ctx: &Context<'_>,
    from: u64,
    to: u64,
) -> FieldResult<Vec<Block>> {
    let db = ctx.data::<DBContext>()?;
    let mut blocks = db.read().await.view(|t| {
        let mut blocks = vec![];
        let mut hash_to_search = None;
        for height in (from..=to).rev() {
            if hash_to_search.is_none() {
                hash_to_search = t.fetch_block_hash_by_height(height)?;
            }
            if let Some(hash) = hash_to_search {
                let (header, txs_id) =
                    t.fetch_block_header(&hash)?.expect("Block to be found");
                hash_to_search = header.prev_block_hash.into();
                blocks.push(Block::new(header, txs_id))
            }
        }
        Ok::<_, anyhow::Error>(blocks)
    })?;
    blocks.reverse();
    Ok(blocks)
}
