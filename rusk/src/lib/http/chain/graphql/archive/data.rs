// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use async_graphql::Object;
use dusk_bytes::Serializable;
use dusk_core::signatures::bls::PublicKey as AccountPublicKey;
use node::archive::MoonlightGroup;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
use translator::{IntermediateEvent, IntermediateMoonlightGroup};

/// List of archived transactions where each transaction includes at least one
/// event indicating a Moonlight transfer of funds (Not necessarily a moonlight
/// transaction).
pub struct MoonlightTransfers(pub Vec<MoonlightGroup>);

impl Serialize for MoonlightTransfers {
    /// Serializes TRANSFER_CONTRACT events specifically as JSON, falling back
    /// to hex encoding for other event types.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut moonlight_groups: &Vec<MoonlightGroup> = &self.0;
        let mut serializable_groups: Vec<IntermediateMoonlightGroup> = vec![];

        // yoink the events from the moonlight group
        for group in moonlight_groups {
            // convert the events of that group to intermediate events
            let intermediate_events: Vec<IntermediateEvent> = group
                .events()
                .iter()
                .map(|event| IntermediateEvent::from(event.clone()))
                .collect();

            // push the intermediate events to the serializable group
            serializable_groups.push(IntermediateMoonlightGroup {
                events: serde_json::to_value(intermediate_events)
                    .unwrap_or_default(),
                origin: *group.origin(),
                block_height: group.block_height(),
            });
        }

        serializable_groups.serialize(serializer)
    }
}

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
        serde_json::to_value(self).unwrap_or_default()
    }
}

#[Object]
impl ContractEvents {
    pub async fn json(&self) -> serde_json::Value {
        self.0.clone()
    }
}

/// Interim solution for sending out deserialized event data
/// TODO: data driver should further simplify this
pub mod translator {
    use dusk_core::abi::ContractId;
    use dusk_core::stake::StakeEvent;
    use dusk_core::stake::{Reward, SlashEvent, STAKE_CONTRACT};
    use dusk_core::transfer::withdraw::WithdrawReceiver;
    use dusk_core::transfer::{
        ContractToAccountEvent, ContractToContractEvent, ConvertEvent,
        DepositEvent, MoonlightTransactionEvent, PhoenixTransactionEvent,
        WithdrawEvent, CONTRACT_TO_ACCOUNT_TOPIC, CONTRACT_TO_CONTRACT_TOPIC,
        CONVERT_TOPIC, DEPOSIT_TOPIC, MINT_CONTRACT_TOPIC, MINT_TOPIC,
        MOONLIGHT_TOPIC, PHOENIX_TOPIC, TRANSFER_CONTRACT, WITHDRAW_TOPIC,
    };
    use node_data::events::contract::{ContractEvent, OriginHash};
    use serde::ser::SerializeStruct;
    use serde::{Deserialize, Serialize};

    use super::*;

    #[serde_with::serde_as]
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub(super) struct IntermediateMoonlightGroup {
        pub events: serde_json::Value,
        #[serde_as(as = "serde_with::hex::Hex")]
        pub origin: OriginHash,
        pub block_height: u64,
    }

    /// Intermediate Event struct which can represent the data field in
    /// different formats.
    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub(super) struct IntermediateEvent {
        pub target: ContractId,
        pub topic: String,
        pub data: serde_json::Value,
    }

    fn handle_unknown_genesis(
        event_data: Vec<u8>,
    ) -> Result<serde_json::Value, serde_json::Error> {
        tracing::warn!("Unknown genesis event found while calling translate_transfer_events");

        serde_json::to_value(hex::encode(event_data))
    }

