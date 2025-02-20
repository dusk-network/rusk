// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Module for GraphQL that is used for moonlight related data in the archive.

use dusk_bytes::Serializable;
use dusk_core::signatures::bls::PublicKey as AccountPublicKey;
use dusk_core::transfer::{
    ConvertEvent, DepositEvent, MoonlightTransactionEvent, WithdrawEvent,
    CONVERT_TOPIC, MINT_TOPIC, MOONLIGHT_TOPIC, TRANSFER_CONTRACT,
    WITHDRAW_TOPIC,
};
use node::archive::{MoonlightGroup, Order};
use node_data::events::contract::ContractEvent;

use async_graphql::{Context, FieldError};

use super::data::translator::*;
use super::data::{MoonlightTransfers, NewAccountPublicKey};
use crate::http::chain::graphql::{DBContext, OptResult};

pub async fn full_moonlight_history(
    ctx: &Context<'_>,
    address: String,
    ordering: Option<String>,
) -> OptResult<MoonlightTransfers> {
    let (_, archive) = ctx.data::<DBContext>()?;
    let v = bs58::decode(address).into_vec()?;

    let pk_bytes: [u8; 96] = v
        .try_into()
        .map_err(|_| FieldError::new("Invalid public key length"))?;

    let pk = AccountPublicKey::from_bytes(&pk_bytes)
        .map_err(|_| anyhow::anyhow!("Failed to serialize given public key"))?;

    let ord = match ordering.unwrap_or_default().as_str() {
        "asc" => Some(Order::Ascending),
        "desc" => Some(Order::Descending),
        _ => None,
    };

    if let Some(moonlight_events) = archive.full_moonlight_history(pk, ord)? {
        Ok(Some(MoonlightTransfers(moonlight_events)))
    } else {
        Ok(None)
    }
}

pub async fn fetch_moonlight_history(
    ctx: &Context<'_>,
    sender: Option<String>,
    receiver: Option<String>,
    from_block: Option<u64>,
    to_block: Option<u64>,
    max_count: Option<usize>,
    page_count: Option<usize>,
) -> OptResult<MoonlightTransfers> {
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
        Ok(Some(MoonlightTransfers(moonlight_events)))
    } else {
        Ok(None)
    }
}

pub async fn moonlight_tx_by_memo(
    ctx: &Context<'_>,
    memo: Vec<u8>,
) -> OptResult<MoonlightTransfers> {
    let (_, archive) = ctx.data::<DBContext>()?;

    let moonlight_events = archive.moonlight_txs_by_memo(memo)?;

    if let Some(moonlight_events) = moonlight_events {
        Ok(Some(MoonlightTransfers(moonlight_events)))
    } else {
        Ok(None)
    }
}
