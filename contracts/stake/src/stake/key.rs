// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::Counter;
use canonical::Canon;
use canonical_derive::Canon;
use core::cmp::Ordering;
use dusk_bls12_381_sign::APK;

/// The key used in the staking contract's key-value store.
#[derive(Debug, Default, Clone, Eq, PartialEq, Canon)]
pub struct Key {
    /// The provisioner's public key.
    pub pk: APK,
    /// The provisioner's index in the identifier set.
    pub w_i: Counter,
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> Ordering {
        self.w_i.cmp(&other.w_i)
    }
}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
