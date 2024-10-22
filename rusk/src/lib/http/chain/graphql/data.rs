// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::ops::Deref;

use async_graphql::{FieldError, FieldResult, Object, SimpleObject};
use node::database::{Ledger, LightBlock, DB};
#[cfg(feature = "archive")]
use {
    dusk_bytes::Serializable,
    execution_core::signatures::bls::PublicKey as AccountPublicKey,
};

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

#[cfg(feature = "archive")]
pub struct MoonlightTransactions(pub Vec<node::archive::MoonlightGroup>);
#[cfg(feature = "archive")]
pub struct BlockEvents(pub(super) serde_json::Value);
#[cfg(feature = "archive")]
pub(super) struct NewAccountPublicKey(pub AccountPublicKey);
#[cfg(feature = "archive")]
impl TryInto<NewAccountPublicKey> for String {
    type Error = String;

    fn try_into(self) -> Result<NewAccountPublicKey, Self::Error> {
        let mut pk_bytes = [0u8; 96];
        bs58::decode(self).into(&mut pk_bytes).map_err(|_| {
            "Failed to decode given public key to bytes".to_string()
        })?;

        Ok(NewAccountPublicKey(
            AccountPublicKey::from_bytes(&pk_bytes)
                .map_err(|e| "Failed to serialize bytes".to_string())?,
        ))
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

#[cfg(feature = "archive")]
#[Object]
impl MoonlightTransactions {
    pub async fn json(&self) -> serde_json::Value {
        serde_json::to_value(&self.0).unwrap_or_default()
    }
}

#[cfg(feature = "archive")]
#[Object]
impl BlockEvents {
    pub async fn json(&self) -> serde_json::Value {
        self.0.clone()
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
        let db = ctx.data::<super::DBContext>()?.0.read().await;
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

/// Interim solution for sending out deserialized event data
#[cfg(feature = "archive")]
pub(super) mod deserialized_archive_data {
    use super::*;
    use execution_core::stake::STAKE_CONTRACT;
    use execution_core::transfer::withdraw::WithdrawReceiver;
    use execution_core::transfer::{
        ConvertEvent, DepositEvent, MoonlightTransactionEvent, WithdrawEvent,
        CONVERT_TOPIC, DEPOSIT_TOPIC, MINT_TOPIC, MOONLIGHT_TOPIC,
        TRANSFER_CONTRACT, WITHDRAW_TOPIC,
    };
    use node_data::events::contract::{
        ContractEvent, TxHash, WrappedContractId,
    };
    use serde::ser::SerializeStruct;
    use serde::{Deserialize, Serialize};

    #[serde_with::serde_as]
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct DeserializedMoonlightGroup {
        pub events: serde_json::Value,
        #[serde_as(as = "serde_with::hex::Hex")]
        pub origin: TxHash,
        pub block_height: u64,
    }
    pub struct DeserializedMoonlightGroups(pub Vec<DeserializedMoonlightGroup>);

    #[Object]
    impl DeserializedMoonlightGroups {
        pub async fn json(&self) -> serde_json::Value {
            serde_json::to_value(&self.0).unwrap_or_default()
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct DeserializedMoonlightTransactionEvent(
        pub MoonlightTransactionEvent,
    );

    impl Serialize for DeserializedMoonlightTransactionEvent {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let moonlight_event = &self.0;

            let mut state =
                serializer.serialize_struct("MoonlightTransactionEvent", 6)?;
            state.serialize_field(
                "sender",
                &bs58::encode(moonlight_event.sender.to_bytes()).into_string(),
            )?;
            state.serialize_field(
                "receiver",
                &moonlight_event
                    .receiver
                    .map(|r| bs58::encode(r.to_bytes()).into_string()),
            )?;
            state.serialize_field("value", &moonlight_event.value)?;
            state
                .serialize_field("memo", &hex::encode(&moonlight_event.memo))?;
            state.serialize_field("gas_spent", &moonlight_event.gas_spent)?;
            state.serialize_field(
                "refund_info",
                &moonlight_event.refund_info.map(|(pk, amt)| {
                    (bs58::encode(pk.to_bytes()).into_string(), amt)
                }),
            )?;

            state.end()
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct DeserializedWithdrawEvent(pub WithdrawEvent);

    impl Serialize for DeserializedWithdrawEvent {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let withdraw_event = &self.0;
            let mut state = serializer.serialize_struct("WithdrawEvent", 3)?;
            state.serialize_field(
                "sender",
                &WrappedContractId(withdraw_event.sender),
            )?;
            state.serialize_field(
                "receiver",
                &match withdraw_event.receiver {
                    WithdrawReceiver::Moonlight(pk) => {
                        bs58::encode(pk.to_bytes()).into_string()
                    }
                    WithdrawReceiver::Phoenix(pk) => {
                        bs58::encode(pk.to_bytes()).into_string()
                    }
                },
            )?;
            state.serialize_field("value", &withdraw_event.value)?;

            state.end()
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct DeserializedConvertEvent(pub ConvertEvent);

    impl Serialize for DeserializedConvertEvent {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let convert_event = &self.0;
            let mut state = serializer.serialize_struct("ConvertEvent", 3)?;
            state.serialize_field(
                "sender",
                &convert_event
                    .sender
                    .map(|pk| bs58::encode(pk.to_bytes()).into_string()),
            )?;
            state.serialize_field(
                "receiver",
                &match convert_event.receiver {
                    WithdrawReceiver::Moonlight(pk) => {
                        bs58::encode(pk.to_bytes()).into_string()
                    }
                    WithdrawReceiver::Phoenix(pk) => {
                        bs58::encode(pk.to_bytes()).into_string()
                    }
                },
            )?;
            state.serialize_field("value", &convert_event.value)?;
            state.end()
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct DeserializedDepositEvent(pub DepositEvent);

    impl Serialize for DeserializedDepositEvent {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let deposit_event = &self.0;
            let mut state = serializer.serialize_struct("DepositEvent", 3)?;
            state.serialize_field(
                "sender",
                &deposit_event
                    .sender
                    .map(|pk| bs58::encode(pk.to_bytes()).into_string()),
            )?;
            state.serialize_field(
                "receiver",
                &WrappedContractId(deposit_event.receiver),
            )?;
            state.serialize_field("value", &deposit_event.value)?;

            state.end()
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct DeserializedContractEvent {
        pub target: WrappedContractId,
        pub topic: String,
        pub data: serde_json::Value,
    }

    impl From<ContractEvent> for DeserializedContractEvent {
        fn from(event: ContractEvent) -> Self {
            let deserialized_data = if event.target.0 == TRANSFER_CONTRACT {
                match event.topic.as_str() {
                    MOONLIGHT_TOPIC => rkyv::from_bytes::<
                        MoonlightTransactionEvent,
                    >(&event.data)
                    .map(|e| {
                        serde_json::to_value(
                            DeserializedMoonlightTransactionEvent(e),
                        )
                    })
                    .unwrap_or_else(|_| serde_json::to_value(event.data)),
                    WITHDRAW_TOPIC | MINT_TOPIC => rkyv::from_bytes::<
                        WithdrawEvent,
                    >(
                        &event.data
                    )
                    .map(|e| serde_json::to_value(DeserializedWithdrawEvent(e)))
                    .unwrap_or_else(|_| serde_json::to_value(event.data)),
                    CONVERT_TOPIC => {
                        rkyv::from_bytes::<ConvertEvent>(&event.data)
                            .map(|e| {
                                serde_json::to_value(DeserializedConvertEvent(
                                    e,
                                ))
                            })
                            .unwrap_or_else(|_| {
                                serde_json::to_value(event.data)
                            })
                    }
                    DEPOSIT_TOPIC => {
                        rkyv::from_bytes::<DepositEvent>(&event.data)
                            .map(|e| {
                                serde_json::to_value(DeserializedDepositEvent(
                                    e,
                                ))
                            })
                            .unwrap_or_else(|_| {
                                serde_json::to_value(event.data)
                            })
                    }
                    _ => serde_json::to_value(hex::encode(event.data)),
                }
            } else {
                serde_json::to_value(hex::encode(event.data))
            }
            .unwrap_or_else(|e| serde_json::Value::String(e.to_string()));

            Self {
                target: event.target,
                topic: event.topic,
                data: deserialized_data,
            }
        }
    }
}
