// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use async_graphql::{Context, FieldError, FieldResult, Object};

use super::data::BlockEvents;
use crate::http::chain::graphql::{DBContext, OptResult};

pub async fn block_events_by_height(
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

pub async fn block_events_by_hash(
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
