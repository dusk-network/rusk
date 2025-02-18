// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Module for GraphQL that relates to stored events in the archive.

use super::data::ContractEvents;
use crate::http::chain::graphql::{DBContext, OptResult};
use async_graphql::{Context, FieldError, FieldResult, Object};
use dusk_core::abi::{ContractId, CONTRACT_ID_BYTES};

pub async fn events_by_height(
    ctx: &Context<'_>,
    height: i64,
) -> OptResult<ContractEvents> {
    let (_, archive, _) = ctx.data::<DBContext>()?;
    let mut events;

    if height < 0 {
        events = archive.fetch_json_last_events().await.map_err(|e| {
            FieldError::new(format!("Cannot fetch events: {}", e))
        })?;
    } else {
        events =
            archive
                .fetch_json_events_by_height(height)
                .await
                .map_err(|e| {
                    FieldError::new(format!("Cannot fetch events: {}", e))
                })?;
    }

    Ok(Some(ContractEvents(serde_json::from_str(&events)?)))
}

pub async fn events_by_hash(
    ctx: &Context<'_>,
    hash: String,
) -> OptResult<ContractEvents> {
    let (_, archive, _) = ctx.data::<DBContext>()?;
    let events = archive
        .fetch_json_events_by_hash(&hash)
        .await
        .map_err(|e| FieldError::new(format!("Cannot fetch events: {}", e)))?;

    Ok(Some(ContractEvents(serde_json::from_str(&events)?)))
}

pub async fn finalized_events_by_contractid(
    ctx: &Context<'_>,
    hex_contract_id: String,
) -> OptResult<ContractEvents> {
    let (_, archive, _) = ctx.data::<DBContext>()?;

    // shallow check if contract id is valid
    if hex_contract_id.len() != CONTRACT_ID_BYTES * 2 {
        return Err(FieldError::new("Invalid contract_id"));
    }

    let events = archive
        .fetch_finalized_events_from_contract(&hex_contract_id)
        .await
        .map_err(|e| FieldError::new(format!("Cannot fetch events: {}", e)))?;

    Ok(Some(ContractEvents(serde_json::to_value(events)?)))
}
