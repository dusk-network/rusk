// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// Bob contract.
#[derive(Debug, Clone)]
pub struct Bob {
    value: u8,
}

impl Bob {
    pub const fn new() -> Self {
        Self { value: 0 }
    }

    #[allow(dead_code)]
    pub fn identifier() -> &'static [u8; 3] {
        b"bob"
    }
}

impl Bob {
    pub fn init(&mut self, n: u8) {
        self.value = n;
    }

    pub fn ping(&mut self) {}

    pub fn echo(&mut self, n: u64) -> u64 {
        n
    }

    pub fn value(&mut self) -> u8 {
        self.value
    }
}
