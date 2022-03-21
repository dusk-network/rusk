// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub(crate) mod cache;
pub mod clients;
pub mod config;
pub mod crypto;
pub mod error;
pub mod prompt;
pub mod store;
pub mod wallet;

use rusk_abi::dusk::*;

pub const SEED_SIZE: usize = 64;

pub(crate) const MAX_CONVERTIBLE: f64 = f64::MAX / dusk(1.0) as f64;
pub(crate) const MIN_CONVERTIBLE: f64 = from_dusk(LUX);

pub(crate) const MIN_GAS_LIMIT: u64 = 350_000_000;
pub(crate) const DEFAULT_GAS_LIMIT: u64 = 500_000_000;
pub(crate) const DEFAULT_GAS_PRICE: f64 = from_dusk(LUX);
