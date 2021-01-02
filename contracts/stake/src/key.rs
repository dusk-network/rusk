// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Counter;
use canonical::Canon;
use canonical_derive::Canon;
use core::cmp::Ordering;
use dusk_bls12_381_sign::APK;

#[derive(Debug, Default, Clone, Eq, PartialEq, Canon)]
pub struct Key {
    pub pk: APK,
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
