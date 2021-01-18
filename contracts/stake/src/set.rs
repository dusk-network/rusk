// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{Counter, Key};
use canonical::{Canon, Store};
use canonical_derive::Canon;
use dusk_kelvin_map::Map;

/// This set contains all of the provisioners in the contract, ordered by time
/// of addition. It is used to retrieve a key associated to a provisioner, which
/// can then be used to access the staking contract map, and allows a user to
/// retrieve information, or update the expiration period.
#[derive(Debug, Clone, Canon)]
pub struct IdentifierSet<S: Store>(Map<Counter, Key, S>);

impl<S> Default for IdentifierSet<S>
where
    S: Store,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S> IdentifierSet<S>
where
    S: Store,
{
    /// Create a new instance of a [`IdentifierSet`].
    pub fn new() -> IdentifierSet<S> {
        Self(Map::<Counter, Key, S>::default())
    }

    /// Include a key -> value mapping to the set.
    ///
    /// If the key was previously mapped, it will return the old value in the
    /// form `Ok(Some(Key))`.
    ///
    /// If the key was not previously mapped, the return will be `Ok(None)`
    pub fn insert(
        &mut self,
        w_i: Counter,
        key: Key,
    ) -> Result<Option<Key>, S::Error> {
        self.0.insert(w_i, key)
    }

    /// Fetch a previously inserted key -> value mapping, provided the key.
    ///
    /// Will return `Ok(None)` if no correspondent key was found.
    pub fn get(&self, w_i: Counter) -> Result<Option<Key>, S::Error> {
        self.0.get(&w_i)
    }
}
