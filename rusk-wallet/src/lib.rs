// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The wallet specification.

#![deny(missing_docs)]
#![deny(clippy::all)]
#![no_std]

#[macro_use]
extern crate alloc;

#[cfg(target_family = "wasm")]
mod ffi;
mod imp;
mod tx;

use alloc::vec::Vec;
use dusk_pki::{SecretSpendKey, ViewKey};
use phoenix_core::Note;

pub use imp::*;
pub use tx::Transaction;

/// The key store backend - where the keys live.
pub trait Store {
    /// The identifier for the stored key.
    type Id;
    /// The error type returned from the store.
    type Error;

    /// Stores the given key in the store under the given `id`.
    fn store_key(
        &self,
        id: &Self::Id,
        key: &SecretSpendKey,
    ) -> Result<(), Self::Error>;

    /// Retrieves a key from the store.
    fn key(&self, id: &Self::Id)
        -> Result<Option<SecretSpendKey>, Self::Error>;
}

/// Provides notes to the caller.
pub trait NoteFinder {
    /// Error returned by the note finder.
    type Error;

    /// Find notes for a view key, starting from the given block height.
    fn find_notes(
        &self,
        height: u64,
        vk: &ViewKey,
    ) -> Result<Vec<Note>, Self::Error>;
}
