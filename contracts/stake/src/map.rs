// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Key;
use crate::Stake;
use canonical::{Canon, Store};
use canonical_derive::Canon;
use dusk_kelvin_map::Map;

/// A mapping, where the key (concatenation of a public key and a 32-byte
/// number) maps to a Stake.
#[derive(Debug, Clone, Canon)]
pub struct StakeMapping<S: Store>(Map<Key, Stake, S>);

impl<S> Default for StakeMapping<S>
where
    S: Store,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S> StakeMapping<S>
where
    S: Store,
{
    /// Create a new instance of a [`StakeMapping`].
    pub fn new() -> StakeMapping<S> {
        Self(Map::<Key, Stake, S>::default())
    }

    /// Include a key -> value mapping to the set.
    ///
    /// If the key was previously mapped, it will return the old value in the
    /// form `Ok(Some(Stake))`.
    ///
    /// If the key was not previously mapped, the return will be `Ok(None)`
    pub fn insert(
        &mut self,
        key: Key,
        stake: Stake,
    ) -> Result<Option<Stake>, S::Error> {
        self.0.insert(key, stake)
    }

    /// Fetch a previously inserted key -> value mapping, provided the key.
    ///
    /// Will return `Ok(None)` if no correspondent key was found.
    pub fn get(&self, key: &Key) -> Result<Option<Stake>, S::Error> {
        self.0.get(key)
    }

    /// Delete a previously inserted key -> value mapping, provided the key.
    ///
    /// Will return `Ok(None)` if no correspondent key was found.
    ///
    /// If the key was previously mapped, it will return the deleted value
    /// in the form of `Ok(Some(Stake))`.
    pub fn delete(&mut self, key: &Key) -> Result<Option<Stake>, S::Error> {
        self.0.remove(key)
    }
}
