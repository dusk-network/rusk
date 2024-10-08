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
    let (db, _) = ctx.data::<DBContext>()?;
    let block = db.read().await.view(|t| {
        let hash = t.op_read(MD_HASH_KEY)?;
        match hash {
            None => Ok(None),
            Some(hash) => t.fetch_light_block(&hash),
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
    let header = db.read().await.view(|t| t.fetch_light_block(&hash))?;
    Ok(header.map(Block::from))
}

pub async fn last_blocks(
    ctx: &Context<'_>,
    count: u64,
) -> FieldResult<Vec<Block>> {
    if (count < 1) {
        return Err(FieldError::new("count must be positive"));
    }
    let (db, _) = ctx.data::<DBContext>()?;
    let last_block = last_block(ctx).await?;
    let mut hash_to_search = last_block.header().prev_block_hash;
    let blocks = db.read().await.view(|t| {
        let mut blocks = vec![last_block];
        let mut count = count - 1;
        while (count > 0) {
            match t.fetch_light_block(&hash_to_search)? {
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
                hash_to_search = t.fetch_block_hash_by_height(height)?;
            }
            if let Some(hash) = hash_to_search {
                let h = t.fetch_light_block(&hash)?.expect("Block to be found");
                hash_to_search = h.header.prev_block_hash.into();
                blocks.push(Block::from(h))
            }
        }
        Ok::<_, anyhow::Error>(blocks)
    })?;
    blocks.reverse();
    Ok(blocks)
}

#[cfg(feature = "archive")]
pub(super) async fn block_events_by_height(
    ctx: &Context<'_>,
    height: i64,
) -> OptResult<BlockEvents> {
    let (_, archive) = ctx.data::<DBContext>()?;
    let mut events;

    if height < 0 {
        events = archive.fetch_json_last_vm_events().await.map_err(|e| {
            FieldError::new(format!("Cannot fetch events: {}", e))
        })?;
    } else {
        events = archive.fetch_json_vm_events(height).await.map_err(|e| {
            FieldError::new(format!("Cannot fetch events: {}", e))
        })?;
    }

    Ok(Some(BlockEvents(serde_json::from_str(&events)?)))
}

#[cfg(feature = "archive")]
pub(super) async fn block_events_by_hash(
    ctx: &Context<'_>,
    hash: String,
) -> OptResult<BlockEvents> {
    let (_, archive) = ctx.data::<DBContext>()?;
    let events = archive
        .fetch_json_vm_events_by_blk_hash(&hash)
        .await
        .map_err(|e| FieldError::new(format!("Cannot fetch events: {}", e)))?;

    Ok(Some(BlockEvents(serde_json::from_str(&events)?)))
}
