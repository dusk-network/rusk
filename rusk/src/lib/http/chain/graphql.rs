// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[cfg(feature = "archive")]
mod archive;
mod block;
mod data;
mod tx;

use block::*;
use data::*;
use tx::*;

use async_graphql::{Context, FieldError, FieldResult, Object};
use execution_core::transfer::TRANSFER_CONTRACT;
use execution_core::ContractId;
use node::database::rocksdb::Backend;
use node::database::{Ledger, DB};
use node_data::ledger::Label;
#[cfg(feature = "archive")]
use {
    archive::data::deserialized_archive_data::DeserializedMoonlightGroups,
    archive::data::*,
    archive::events::*,
    archive::finalized_block::*,
    archive::moonlight::*,
    node::archive::{Archive, MoonlightGroup},
};

use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(feature = "archive")]
pub type DBContext = (Arc<RwLock<Backend>>, Archive);
#[cfg(not(feature = "archive"))]
pub type DBContext = (Arc<RwLock<Backend>>, ());

pub type OptResult<T> = FieldResult<Option<T>>;

pub struct Query;

#[Object]
impl Query {
    async fn block(
        &self,
        ctx: &Context<'_>,
        height: Option<f64>,
        hash: Option<String>,
    ) -> OptResult<data::Block> {
        let block = match (height, hash) {
            (Some(height), None) => block_by_height(ctx, height).await,
            (None, Some(hash)) => block_by_hash(ctx, hash).await,
            _ => Err(FieldError::new("Specify heigth or hash")),
        };
        Ok(block?)
    }

    async fn tx(
        &self,
        ctx: &Context<'_>,
        hash: String,
    ) -> OptResult<SpentTransaction> {
        tx_by_hash(ctx, hash).await
    }

    async fn transactions(
        &self,
        ctx: &Context<'_>,
        last: u64,
    ) -> FieldResult<Vec<SpentTransaction>> {
        last_transactions(ctx, last as usize).await
    }

    async fn block_txs(
        &self,
        ctx: &Context<'_>,
        last: Option<u64>,
        range: Option<[u64; 2]>,
        contract: Option<String>,
    ) -> FieldResult<Vec<SpentTransaction>> {
        let blocks = self.blocks(ctx, last, range).await?;

        let contract = match contract {
            Some(contract) => {
                let mut decoded = [0u8; 32];
                decoded.copy_from_slice(&hex::decode(contract)?[..]);
                Some(ContractId::from(decoded))
            }
            _ => None,
        };

        let mut txs = vec![];
        for b in blocks.iter() {
            let mut block_txs = b.transactions(ctx).await?;
            match contract.as_ref() {
                None => txs.append(&mut block_txs),
                Some(contract) => {
                    let mut txs_to_add = block_txs
                        .into_iter()
                        .filter(|t| {
                            let tx_contract =
                                t.0.inner
                                    .inner
                                    .call()
                                    .map(|c| c.contract)
                                    .unwrap_or(TRANSFER_CONTRACT);

                            tx_contract == *contract
                        })
                        .collect();
                    txs.append(&mut txs_to_add);
                }
            }
        }

        Ok(txs)
    }

    async fn blocks(
        &self,
        ctx: &Context<'_>,
        last: Option<u64>,
        range: Option<[u64; 2]>,
    ) -> FieldResult<Vec<Block>> {
        match (last, range) {
            (Some(count), None) => last_blocks(ctx, count).await,
            (None, Some([from, to])) => blocks_range(ctx, from, to).await,
            _ => Err(FieldError::new("")),
        }
    }

    async fn mempool_txs(
        &self,
        ctx: &Context<'_>,
    ) -> FieldResult<Vec<Transaction>> {
        mempool(ctx).await
    }

    async fn mempool_tx(
        &self,
        ctx: &Context<'_>,
        hash: String,
    ) -> OptResult<Transaction> {
        mempool_by_hash(ctx, hash).await
    }

    /// Get a pair of two tuples containing the height and hash of the last
    /// block and the last finalized block.
    async fn last_block_pair(
        &self,
        ctx: &Context<'_>,
    ) -> FieldResult<BlockPair> {
        let tip = last_block(ctx).await?;
        let tip_height = tip.header().height;
        let last_finalized_block =
            ctx.data::<DBContext>()?.0.read().await.view(|v| {
                // Start from the tip to check if it is final
                for height in (0..=tip_height).rev() {
                    if let Ok(Some((hash, Label::Final(_)))) =
                        v.block_label_by_height(height)
                    {
                        let hash = hex::encode(hash);
                        return Ok((height, hash));
                    }
                }

                Err(anyhow::anyhow!(
                    "could not fetch the last final block by height"
                ))
            })?;

        let last_block = (tip_height, hex::encode(tip.header().hash));

        Ok(BlockPair {
            last_block,
            last_finalized_block,
        })
    }

    #[cfg(feature = "archive")]
    async fn full_moonlight_history(
        &self,
        ctx: &Context<'_>,
        address: String,
    ) -> OptResult<DeserializedMoonlightGroups> {
        full_moonlight_history(ctx, address).await
    }

    #[allow(clippy::too_many_arguments)]
    #[cfg(feature = "archive")]
    async fn moonlight_history(
        &self,
        ctx: &Context<'_>,
        sender: Option<String>,
        receiver: Option<String>,
        from_block: Option<u64>,
        to_block: Option<u64>,
        max_count: Option<usize>,
        page_count: Option<usize>,
    ) -> OptResult<MoonlightTransfers> {
        if max_count == Some(0) {
            return Err(FieldError::new("MaxCount must be greater than 0"));
        }

        fetch_moonlight_history(
            ctx, sender, receiver, from_block, to_block, max_count, page_count,
        )
        .await
    }

    #[cfg(feature = "archive")]
    async fn transactions_by_memo(
        &self,
        ctx: &Context<'_>,
        memo: String,
    ) -> OptResult<MoonlightTransfers> {
        // convert String to Vec<u8>
        let memo = memo.into_bytes();
        moonlight_tx_by_memo(ctx, memo).await
    }

    /// Get contract events by height or hash.
    #[cfg(feature = "archive")]
    async fn contract_events(
        &self,
        ctx: &Context<'_>,
        height: Option<i64>,
        hash: Option<String>,
    ) -> OptResult<ContractEvents> {
        match (height, hash) {
            (Some(height), None) => events_by_height(ctx, height).await,
            (None, Some(hash)) => events_by_hash(ctx, hash).await,
            _ => Err(FieldError::new("Specify height or hash")),
        }
    }

    /// Get all finalized contract events from a specific contract id.
    #[cfg(feature = "archive")]
    async fn finalized_events(
        &self,
        ctx: &Context<'_>,
        contract_id: String,
    ) -> OptResult<ContractEvents> {
        finalized_events_by_contractid(ctx, contract_id).await
    }

    /// Check if a given block height matches a given block hash.
    ///
    /// If `only_finalized` is set to `true`, only finalized blocks will be
    /// checked `only_finalized` is set to `false` by default.
    #[cfg(feature = "archive")]
    async fn check_block(
        &self,
        ctx: &Context<'_>,
        height: u64,
        hash: String,
        only_finalized: Option<bool>,
    ) -> FieldResult<bool> {
        if only_finalized.unwrap_or(false) {
            check_finalized_block(ctx, height as i64, hash).await
        } else {
            check_block(ctx, height, hash).await
        }
    }
}
