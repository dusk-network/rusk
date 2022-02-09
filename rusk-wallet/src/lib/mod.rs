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

// Types that define Dusk's denomination.
pub type Dusk = f64;
pub type MicroDusk = u64;

pub const SEED_SIZE: usize = 64;

pub const ONE_MILLION: Dusk = 1_000_000.0;

pub(crate) const DEFAULT_GAS_LIMIT: u64 = 100000;

pub(crate) const DEFAULT_GAS_PRICE: Dusk = 0.000001;

/// Convert from Dusk to uDusk
pub fn to_udusk(dusk: Dusk) -> MicroDusk {
    (dusk * ONE_MILLION) as MicroDusk
}

/// Convert from uDusk to Dusk
pub fn to_dusk(udusk: MicroDusk) -> Dusk {
    let udusk = udusk as Dusk;
    udusk / ONE_MILLION
}
