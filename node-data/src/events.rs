// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod blocks;
pub mod contract;
mod transactions;

pub use blocks::{BlockEvent, BLOCK_CONFIRMED, BLOCK_FINALIZED};
pub use transactions::TransactionEvent;

/// Represents an event in the system, including its source (`component`),
/// type (`topic`), associated entity (`entity`), and optional data (`data`).
///
/// - `component`: Source of the event (e.g., `"transaction"/"block"`).
/// - `topic`: Type/category of the event (e.g., `"accepted"/"removed"`).
/// - `entity`: Identifier for the related entity (e.g., `transaction hash`).
/// - `data`: Optional JSON data with additional event details.
#[derive(Clone, Debug)]
pub struct Event {
    pub component: &'static str,
    pub topic: &'static str,
    pub entity: String,
    pub data: Option<serde_json::Value>,
}

trait EventSource {
    const COMPONENT: &'static str;

    fn topic(&self) -> &'static str;
    fn entity(&self) -> String;
    fn data(&self) -> Option<serde_json::Value>;
}

impl<ES: EventSource> From<ES> for Event {
    fn from(value: ES) -> Self {
        Self {
            data: value.data(),
            topic: value.topic(),
            entity: value.entity(),
            component: ES::COMPONENT,
        }
    }
}
