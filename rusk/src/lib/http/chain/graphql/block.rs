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
    let (db, _) = ctx.data::<DBContext>()?;
    let block_hash = db.read().await.view(|t| {
        if height >= 0f64 {
            t.block_hash_by_height(height as u64)
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
    let (db, _) = ctx.data::<DBContext>()?;
    let block = db.read().await.view(|t| {
        let hash = t.op_read(MD_HASH_KEY)?;
        match hash {
            None => Ok(None),
            Some(hash) => t.light_block(&hash),
        }
    })?;

    block
        .map(Block::from)
        .ok_or_else(|| FieldError::new("Cannot find last block"))
}

pub async fn block_by_hash(
    ctx: &Context<'_>,
    hash: String,
) -> OptResult<Block> {
    let (db, _) = ctx.data::<DBContext>()?;
    let hash = hex::decode(hash)?;
    let header = db.read().await.view(|t| t.light_block(&hash))?;
    Ok(header.map(Block::from))
}

pub async fn last_blocks(
    ctx: &Context<'_>,
    count: u64,
) -> FieldResult<Vec<Block>> {
    if count < 1 {
        return Err(FieldError::new("count must be positive"));
    }
    let (db, _) = ctx.data::<DBContext>()?;
    let last_block = last_block(ctx).await?;
    let mut hash_to_search = last_block.header().prev_block_hash;
    let blocks = db.read().await.view(|t| {
        let mut blocks = vec![last_block];
        let mut count = count - 1;
        while count > 0 {
            match t.light_block(&hash_to_search)? {
                None => break,
                Some(h) => {
                    hash_to_search = h.header.prev_block_hash;
                    blocks.push(Block::from(h));
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
    let (db, _) = ctx.data::<DBContext>()?;
    let mut blocks = db.read().await.view(|t| {
        let mut blocks = vec![];
        let mut hash_to_search = None;
        for height in (from..=to).rev() {
            if hash_to_search.is_none() {
                hash_to_search = t.block_hash_by_height(height)?;
            }
            if let Some(hash) = hash_to_search {
                let h = t.light_block(&hash)?.expect("Block to be found");
                hash_to_search = h.header.prev_block_hash.into();
                blocks.push(Block::from(h))
            }
        }
        Ok::<_, anyhow::Error>(blocks)
    })?;
    blocks.reverse();
    Ok(blocks)
}

/// Check if a block height matches a block hash for a block
/// (finalized **or** unfinalized).
pub(super) async fn check_block(
    ctx: &Context<'_>,
    block_height: u64,
    hex_block_hash: String,
) -> FieldResult<bool> {
    let (db, _) = ctx.data::<DBContext>()?;
    let block_hash = hex::decode(hex_block_hash)?;
    let block = db.read().await.view(|t| {
        t.block_hash_by_height(block_height).map(|hash| {
            if let Some(hash) = hash {
                hash == block_hash[..]
            } else {
                false
            }
        })
    })?;

    Ok(block)
}
