// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate alloc;

use alloc::string::ToString;

use execution_core::stake::{Stake, STAKE_CONTRACT};
use execution_core::transfer::{ContractToContract, TRANSFER_CONTRACT};

const SCRATCH_BUF_BYTES: usize = 256;

/// Bob contract.
#[derive(Debug, Clone)]
pub struct Charlie;
impl Charlie {
    pub fn stake(&mut self, stake: Stake) {
        let value = stake.value();
        let contract = STAKE_CONTRACT;
        let fn_name = "stake_from_contract".to_string();
        let data = rkyv::to_bytes::<_, SCRATCH_BUF_BYTES>(&stake)
            .expect("stake to be rkyv serialized")
            .to_vec();

        // make call to transfer contract to transfer balance from the user to
        // this contract
        let _: () =
            rusk_abi::call::<_, ()>(TRANSFER_CONTRACT, "deposit", &value)
                .expect("Depositing funds into contract should succeed");

        let contract_to_contract = ContractToContract {
            contract,
            value,
            data,
            fn_name,
        };

        let _: () = rusk_abi::call(
            TRANSFER_CONTRACT,
            "contract_to_contract",
            &contract_to_contract,
        )
        .expect("Transferring to stake contract should succeed");
    }
}
