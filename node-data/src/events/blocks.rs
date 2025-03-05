// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;
use crate::ledger::{Block, Hash};

/// Represents the state of an accepted block in the chain.
///
/// # Variants
///
/// - `Confirmed` - The block has been confirmed. This means a succeeding block
///   has been accepted.
/// - `Finalized` - The block has been finalized i.e., reached finality.
///
/// # Methods
///
/// - `as_str() -> &'static str` - Returns the string representation of the
///   block state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockState {
    Confirmed,
    Finalized,
}

impl BlockState {
    const BLOCK_CONFIRMED: &'static str = "confirmed";
    const BLOCK_FINALIZED: &'static str = "finalized";

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Confirmed => Self::BLOCK_CONFIRMED,
            Self::Finalized => Self::BLOCK_FINALIZED,
        }
    }
}

/// Represents events related to blocks in the chain.
///
/// # Variants
///
/// - `Accepted(&'b Block)`
///
///     Indicates that a block has been accepted into the chain.
///
/// - `StateChange`
///
///     Represents a change in the state of an accepted block.
///
///     - `hash: Hash` The unique identifier of the block whose state has
///       changed.
///
///     - `state: BlockState` Describes the new state of the block (can be
///       either `Confirmed` or `Finalized`).
///
///     - `height: u64` Indicates at which block height the state changed.
///
/// - `Reverted`
///
///     Indicates that a block has been removed from the chain because it got
///     reverted during consensus.
#[derive(Clone, Debug)]
pub enum BlockEvent<'b> {
    Accepted(&'b Block),
    StateChange {
        hash: Hash,
        state: BlockState,
        height: u64,
    },
    Reverted {
        hash: Hash,
        height: u64,
    },
}

impl EventSource for BlockEvent<'_> {
    const COMPONENT: &'static str = "blocks";

    fn topic(&self) -> &'static str {
        match self {
            Self::Accepted(_) => "accepted",
            Self::StateChange { .. } => "statechange",
            Self::Reverted { .. } => "reverted",
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
                    "state": state.as_str(),
                    "atHeight": height,
                })
            }
            BlockEvent::Reverted { height, .. } => {
                serde_json::json!({
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
            Self::Reverted { hash, .. } => *hash,
        };
        hex::encode(hash)
    }
}
