// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use phoenix_core::transaction::*;
use rusk_abi::TRANSFER_CONTRACT;

/// Alice contract.
#[derive(Debug, Clone)]
pub struct Alice;

impl Alice {
    pub fn ping(&mut self) {
        // no-op
    }

    pub fn withdraw(&mut self, wfct: Wfct) {
        let _: bool = rusk_abi::call(TRANSFER_CONTRACT, "wfct", &wfct)
            .expect("Transparent withdrawal transaction should succeed");
    }

    pub fn withdraw_to_contract(&mut self, wfctc: Wfctc) {
        let _: bool = rusk_abi::call(TRANSFER_CONTRACT, "wfctc", &wfctc)
            .expect("Withdrawal tco contract transaction should succeed");
    }
}
