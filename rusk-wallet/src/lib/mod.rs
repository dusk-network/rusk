// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub(crate) mod cache;
pub(crate) mod clients;
pub(crate) mod config;
pub(crate) mod crypto;
pub(crate) mod error;
pub(crate) mod gql;
pub(crate) mod prompt;
pub(crate) mod store;
pub(crate) mod wallet;

use rusk_abi::dusk::*;

pub(crate) const SEED_SIZE: usize = 64;

pub(crate) const MAX_CONVERTIBLE: f64 = f64::MAX / dusk(1.0) as f64;
pub(crate) const MIN_CONVERTIBLE: f64 = from_dusk(LUX);

pub(crate) const MIN_GAS_LIMIT: u64 = 350_000_000;
pub(crate) const DEFAULT_GAS_LIMIT: u64 = 500_000_000;
pub(crate) const DEFAULT_GAS_PRICE: f64 = from_dusk(LUX);
