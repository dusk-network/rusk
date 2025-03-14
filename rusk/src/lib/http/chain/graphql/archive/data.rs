// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use async_graphql::Object;
use dusk_bytes::Serializable;
use dusk_core::signatures::bls::PublicKey as AccountPublicKey;
use node::archive::MoonlightGroup;
use serde::Serialize;
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
        let moonlight_groups: &Vec<MoonlightGroup> = &self.0;
        let mut serializable_groups: Vec<IntermediateMoonlightGroup> = vec![];

        // yoink the events from the moonlight group
        for group in moonlight_groups {
            // convert the events of that group to intermediate events
            let intermediate_events: Vec<IntermediateEvent> =
                group.events().iter().map(IntermediateEvent::from).collect();

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
                .map_err(|e| format!("Failed to deserialize bytes {e:?}"))?,
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
    use dusk_core::stake::STAKE_CONTRACT;
    use dusk_core::transfer::TRANSFER_CONTRACT;
    use dusk_data_driver::ConvertibleContract;
    use dusk_stake_contract_dd::ContractDriver as StakeContractDriver;
    use dusk_transfer_contract_dd::ContractDriver as TransferContractDriver;
    use node_data::events::contract::{ContractEvent, OriginHash};
    use serde::{Deserialize, Serialize};

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

    /// TODO: core should be able to provide this translation from bytes to
    /// struct & from bytes to json for events?
    impl From<&ContractEvent> for IntermediateEvent {
        fn from(event: &ContractEvent) -> Self {
            let target = event.target;
            let topic = event.topic.clone();
            let data = &event.data;

            let deserialized_data = match event.target {
                TRANSFER_CONTRACT => {
                    TransferContractDriver.decode_event(&topic, data)
                }
                STAKE_CONTRACT => {
                    StakeContractDriver.decode_event(&topic, data)
                }
                _ => serde_json::to_value(hex::encode(data))
                    .map_err(dusk_data_driver::Error::from),
            }
            .unwrap_or_else(|e| {
                tracing::warn!(
                    event = "Cannot decode event",
                    ?target,
                    topic,
                    ?e
                );
                serde_json::to_value(hex::encode(data))
                    .expect("String to be serialized")
            });

            Self {
                target,
                topic,
                data: deserialized_data,
            }
        }
    }
}
