// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;

pub async fn block_by_height(ctx: &Ctx, height: f64) -> OptResult<Block> {
    let block = ctx.read().await.view(|t| match height > 0f64 {
        true => t.fetch_block_by_height(height as u64),
        false => t.get_register().and_then(|reg| match reg {
            Some(Register { mrb_hash, .. }) => t.fetch_block(&mrb_hash),
            None => Ok(None),
        }),
    })?;
    Ok(block)
}

pub async fn last_block(ctx: &Ctx) -> FieldResult<Block> {
    let block = ctx.read().await.view(|t| {
        t.get_register().and_then(|reg| match reg {
            Some(Register { mrb_hash, .. }) => t.fetch_block(&mrb_hash),
            None => Ok(None),
        })
    })?;
    block.ok_or_else(|| FieldError::new("Cannot find last block"))
}

pub async fn block_by_hash(ctx: &Ctx, hash: String) -> OptResult<Block> {
    let hash = hex::decode(hash)?;
    let block = ctx.read().await.view(|t| t.fetch_block(&hash))?;
    Ok(block)
}

pub async fn last_blocks(ctx: &Ctx, count: i32) -> FieldResult<Vec<Block>> {
    let last_block = last_block(ctx).await?;
    let mut hash_to_search = last_block.header().prev_block_hash;
    let blocks = ctx.read().await.view(|t| {
        let mut blocks = vec![last_block];
        let mut count = count - 1;
        while (count > 0) {
            match t.fetch_block(&hash_to_search)? {
                None => break,
                Some(b) => {
                    hash_to_search = b.header().prev_block_hash;
                    blocks.push(b);
                    count -= 1;
                }
            }
        }
        Ok::<_, anyhow::Error>(blocks)
    })?;
    Ok(blocks)
}

pub async fn blocks_range(
    ctx: &Ctx,
    from: i32,
    to: i32,
) -> FieldResult<Vec<Block>> {
    unimplemented!()
}
