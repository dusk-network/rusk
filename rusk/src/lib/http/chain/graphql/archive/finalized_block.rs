// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Module for GraphQL that only pertains to finalized blocks.

use crate::http::chain::graphql::{DBContext, OptResult};
use async_graphql::{Context, FieldError, FieldResult, Object};

/// Check if a block height matches a block hash for a finalized block.
pub async fn check_finalized_block(
    ctx: &Context<'_>,
    block_height: i64,
    hex_block_hash: String,
) -> FieldResult<bool> {
    let (_, archive) = ctx.data::<DBContext>()?;

    archive
        .match_finalized_block_height_hash(block_height, &hex_block_hash)
        .await
        .map_err(|e| FieldError::new(format!("Cannot check block: {}", e)))
}

/// Get the last finalized block.
pub async fn last_finalized_block(
    ctx: &Context<'_>,
) -> FieldResult<(u64, String)> {
    let (_, archive) = ctx.data::<DBContext>()?;

    archive.fetch_last_finalized_block().await.map_err(|e| {
        FieldError::new(format!("Cannot get last finalized block: {}", e))
    })
}
