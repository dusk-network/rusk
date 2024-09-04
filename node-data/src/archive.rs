// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use execution_core::ContractId;
use execution_core::Event;
use execution_core::CONTRACT_ID_BYTES;
use serde::{Deserialize, Serialize};

use crate::ledger::Hash;

/// Wrapper around a contract event that is to be archived.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractEvent {
    pub source: [u8; CONTRACT_ID_BYTES],
    pub topic: String,
    pub data: Vec<u8>,
}

/// Defined data, that the archivist will store.
///
/// This is also the type of the mpsc channel where the archivist listens for
/// data to archive.
///
/// Any data that archive nodes can store must be defined here
pub enum ArchivalData {
    /// List of contract events from one block together with the block height
    /// and block hash.
    ArchivedEvents(u64, Hash, Vec<ContractEvent>),
}

impl ArchivalData {
    /// Returns the block height of the data.
    pub fn block_height(&self) -> u64 {
        match self {
            ArchivalData::ArchivedEvents(height, _, _) => *height,
        }
    }

    /// Returns the block hash of the data.
    pub fn block_hash(&self) -> &Hash {
        match self {
            ArchivalData::ArchivedEvents(_, hash, _) => hash,
        }
    }
}

impl From<Event> for ContractEvent {
    fn from(event: Event) -> Self {
        ContractEvent {
            source: event.source.to_bytes(),
            topic: event.topic,
            data: event.data,
        }
    }
}

impl From<ContractEvent> for Event {
    fn from(contract_event: ContractEvent) -> Self {
        Event {
            source: ContractId::from_bytes(contract_event.source),
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
