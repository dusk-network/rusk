// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical::Canon;
use canonical_derive::Canon;

/// This is a simple 32-byte counter. The number is counted upwards in
/// big-endian fashion. We use a big-endian number because the keys to
/// the stake identifier map will be sorted in ascending order this way,
/// as the `Ord` implementation for byte slices always checks for
/// lexicographical ordering.
#[derive(
    Debug, Clone, Copy, Default, Ord, PartialOrd, Eq, PartialEq, Canon,
)]
pub struct Counter([u8; 32]);

impl Counter {
    /// Increment the [`Counter`] by one, in a big-endian fashion.
    pub fn increment(&mut self) {
        let mut carry = 1;
        for i in (0..self.0.len()).rev() {
            self.0[i] += carry;
            if self.0[i] != 0 {
                carry = 0;
            }

            if carry == 0 {
                break;
            }
        }
    }
}
