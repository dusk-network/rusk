// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Wallet files and key management

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(clippy::pedantic)]
#![deny(unused_crate_dependencies)]
#![deny(unused_extern_crates)]

mod crypto;
mod error;

pub mod provisioner;
pub mod rusk_wallet;

pub use error::Error;

/// Size in bytes of the IV used to encrypt wallet data
pub(crate) const IV_SIZE: usize = 12;
/// Size in bytes of the salt used to encrypt wallet data
pub(crate) const SALT_SIZE: usize = 32;
/// Number of PBKDF2 rounds used to derive the key for encrypting wallet data
pub(crate) const PBKDF2_ROUNDS: u32 = 10_000;

#[cfg(test)]
mod deps {
    use anyhow as _;
    use tempfile as _;
}
