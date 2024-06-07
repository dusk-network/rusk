// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use execution_core::transfer::Mint;
use rusk_abi::TRANSFER_CONTRACT;

/// Alice contract.
#[derive(Debug, Clone)]
pub struct Alice;

impl Alice {
    pub fn ping(&mut self) {
        // no-op
    }

    pub fn withdraw(&mut self, mint: Mint) {
        let _: () = rusk_abi::call(TRANSFER_CONTRACT, "withdraw", &mint)
            .expect("Transparent withdrawal transaction should succeed");
    }

    pub fn deposit(&mut self, value: u64) {
        let _: () = rusk_abi::call(TRANSFER_CONTRACT, "deposit", &value)
            .expect("Transparent withdrawal transaction should succeed");
    }
}
