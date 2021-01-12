// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical::Canon;
use canonical_derive::Canon;
use core::cmp::Ordering;
use dusk_bls12_381_sign::APK;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Canon)]
pub struct Key(APK);

impl Key {
    pub fn new(k: APK) -> Self {
        Key(k)
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> Ordering {
        let first = self.0.to_bytes();
        let second = other.0.to_bytes();
        for i in 0..first.len() {
            if first[i] > second[i] {
                return Ordering::Greater;
            } else if first[i] < second[i] {
                return Ordering::Less;
            }
        }

        Ordering::Equal
    }
}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
