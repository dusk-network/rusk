// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// License contract.
#[derive(Debug, Clone)]
pub struct License;

impl License {
    pub const fn new() -> Self {
        Self
    }

    #[allow(dead_code)]
    pub fn identifier() -> &'static [u8; 7] {
        b"license"
    }
}

impl License {
    pub fn ping(&mut self) {
        rusk_abi::debug!("License ping");
    }
}
