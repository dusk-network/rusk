// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod blocks;
mod transactions;

pub use blocks::BlockEvent;
pub use transactions::TransactionEvent;

#[derive(Clone, Debug)]
pub struct Event {
    pub component: &'static str,
    pub topic: &'static str,
    pub entity: String,
    pub data: EventData,
}

pub trait EventSource {
    const COMPONENT: &'static str;

    fn topic(&self) -> &'static str;
    fn entity(&self) -> String;
    fn data(&self) -> EventData;
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

#[derive(Clone, Debug)]
pub enum EventData {
    None,
    Json(serde_json::Value),
    Text(String),
    Binary(Vec<u8>),
}
