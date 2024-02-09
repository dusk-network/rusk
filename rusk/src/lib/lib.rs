// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![feature(lazy_cell)]

pub mod chain;
mod error;
pub mod http;
#[cfg(feature = "testwallet")]
mod test_utils;
pub mod verifier;
mod version;

pub use crate::error::Error;
pub use version::{VERSION, VERSION_BUILD};

pub type Result<T, E = Error> = core::result::Result<T, E>;
pub use chain::Rusk;