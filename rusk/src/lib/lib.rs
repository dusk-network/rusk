// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![feature(lazy_cell)]

mod bloom;
mod error;
pub mod http;
#[cfg(feature = "chain")]
pub mod node;

mod builder;
pub mod verifier;
mod version;

pub use crate::error::Error;
pub use version::{VERSION, VERSION_BUILD};

pub use builder::Builder;
pub type Result<T, E = Error> = core::result::Result<T, E>;

#[cfg(feature = "chain")]
pub use node::Rusk;

pub const DELETING_VM_FNAME: &str = ".delete";

#[cfg(feature = "testwallet")]
mod test_utils;
