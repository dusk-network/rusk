// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::ops::Deref;

use async_graphql::{FieldError, FieldResult, Object, SimpleObject};
use node::database::{Ledger, LightBlock, DB};
use serde::{Deserialize, Serialize};

/// Pair of (block height, block hash) of the last block and the last finalized
/// block.
#[derive(Serialize, Deserialize)]
pub struct BlockPair {
    pub last_block: (u64, String),
    pub last_finalized_block: (u64, String),
}

#[Object]
impl BlockPair {
    pub async fn json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

pub struct Block {
    header: node_data::ledger::Header,
    txs_id: Vec<[u8; 32]>,
}

impl From<LightBlock> for Block {
    fn from(value: LightBlock) -> Self {
        Self {
            header: value.header,
            txs_id: value.transactions_ids,
        }
    }
}

impl Block {
    pub fn header(&self) -> &node_data::ledger::Header {
        &self.header
    }
}

pub struct Header<'a>(&'a node_data::ledger::Header);
pub struct SpentTransaction(pub node_data::ledger::SpentTransaction);
pub struct Transaction<'a>(TransactionData<'a>);

impl<'a> From<&'a node_data::ledger::Transaction> for Transaction<'a> {
    fn from(value: &'a node_data::ledger::Transaction) -> Self {
        Self(TransactionData::Ref(value))
    }
}

impl From<node_data::ledger::Transaction> for Transaction<'_> {
    fn from(value: node_data::ledger::Transaction) -> Self {
        Self(TransactionData::Owned(value))
    }
}

#[allow(clippy::large_enum_variant)]
enum TransactionData<'a> {
    Owned(node_data::ledger::Transaction),
    Ref(&'a node_data::ledger::Transaction),
}

impl Deref for TransactionData<'_> {
    type Target = node_data::ledger::Transaction;
    fn deref(&self) -> &Self::Target {
        match self {
            TransactionData::Owned(t) => t,
            TransactionData::Ref(t) => t,
        }
    }
}

#[Object]
impl Block {
    #[graphql(name = "header")]
    pub async fn gql_header(&self) -> Header {
        Header(&self.header)
    }

    pub async fn transactions(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> FieldResult<Vec<SpentTransaction>> {
        let db = ctx.data::<super::DBContext>()?.0.read().await;
        let mut ret = vec![];

        db.view(|t| {
            for id in &self.txs_id {
                let tx = t.ledger_tx(id)?.ok_or_else(|| {
                    FieldError::new("Cannot find transaction")
                })?;
                ret.push(SpentTransaction(tx));
            }
            Ok::<(), async_graphql::Error>(())
        })?;

        Ok(ret)
    }

    pub async fn reward(&self) -> u64 {
        crate::node::emission_amount(self.header.height)
    }

    pub async fn fees(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> FieldResult<u64> {
        let fees = self
            .transactions(ctx)
            .await?
            .iter()
            .map(|t| t.0.gas_spent * t.0.inner.gas_price())
            .sum();
        Ok(fees)
    }

    pub async fn gas_spent(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> FieldResult<u64> {
        let gas_spent = self
            .transactions(ctx)
            .await?
            .iter()
            .map(|t| t.0.gas_spent)
            .sum();
        Ok(gas_spent)
    }
}

#[Object]
impl Header<'_> {
    pub async fn version(&self) -> u8 {
        self.0.version
    }

    pub async fn height(&self) -> u64 {
        self.0.height
    }

    pub async fn prev_block_hash(&self) -> String {
        hex::encode(self.0.prev_block_hash)
    }

    pub async fn timestamp(&self) -> u64 {
        self.0.timestamp
    }

    pub async fn hash(&self) -> String {
        hex::encode(self.0.hash)
    }

    pub async fn state_hash(&self) -> String {
        hex::encode(self.0.state_hash)
    }

    pub async fn generator_bls_pubkey(&self) -> String {
        bs58::encode(self.0.generator_bls_pubkey.0).into_string()
    }

    pub async fn tx_root(&self) -> String {
        hex::encode(self.0.txroot)
    }

    pub async fn gas_limit(&self) -> u64 {
        self.0.gas_limit
    }

    pub async fn seed(&self) -> String {
        hex::encode(self.0.seed.inner())
    }

    pub async fn iteration(&self) -> u8 {
        self.0.iteration
    }

    pub async fn json(&self) -> String {
        serde_json::to_string(self.0).unwrap_or_default()
    }
}

#[Object]
impl SpentTransaction {
    pub async fn tx(&self) -> Transaction {
        let inner = &self.0.inner;
        inner.into()
    }

    pub async fn err(&self) -> &Option<String> {
        &self.0.err
    }

    pub async fn gas_spent(&self) -> u64 {
        self.0.gas_spent
    }

    pub async fn block_hash(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> FieldResult<String> {
        let db = ctx.data::<super::DBContext>()?.0.read().await;
        let block_height = self.0.block_height;

        let block_hash = db.view(|t| {
            t.block_hash_by_height(block_height)?.ok_or_else(|| {
                FieldError::new("Cannot find block hash by height")
            })
        })?;

        Ok(hex::encode(block_hash))
    }

    pub async fn block_height(&self) -> u64 {
        self.0.block_height
    }

    pub async fn error(&self) -> Option<String> {
        self.0.err
    }

    pub async fn block_timestamp(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> FieldResult<u64> {
        let db = ctx.data::<super::DBContext>()?.0.read().await;
        let block_height = self.0.block_height;

        let header = db.view(|t| {
            let block_hash =
                t.block_hash_by_height(block_height)?.ok_or_else(|| {
                    FieldError::new("Cannot find block hash by height")
                })?;
            t.block_header(&block_hash)?
                .ok_or_else(|| FieldError::new("Cannot find block header"))
        })?;

        Ok(header.timestamp)
    }

    pub async fn id(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> FieldResult<String> {
        self.tx(ctx).await?.id(ctx).await
    }

    pub async fn raw(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> FieldResult<String> {
        self.tx(ctx).await?.raw(ctx).await
    }
}

#[Object]
impl Transaction<'_> {
    pub async fn raw(&self) -> String {
        hex::encode(self.0.inner.to_var_bytes())
    }

    pub async fn json(&self) -> String {
        let tx: &node_data::ledger::Transaction = &self.0;
        serde_json::to_string(tx).unwrap_or_default()
    }

    pub async fn id(&self) -> String {
        hex::encode(self.0.id())
    }

    pub async fn gas_limit(&self) -> u64 {
        self.0.inner.gas_limit()
    }

    pub async fn gas_price(&self) -> u64 {
        self.0.inner.gas_price()
    }

    pub async fn tx_type(&self) -> String {
        match self.0.inner {
            execution_core::transfer::Transaction::Phoenix(_) => "Phoenix",
            execution_core::transfer::Transaction::Moonlight(_) => "Moonlight",
        }
        .into()
    }

    pub async fn call_data(&self) -> Option<CallData> {
        self.0.inner.call().map(|call| CallData {
            contract_id: hex::encode(call.contract),
            fn_name: call.fn_name.clone(),
            data: hex::encode(&call.fn_args),
        })
    }

    pub async fn is_deploy(&self) -> bool {
        self.0.inner.deploy().is_some()
    }

    pub async fn memo(&self) -> Option<String> {
        self.0.inner.memo().map(hex::encode)
    }
}

#[derive(SimpleObject)]
pub struct CallData {
    contract_id: String,
    fn_name: String,
    data: String,
}
