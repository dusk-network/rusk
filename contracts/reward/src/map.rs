// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Key;
use canonical::{Canon, Store};
use canonical_derive::Canon;
use core::ops::Deref;
use dusk_bls12_381_sign::APK;
use dusk_kelvin_map::Map;

/// A mapping, where the provisioner BLS public key maps to an amount of DUSK,
/// and the last withdrawal time.
#[derive(Debug, Clone, Canon)]
pub struct BalanceMapping<S: Store>(Map<Key, (u64, u64), S, 32>);

impl<S> BalanceMapping<S>
where
    S: Store,
{
    /// Create a new instance of a [`BalanceMapping`].
    pub fn new() -> BalanceMapping<S> {
        Self(Map::<Key, (u64, u64), S, 32>::default())
    }

    /// Include a key -> value mapping to the set.
    ///
    /// If the key was previously mapped, it will return the old value in the
    /// form `Ok(Some((u64, u64)))`.
    ///
    /// If the key was not previously mapped, the return will be `Ok(None)`
    pub fn insert(
        &mut self,
        key: APK,
        amount: u64,
        block_height: u64,
    ) -> Result<Option<(u64, u64)>, S::Error> {
        self.0.insert(Key::new(key), (amount, block_height))
    }

    /// Fetch a previously inserted key -> value mapping, provided the key.
    ///
    /// Will return `Ok(None)` if no correspondent key was found.
    pub fn get<'a>(
        &'a self,
        key: APK,
    ) -> Result<Option<impl Deref<Target = (u64, u64)> + 'a>, S::Error> {
        self.0.get(&Key::new(key))
    }

    /// Delete a previously inserted key -> value mapping, provided the key.
    ///
    /// Will return `Ok(None)` if no correspondent key was found.
    ///
    /// If the key was previously mapped, it will return the deleted value
    /// in the form of `Ok(Some((u64, u64)))`.
    pub fn delete(&mut self, key: APK) -> Result<Option<(u64, u64)>, S::Error> {
        self.0.remove(&Key::new(key))
    }
}
