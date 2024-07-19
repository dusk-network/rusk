// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::ops::Deref;

use async_graphql::{FieldError, FieldResult, Object, SimpleObject};
use node::database::{Ledger, LightBlock, DB};

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
        let db = ctx.data::<super::DBContext>()?.read().await;
        let mut ret = vec![];

        db.view(|t| {
            for id in &self.txs_id {
                let tx = t.get_ledger_tx_by_hash(id)?.ok_or_else(|| {
                    FieldError::new("Cannot find transaction")
                })?;
                ret.push(SpentTransaction(tx));
            }
            Ok::<(), async_graphql::Error>(())
        })?;

        Ok(ret)
    }

    pub async fn reward(&self) -> u64 {
        crate::chain::emission_amount(self.header.height)
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
        let db = ctx.data::<super::DBContext>()?.read().await;
        let block_height = self.0.block_height;

        let block_hash = db.view(|t| {
            t.fetch_block_hash_by_height(block_height)?.ok_or_else(|| {
                FieldError::new("Cannot find block hash by height")
            })
        })?;

        Ok(hex::encode(block_hash))
    }

    pub async fn block_height(&self) -> u64 {
        self.0.block_height
    }

    pub async fn block_timestamp(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> FieldResult<u64> {
        let db = ctx.data::<super::DBContext>()?.read().await;
        let block_height = self.0.block_height;

        let header = db.view(|t| {
            let block_hash =
                t.fetch_block_hash_by_height(block_height)?.ok_or_else(
                    || FieldError::new("Cannot find block hash by height"),
                )?;
            t.fetch_block_header(&block_hash)?
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
        use dusk_bytes::Serializable;
        use serde::Serialize;
        use serde_json::{json, Map, Value};

        let tx = &self.0.inner;

        let mut map = Map::new();
        map.insert(
            "root".into(),
            json!(hex::encode(tx.payload().tx_skeleton.root.to_bytes())),
        );
        let nullifiers: Vec<_> = tx
            .payload()
            .tx_skeleton
            .nullifiers
            .iter()
            .map(|n| hex::encode(n.to_bytes()))
            .collect();
        map.insert("nullifier".into(), json!(nullifiers));
        map.insert(
            "deposit".into(),
            json!(hex::encode(tx.payload().tx_skeleton.deposit.to_bytes())),
        );
        let notes: Vec<_> = tx
            .payload()
            .tx_skeleton
            .outputs
            .iter()
            .map(|n| {
                let mut map = Map::new();
                map.insert("note_type".into(), json!(n.note_type() as u8));
                map.insert(
                    "value_commitment".into(),
                    json!([
                        hex::encode(n.value_commitment().get_u().to_bytes()),
                        hex::encode(n.value_commitment().get_v().to_bytes())
                    ]),
                );
                map.insert(
                    "stealth_address".into(),
                    json!(bs58::encode(n.stealth_address().to_bytes())
                        .into_string()),
                );
                map.insert(
                    "value_enc".into(),
                    json!(n
                        .value_enc()
                        .iter()
                        .map(|c| hex::encode(c.to_bytes()))
                        .collect::<Vec<_>>()),
                );
                map.insert(
                    "sender".into(),
                    json!(hex::encode(n.sender().to_bytes())),
                );
                map
            })
            .collect();
        map.insert("notes".into(), json!(notes));

        let mut fee = Map::new();
        fee.insert("gas_limit".into(), json!(tx.payload().fee.gas_limit));
        fee.insert("gas_price".into(), json!(tx.payload().fee.gas_price));
        fee.insert(
            "stealth_address".into(),
            json!(bs58::encode(tx.payload().fee.stealth_address.to_bytes())
                .into_string()),
        );
        fee.insert(
            "sender".into(),
            json!(hex::encode(tx.payload().fee.sender.to_bytes())),
        );
        map.insert("fee".into(), json!(fee));

        if let Some(c) = &tx.payload().contract_call() {
            let mut call = Map::new();
            call.insert("contract".into(), json!(hex::encode(c.contract)));
            call.insert("fn_name".into(), json!(&c.fn_name));
            call.insert("fn_args".into(), json!(hex::encode(&c.fn_args)));
            map.insert("call".into(), json!(call));
        }

        json!(map).to_string()
    }

    pub async fn id(&self) -> String {
        hex::encode(self.0.id())
    }

    pub async fn gas_limit(&self) -> u64 {
        self.0.inner.payload().fee.gas_limit
    }

    pub async fn gas_price(&self) -> u64 {
        self.0.inner.payload().fee.gas_price
    }

    pub async fn call_data(&self) -> Option<CallData> {
        self.0
            .inner
            .payload()
            .contract_call()
            .as_ref()
            .map(|call| CallData {
                contract: hex::encode(call.contract),
                fn_name: call.fn_name.clone(),
                fn_args: hex::encode(&call.fn_args),
            })
    }
}

#[derive(SimpleObject)]
pub struct CallData {
    contract: String,
    fn_name: String,
    fn_args: String,
}
