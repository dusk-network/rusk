// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Module for GraphQL that is used for moonlight related data in the archive.

use dusk_bytes::Serializable;
use execution_core::signatures::bls::PublicKey as AccountPublicKey;
use execution_core::transfer::{
    ConvertEvent, DepositEvent, MoonlightTransactionEvent, WithdrawEvent,
    CONVERT_TOPIC, MINT_TOPIC, MOONLIGHT_TOPIC, TRANSFER_CONTRACT,
    WITHDRAW_TOPIC,
};
use node::archive::MoonlightGroup;
use node_data::events::contract::ContractEvent;

use async_graphql::{Context, FieldError};

use super::data::deserialized_archive_data::*;
use super::data::{MoonlightTransactions, NewAccountPublicKey};
use crate::http::chain::graphql::{DBContext, OptResult};

pub async fn full_moonlight_history(
    ctx: &Context<'_>,
    address: String,
) -> OptResult<DeserializedMoonlightGroups> {
    let (_, archive) = ctx.data::<DBContext>()?;
    let v = bs58::decode(address).into_vec()?;

    let pk_bytes: [u8; 96] = v
        .try_into()
        .map_err(|_| FieldError::new("Invalid public key length"))?;

    let pk = AccountPublicKey::from_bytes(&pk_bytes)
        .map_err(|_| anyhow::anyhow!("Failed to serialize given public key"))?;

    let moonlight_groups = archive.full_moonlight_history(pk)?;

    let mut deser_moonlight_groups = Vec::new();

    if let Some(moonlight_groups) = moonlight_groups {
        for moonlight_group in moonlight_groups {
            let deser_events = moonlight_group
                .events()
                .iter()
                .map(|event| event.clone().into())
                .collect::<Vec<DeserializedContractEvent>>();

            let deserialized_moonlight_group = DeserializedMoonlightGroup {
                events: serde_json::to_value(deser_events)?,
                origin: *moonlight_group.origin(),
                block_height: moonlight_group.block_height(),
            };

            deser_moonlight_groups.push(deserialized_moonlight_group);
        }
    }

    if deser_moonlight_groups.is_empty() {
        Ok(None)
    } else {
        Ok(Some(DeserializedMoonlightGroups(deser_moonlight_groups)))
    }
}

pub async fn moonlight_transactions(
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

pub async fn moonlight_tx_by_memo(
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
