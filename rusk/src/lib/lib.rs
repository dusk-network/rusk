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
#[cfg(feature = "chain")]
pub mod rpc;

mod builder;
pub mod verifier;
mod version;

use std::sync::LazyLock;

pub use crate::error::Error;
pub use version::{VERSION, VERSION_BUILD};

pub use builder::Builder;
pub type Result<T, E = Error> = core::result::Result<T, E>;

use dusk_bytes::DeserializableSlice;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;

#[cfg(feature = "chain")]
pub use node::Rusk;

pub const DELETING_VM_FNAME: &str = ".delete";

pub static DUSK_CONSENSUS_KEY: LazyLock<BlsPublicKey> = LazyLock::new(|| {
    let dusk_cpk_bytes = include_bytes!("../assets/dusk.cpk");
    BlsPublicKey::from_slice(dusk_cpk_bytes)
        .expect("Dusk consensus public key to be valid")
});

#[cfg(feature = "testwallet")]
mod test_utils;
