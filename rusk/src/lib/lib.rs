// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![feature(lazy_cell)]

#[cfg(feature = "node")]
pub mod chain;
mod error;
#[cfg(feature = "node")]
pub mod free_tx_verifier;
pub mod http;
pub mod pow_verifier;
pub mod verifier;
mod version;

pub use crate::error::Error;
pub use version::{VERSION, VERSION_BUILD};

pub type Result<T, E = Error> = core::result::Result<T, E>;
#[cfg(feature = "node")]
pub use chain::Rusk;

#[cfg(feature = "testwallet")]
mod test_utils;
