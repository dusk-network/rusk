// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// Bob contract.
#[derive(Debug, Clone)]
pub struct Bob;

impl Bob {
    pub const fn new() -> Self {
        Self
    }

    #[allow(dead_code)]
    pub fn identifier() -> &'static [u8; 3] {
        b"bob"
    }
}

impl Bob {
    pub fn ping(&mut self) {
        rusk_abi::debug!("Bob ping");
    }
}
