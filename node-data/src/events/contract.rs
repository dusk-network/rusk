// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use anyhow::Result;
use execution_core::ContractId;
use execution_core::Event;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Contract event with optional origin (tx hash).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractTxEvent {
    pub event: ContractEvent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<[u8; 32]>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
#[repr(C)]
pub struct WrappedContractId(pub ContractId);

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
        let mut source_array = [0u8; 32];

        if source_bytes.len() != 32 {
            return Err(serde::de::Error::custom(format!(
                "Invalid length: expected 32 bytes, got {}",
                source_bytes.len()
            )));
        }

        source_array.copy_from_slice(&source_bytes);
        Ok(WrappedContractId(ContractId::from_bytes(source_array)))
    }
}

/// Wrapper around a contract event that is to be archived or sent to a
/// websocket client.
#[serde_with::serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractEvent {
    pub target: WrappedContractId,
    pub topic: String,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub data: Vec<u8>,
}

impl From<execution_core::Event> for ContractEvent {
    fn from(event: execution_core::Event) -> Self {
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
    use execution_core::ContractId;

    fn exec_core_event() -> Event {
        Event {
            source: ContractId::from_bytes([0; 32]),
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
