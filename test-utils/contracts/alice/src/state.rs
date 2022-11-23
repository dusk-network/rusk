// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alice_contract_types::*;
use rusk_abi::{ModuleId, State};
use transfer_contract_types::*;

/// Alice contract.
///
#[derive(Debug, Clone)]
pub struct Alice {
    transfer: ModuleId,
}

impl Alice {
    pub const fn new(transfer: ModuleId) -> Self {
        Self { transfer }
    }

    #[allow(dead_code)]
    pub const fn identifier() -> &'static [u8; 5] {
        b"alice"
    }
}

impl Alice {
    pub fn ping(&mut self) {
        rusk_abi::debug!("Alice ping");
    }

    pub fn withdraw(self: &mut State<Self>, withdraw: Withdraw) {
        let _: bool = self
            .transact(
                self.transfer,
                "wfct",
                Wfct {
                    value: withdraw.value,
                    note: withdraw.note,
                    proof: withdraw.proof,
                },
            )
            .expect("Transparent withdrawal transaction should succeed");
    }

    pub fn withdraw_obfuscated(
        self: &mut State<Self>,
        withdraw_obfuscated: WithdrawObfuscated,
    ) {
        let _: bool = self
            .transact(
                self.transfer,
                "wfco",
                Wfco {
                    message: withdraw_obfuscated.message,
                    message_address: withdraw_obfuscated.message_address,
                    change: withdraw_obfuscated.change,
                    change_address: withdraw_obfuscated.change_address,
                    output: withdraw_obfuscated.output,
                    proof: withdraw_obfuscated.proof,
                },
            )
            .expect("Obfuscated withdrawal transaction should succeed");
    }

    pub fn withdraw_to_contract(
        self: &mut State<Self>,
        withdraw_to_contract: WithdrawToContract,
    ) {
        let _: bool = self
            .transact(
                self.transfer,
                "wfctc",
                Wfctc {
                    module: withdraw_to_contract.module,
                    value: withdraw_to_contract.value,
                },
            )
            .expect("Obfuscated withdrawal transaction should succeed");
    }
}