    /// This function expects an event from the transfer contract.
    ///
    /// Otherwise it will return the hex encoded data.
    fn translate_transfer_events(
        transfer_contract_event: ContractEvent,
    ) -> Result<serde_json::Value, serde_json::Error> {
        match transfer_contract_event.topic.as_str() {
            MOONLIGHT_TOPIC => rkyv::from_bytes::<MoonlightTransactionEvent>(
                &transfer_contract_event.data,
            )
            .map(serde_json::to_value)
            .unwrap_or_else(|_| {
                handle_unknown_genesis(transfer_contract_event.data)
            }),
            WITHDRAW_TOPIC | MINT_TOPIC => {
                rkyv::from_bytes::<WithdrawEvent>(&transfer_contract_event.data)
                    .map(serde_json::to_value)
                    .unwrap_or_else(|_| {
                        handle_unknown_genesis(transfer_contract_event.data)
                    })
            }
            CONVERT_TOPIC => {
                rkyv::from_bytes::<ConvertEvent>(&transfer_contract_event.data)
                    .map(serde_json::to_value)
                    .unwrap_or_else(|_| {
                        handle_unknown_genesis(transfer_contract_event.data)
                    })
            }
            DEPOSIT_TOPIC => {
                rkyv::from_bytes::<DepositEvent>(&transfer_contract_event.data)
                    .map(serde_json::to_value)
                    .unwrap_or_else(|_| {
                        handle_unknown_genesis(transfer_contract_event.data)
                    })
            }
            CONTRACT_TO_ACCOUNT_TOPIC => {
                rkyv::from_bytes::<ContractToAccountEvent>(
                    &transfer_contract_event.data,
                )
                .map(serde_json::to_value)
                .unwrap_or_else(|_| {
                    handle_unknown_genesis(transfer_contract_event.data)
                })
            }
            MINT_CONTRACT_TOPIC | CONTRACT_TO_CONTRACT_TOPIC => {
                rkyv::from_bytes::<ContractToContractEvent>(
                    &transfer_contract_event.data,
                )
                .map(serde_json::to_value)
                .unwrap_or_else(|_| {
                    handle_unknown_genesis(transfer_contract_event.data)
                })
            }
            PHOENIX_TOPIC => rkyv::from_bytes::<PhoenixTransactionEvent>(
                &transfer_contract_event.data,
            )
            .map(serde_json::to_value)
            .unwrap_or_else(|_| {
                handle_unknown_genesis(transfer_contract_event.data)
            }),
            _ => handle_unknown_genesis(transfer_contract_event.data),
        }
    }

    /// This function expects an event from the stake contract.
    ///
    /// Otherwise it will return the hex encoded data.
    fn translate_stake_events(
        stake_contract_event: ContractEvent,
    ) -> Result<serde_json::Value, serde_json::Error> {
        match stake_contract_event.topic.as_str() {
            "stake" | "unstake" | "withdraw" => {
                rkyv::from_bytes::<StakeEvent>(&stake_contract_event.data)
                    .map(serde_json::to_value)
                    .unwrap_or_else(|_| {
                        handle_unknown_genesis(stake_contract_event.data)
                    })
            }
            "reward" => {
                rkyv::from_bytes::<Vec<Reward>>(&stake_contract_event.data)
                    .map(serde_json::to_value)
                    .unwrap_or_else(|_| {
                        handle_unknown_genesis(stake_contract_event.data)
                    })
            }
            "slash" | "hard_slash" => {
                rkyv::from_bytes::<Vec<SlashEvent>>(&stake_contract_event.data)
                    .map(serde_json::to_value)
                    .unwrap_or_else(|_| {
                        handle_unknown_genesis(stake_contract_event.data)
                    })
            }
            _ => handle_unknown_genesis(stake_contract_event.data),
        }
    }

    /// TODO: core should be able to provide this translation from bytes to
    /// struct & from bytes to json for events?
    impl From<ContractEvent> for IntermediateEvent {
        fn from(event: ContractEvent) -> Self {
            let target = event.target;
            let topic = event.topic.clone();

            let deserialized_data = match event.target {
                TRANSFER_CONTRACT => translate_transfer_events(event),
                STAKE_CONTRACT => translate_stake_events(event),
                _ => serde_json::to_value(hex::encode(event.data)),
            }
            .unwrap_or_else(|e| serde_json::Value::String(e.to_string()));

            Self {
                target,
                topic,
                data: deserialized_data,
            }
        }
    }
}
