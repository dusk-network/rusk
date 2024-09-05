// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use execution_core::transfer::{
    withdraw::Withdraw, TransferToAccount, TransferToContract,
    TRANSFER_CONTRACT,
};

/// Alice contract.
#[derive(Debug, Clone)]
pub struct Alice;

impl Alice {
    pub fn ping(&mut self) {
        // no-op
    }

    pub fn withdraw(&mut self, withdraw: Withdraw) {
        let _: () = rusk_abi::call(TRANSFER_CONTRACT, "withdraw", &withdraw)
            .expect("Transparent withdrawal transaction should succeed");
    }

    pub fn deposit(&mut self, value: u64) {
        let _: () = rusk_abi::call(TRANSFER_CONTRACT, "deposit", &value)
            .expect("Transparent deposit transaction should succeed");
    }

    pub fn transfer_to_contract(&mut self, transfer: TransferToContract) {
        let _: () = rusk_abi::call(
            TRANSFER_CONTRACT,
            "transfer_to_contract",
            &transfer,
        )
        .expect("Transferring to contract should succeed");
    }

    pub fn transfer_to_account(&mut self, transfer: TransferToAccount) {
        rusk_abi::call::<_, ()>(
            TRANSFER_CONTRACT,
            "transfer_to_account",
            &transfer,
        )
        .expect("Transferring to account should succeed");
    }
}
