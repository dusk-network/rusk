// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! This module defines the contract event type and related types.

use anyhow::Result;
use dusk_core::abi::{ContractId, Event, CONTRACT_ID_BYTES};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub const ORIGIN_HASH_BYTES: usize = 32;
/// Origin hash of a contract event. This is in most cases the transaction hash.
/// In the case of a reward or slash event, it is the block hash.
pub type OriginHash = [u8; ORIGIN_HASH_BYTES];

/// Contract event with origin `OriginHash`.
#[serde_with::serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ContractTxEvent {
    pub event: ContractEvent,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub origin: OriginHash,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
#[repr(C)]
pub struct WrappedContractId(pub ContractId);

impl TryFrom<String> for WrappedContractId {
    type Error = anyhow::Error;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        let source_bytes = hex::decode(value)?;
        let mut source_array = [0u8; CONTRACT_ID_BYTES];

        if source_bytes.len() != CONTRACT_ID_BYTES {
            return Err(anyhow::anyhow!(
                "Invalid length: expected {} bytes, got {}",
                CONTRACT_ID_BYTES,
                source_bytes.len()
            ));
        } else {
            source_array.copy_from_slice(&source_bytes);
        }

        Ok(WrappedContractId(ContractId::from_bytes(source_array)))
    }
}

impl Serialize for WrappedContractId {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let source_hex = hex::encode(self.0.as_bytes());
        s.serialize_str(&source_hex)
    }
}

impl<'de> Deserialize<'de> for WrappedContractId {
    fn deserialize<D>(deserializer: D) -> Result<WrappedContractId, D::Error>
    where
        D: Deserializer<'de>,
    {
        let source_hex: String = Deserialize::deserialize(deserializer)?;
        let source_bytes =
            hex::decode(source_hex).map_err(serde::de::Error::custom)?;
        let mut source_array = [0u8; CONTRACT_ID_BYTES];

        if source_bytes.len() != CONTRACT_ID_BYTES {
            return Err(serde::de::Error::custom(format!(
                "Invalid length: expected {} bytes, got {}",
                CONTRACT_ID_BYTES,
                source_bytes.len()
            )));
        } else {
            source_array.copy_from_slice(&source_bytes);
        }

        Ok(WrappedContractId(ContractId::from_bytes(source_array)))
    }
}

/// Wrapper around a contract event that is to be archived or sent to a
/// websocket client.
#[serde_with::serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ContractEvent {
    pub target: WrappedContractId,
    pub topic: String,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub data: Vec<u8>,
}

impl From<Event> for ContractEvent {
    fn from(event: Event) -> Self {
        Self {
            target: WrappedContractId(event.source),
            topic: event.topic,
            data: event.data,
        }
    }
}

impl From<ContractEvent> for Event {
    fn from(contract_event: ContractEvent) -> Self {
        Event {
            source: contract_event.target.0,
            topic: contract_event.topic,
            data: contract_event.data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exec_core_event() -> Event {
        Event {
            source: ContractId::from_bytes([0; CONTRACT_ID_BYTES]),
            topic: "contract".to_string(),
            data: vec![1, 2, 3],
        }
    }

    #[test]
    fn test_converting_contract_event() {
        let contract_event: ContractEvent = exec_core_event().into();

        assert_eq!(Event::from(contract_event), exec_core_event());
    }

    #[test]
    fn test_serialize_contract_event() {
        let event: ContractEvent = exec_core_event().into();
        let json_event = serde_json::to_string(&event).unwrap();
        assert_eq!(event, serde_json::from_str(&json_event).unwrap());

        let events: Vec<ContractEvent> = vec![event.clone(), event];
        let json_events = serde_json::to_string(&events).unwrap();
        assert_eq!(
            events,
            serde_json::from_str::<Vec<ContractEvent>>(&json_events).unwrap()
        );

        let empty_events: Vec<ContractEvent> = vec![];
        let empty_json_events = serde_json::to_string(&empty_events).unwrap();
        assert_eq!(
            empty_events,
            serde_json::from_str::<Vec<ContractEvent>>(&empty_json_events)
                .unwrap()
        );
    }
}
