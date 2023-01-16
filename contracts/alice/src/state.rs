// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk_abi::State;
use transfer_contract_types::*;

/// Alice contract.
#[derive(Debug, Clone)]
pub struct Alice;

impl Alice {
    pub fn ping(&mut self) {
        // no-op
    }

    pub fn withdraw(self: &mut State<Self>, wfct: Wfct) {
        let _: bool = self
            .transact(rusk_abi::transfer_module(), "wfct", &wfct)
            .expect("Transparent withdrawal transaction should succeed");
    }

    pub fn withdraw_obfuscated(self: &mut State<Self>, wfco: Wfco) {
        let _: bool = self
            .transact(rusk_abi::transfer_module(), "wfco", &wfco)
            .expect("Obfuscated withdrawal transaction should succeed");
    }

    pub fn withdraw_to_contract(self: &mut State<Self>, wfctc: Wfctc) {
        let _: bool = self
            .transact(rusk_abi::transfer_module(), "wfctc", &wfctc)
            .expect("Obfuscated withdrawal transaction should succeed");
    }
}
