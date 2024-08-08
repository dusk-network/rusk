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
    fn data(&self) -> Option<serde_json::Value> {
        let data = match self {
            Self::Accepted(b) => {
                let header = b.header();
                let header = serde_json::to_value(header)
                    .expect("json to be serialized");
                let txs: Vec<_> =
                    b.txs().iter().map(|t| hex::encode(t.id())).collect();
                serde_json::json!({
                    "header": header,
                    "transactions": txs,
                })
            }
            Self::StateChange { state, height, .. } => {
                serde_json::json!({
                    "state": state,
                    "atHeight": height,
                })
            }
        };
        Some(data)
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
