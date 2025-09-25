// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Module for GraphQL that relates to stored events in the archive.

use super::data::ContractEvents;
use crate::http::chain::graphql::{DBContext, OptResult};
use async_graphql::{Context, FieldError};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use dusk_core::abi::CONTRACT_ID_BYTES;

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;

pub async fn events_by_height(
    ctx: &Context<'_>,
    height: i64,
) -> OptResult<ContractEvents> {
    let (_, archive) = ctx.data::<DBContext>()?;

    let events = if height < 0 {
        archive.fetch_json_last_events().await
    } else {
        archive.fetch_json_events_by_height(height).await
    }
    .map_err(|e| FieldError::new(format!("Cannot fetch events: {e}")))?;

    Ok(Some(ContractEvents(serde_json::from_str(&events)?)))
}

pub async fn events_by_hash(
    ctx: &Context<'_>,
    hash: String,
) -> OptResult<ContractEvents> {
    let (_, archive) = ctx.data::<DBContext>()?;
    let events = archive
        .fetch_json_events_by_hash(&hash)
        .await
        .map_err(|e| FieldError::new(format!("Cannot fetch events: {}", e)))?;

    Ok(Some(ContractEvents(serde_json::from_str(&events)?)))
}

/// Returns a paginated list of finalized events for a given contract.
///
/// * `hex_contract_id` – hex contract ID.
/// * `limit` – Max rows to return (defaults to `DEFAULT_LIMIT`, clamped to
///   `MAX_LIMIT`).
/// * `cursor` – Opaque base64 cursor, fetches rows with `id` > cursor. `None`
///   starts from the beginning.
///
/// Results are ordered by ascending `id` and wrapped in `ContractEvents`
/// with `events`, `endCursor`, and `hasNextPage` fields.
pub async fn finalized_events_by_contract(
    ctx: &Context<'_>,
    hex_contract_id: String,
    limit: Option<i64>,
    cursor: Option<String>,
) -> OptResult<ContractEvents> {
    let (_, archive) = ctx.data::<DBContext>()?;

    // check if contract ID is valid
    if hex_contract_id.len() != CONTRACT_ID_BYTES * 2 {
        return Err(FieldError::new("Invalid contract_id"));
    }

    // clamp page size
    let clamped_limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

    // decode opaque cursor -> numeric ID
    let cursor_id = match cursor {
        None => None,
        Some(s) => decode_cursor_id(&s)
            .ok_or_else(|| FieldError::new("Invalid cursor"))?
            .into(),
    };

    // fetch one page
    let (events, next_id, has_next) = archive
        .fetch_finalized_events_from_contract(
            &hex_contract_id,
            clamped_limit,
            cursor_id,
        )
        .await
        .map_err(|e| FieldError::new(format!("Cannot fetch events: {e}")))?;

    let start_cursor = events.first().map(|e| encode_cursor_id(e.id));
    let end_cursor = next_id.map(encode_cursor_id);

    let value = serde_json::json!({
        "events": events,
        "endCursor": end_cursor,
        "startCursor": start_cursor,
        "hasNextPage": has_next
    });

    Ok(Some(ContractEvents(value)))
}

/// Encode a numeric ID into an opaque cursor.
fn encode_cursor_id(id: i64) -> String {
    B64.encode(format!("v1:{}", id))
}

/// Decode an opaque cursor back to the numeric ID.
/// Returns None if the cursor is invalid or uses an unknown version.
fn decode_cursor_id(s: &str) -> Option<i64> {
    let bytes = B64.decode(s).ok()?;
    let text = String::from_utf8(bytes).ok()?;
    let (v, rest) = text.split_once(':')?;
    if v != "v1" {
        return None;
    }
    rest.parse::<i64>().ok()
}
