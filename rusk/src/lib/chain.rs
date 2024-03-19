// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod rusk;
mod vm;

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;

use node::database::rocksdb::Backend;
use node::network::Kadcast;
use rusk_abi::dusk::{dusk, Dusk};
use rusk_abi::VM;

pub const MINIMUM_STAKE: Dusk = dusk(1000.0);

#[derive(Debug, Clone, Copy)]
pub struct RuskTip {
    pub current: [u8; 32],
    pub base: [u8; 32],
    pub epoch: Option<[u8; 32]>,
}

#[derive(Clone)]
pub struct Rusk {
    pub(crate) tip: Arc<RwLock<RuskTip>>,
    pub(crate) vm: Arc<VM>,
    dir: PathBuf,
}

#[derive(Clone)]
pub struct RuskNode(pub node::Node<Kadcast<255>, Backend, Rusk>);

impl RuskNode {
    pub fn db(&self) -> Arc<tokio::sync::RwLock<Backend>> {
        self.0.database() as Arc<tokio::sync::RwLock<Backend>>
    }

    pub fn network(&self) -> Arc<tokio::sync::RwLock<Kadcast<255>>> {
        self.0.network() as Arc<tokio::sync::RwLock<Kadcast<255>>>
    }
}

/// Calculates the value that the coinbase notes should contain.
///
/// 90% of the total value goes to the generator (rounded up).
/// 10% of the total value goes to the Dusk address (rounded down).
const fn coinbase_value(block_height: u64, dusk_spent: u64) -> (Dusk, Dusk) {
    let value = emission_amount(block_height) + dusk_spent;

    let dusk_value = value / 10;
    let generator_value = value - dusk_value;

    (dusk_value, generator_value)
}

/// This implements the emission schedule described in the economic paper.
pub const fn emission_amount(block_height: u64) -> Dusk {
    match block_height {
        1..=12_500_000 => dusk(16.0),
        12_500_001..=18_750_000 => dusk(12.8),
        18_750_001..=25_000_000 => dusk(9.6),
        25_000_001..=31_250_000 => dusk(8.0),
        31_250_001..=37_500_000 => dusk(6.4),
        37_500_001..=43_750_000 => dusk(4.8),
        43_750_001..=50_000_000 => dusk(3.2),
        50_000_001..=62_500_000 => dusk(1.6),
        _ => dusk(0.0),
    }
}
