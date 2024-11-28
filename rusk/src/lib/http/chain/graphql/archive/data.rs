// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use async_graphql::Object;
use dusk_bytes::Serializable;
use execution_core::signatures::bls::PublicKey as AccountPublicKey;
use node::archive::MoonlightGroup;

pub struct MoonlightTransfers(pub Vec<MoonlightGroup>);

pub struct ContractEvents(pub(super) serde_json::Value);

pub(super) struct NewAccountPublicKey(pub AccountPublicKey);

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
impl MoonlightTransfers {
    pub async fn json(&self) -> serde_json::Value {
        serde_json::to_value(&self.0).unwrap_or_default()
    }
}

#[Object]
impl ContractEvents {
    pub async fn json(&self) -> serde_json::Value {
        self.0.clone()
    }
}

/// Interim solution for sending out deserialized event data
/// TODO: #2773 add serde feature to execution-core
pub mod deserialized_archive_data {
    use super::*;
    use execution_core::stake::STAKE_CONTRACT;
    use execution_core::transfer::withdraw::WithdrawReceiver;
    use execution_core::transfer::{
        ConvertEvent, DepositEvent, MoonlightTransactionEvent, WithdrawEvent,
        CONVERT_TOPIC, DEPOSIT_TOPIC, MINT_TOPIC, MOONLIGHT_TOPIC,
        TRANSFER_CONTRACT, WITHDRAW_TOPIC,
    };
    use node_data::events::contract::{
        ContractEvent, OriginHash, WrappedContractId,
    };
    use serde::ser::SerializeStruct;
    use serde::{Deserialize, Serialize};

    #[serde_with::serde_as]
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct DeserializedMoonlightGroup {
        pub events: serde_json::Value,
        #[serde_as(as = "serde_with::hex::Hex")]
        pub origin: OriginHash,
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
