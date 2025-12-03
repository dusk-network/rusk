// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::abi::{self, ContractId};
use dusk_core::stake::Stake;
use dusk_core::transfer::{
    withdraw::Withdraw, ContractToAccount, ContractToContract,
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
        let _: () = abi::call(TRANSFER_CONTRACT, "withdraw", &withdraw)
            .expect("Transparent withdrawal transaction should succeed");
    }

    pub fn deposit(&mut self, value: u64) {
        let _: () = abi::call(TRANSFER_CONTRACT, "deposit", &value)
            .expect("Transparent deposit transaction should succeed");
    }

    pub fn contract_to_contract(&mut self, transfer: ContractToContract) {
        let _: () =
            abi::call(TRANSFER_CONTRACT, "contract_to_contract", &transfer)
                .expect("Transferring to contract should succeed");
    }

    pub fn contract_to_account(&mut self, transfer: ContractToAccount) {
        abi::call::<_, ()>(TRANSFER_CONTRACT, "contract_to_account", &transfer)
            .expect("Transferring to account should succeed");
    }

    pub fn stake_activate(&mut self, stake: Stake) {
        const SCRATCH_BUF_BYTES: usize = 256;
        const CHARLIE_ID: ContractId = ContractId::from_bytes([4; 32]);

        // adding a query to the transfer contract reproduces the wasm trap
        abi::call::<_, u64>(TRANSFER_CONTRACT, "root", &())
            .expect("quering the transfer contract should succeed");

        let data = rkyv::to_bytes::<_, SCRATCH_BUF_BYTES>(&stake)
            .expect("Stake should be rkyv serialized correctly")
            .to_vec();

        let transfer = ContractToContract {
            contract: CHARLIE_ID,
            value: stake.value(),
            fn_name: "stake_from_contract".into(),
            data,
        };

        abi::call::<_, ()>(
            TRANSFER_CONTRACT,
            "contract_to_contract",
            &transfer,
        )
        .expect(
            "Staking to the stake contract via the relayer contract should succeed",
        );
    }
}
