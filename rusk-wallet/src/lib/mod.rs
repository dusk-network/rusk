// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod clients;
pub mod crypto;
pub mod error;
pub mod prompt;
pub mod store;
pub mod wallet;

pub const SEED_SIZE: usize = 64;
pub const ONE_MILLION: f64 = 1_000_000.0;

/// The default gas price is 0.001 Dusk
pub(crate) const DEFAULT_GAS_PRICE: u64 = 1_000;

/// Convert from Dusk to uDusk
pub fn to_udusk(dusk: f64) -> u64 {
    (dusk * ONE_MILLION) as u64
}

/// Convert from uDusk to Dusk
pub fn to_dusk(udusk: u64) -> f64 {
    let udusk = udusk as f64;
    udusk / ONE_MILLION
}
