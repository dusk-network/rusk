// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The wallet specification.

#![deny(missing_docs)]
#![deny(clippy::all)]

#[cfg(target_family = "wasm")]
mod ffi;
mod imp;

use dusk_pki::SecretSpendKey;

pub use imp::*;

/// The key store backend - where the keys live.
pub trait Store {
    /// The identifier for the stored key.
    type Id;
    /// The error type returned from the store.
    type Error;

    /// Stores the given key in the store under the given `id`.
    fn store_key(
        &mut self,
        id: &Self::Id,
        key: &SecretSpendKey,
    ) -> Result<(), Self::Error>;

    /// Retrieves a key from the store.
    fn key(&self, id: &Self::Id)
        -> Result<Option<SecretSpendKey>, Self::Error>;
}
