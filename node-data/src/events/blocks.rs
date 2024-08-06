// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;
use crate::ledger::{Block, Hash};

impl EventSource for BlockEvent<'_> {
    const COMPONENT: &'static str = "blocks";

    fn topic(&self) -> &'static str {
        match self {
            Self::Accepted(_) => "accepted",
            Self::StateChange { .. } => "statechange",
        }
    }
    fn data(&self) -> EventData {
        match self {
            Self::Accepted(_) => EventData::None,
            Self::StateChange { state, height, .. } => {
                EventData::Json(serde_json::json!({
                    "state": state,
                    "atHeight": height,
                }))
            }
        }
    }
    fn entity(&self) -> String {
        let hash = match self {
            Self::Accepted(block) => block.header().hash,
            Self::StateChange { hash, .. } => *hash,
        };
        hex::encode(hash)
    }
}

#[derive(Clone, Debug)]
pub enum BlockEvent<'b> {
    Accepted(&'b Block),
    StateChange {
        hash: Hash,
        state: &'static str,
        height: u64,
    },
}
