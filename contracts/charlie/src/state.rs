// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate alloc;

use execution_core::stake::{
    Stake, Withdraw, WithdrawToContract, STAKE_CONTRACT,
};
use execution_core::transfer::{
    withdraw::Withdraw as TransferWithdraw, ContractToContract,
    ReceiveFromContract, TRANSFER_CONTRACT,
};

const SCRATCH_BUF_BYTES: usize = 256;

/// Charlie contract.
#[derive(Debug, Clone)]
pub struct Charlie;
impl Charlie {
    pub fn stake(&mut self, stake: Stake) {
        let value = stake.value();
        let data = rkyv::to_bytes::<_, SCRATCH_BUF_BYTES>(&stake)
            .expect("stake to be rkyv serialized")
            .to_vec();

        // make call to transfer contract to transfer balance from the user to
        // this contract
        let _: () = rusk_abi::call(TRANSFER_CONTRACT, "deposit", &value)
            .expect("Depositing funds into contract should succeed");

        let contract_to_contract = ContractToContract {
            contract: STAKE_CONTRACT,
            value,
            data,
            fn_name: "stake_from_contract".into(),
        };

        let _: () = rusk_abi::call(
            TRANSFER_CONTRACT,
            "contract_to_contract",
            &contract_to_contract,
        )
        .expect("Transferring to stake contract should succeed");
    }

    pub fn unstake(&mut self, unstake: Withdraw) {
        let value = unstake.transfer_withdraw().value();
        let data =
            rkyv::to_bytes::<_, SCRATCH_BUF_BYTES>(unstake.transfer_withdraw())
                .expect("withdraw to be rkyv serialized")
                .to_vec();

        let withdraw_to_contract = WithdrawToContract::new(
            *unstake.account(),
            value,
            "receive_unstake",
        )
        .with_data(data);

        let _: () = rusk_abi::call(
            STAKE_CONTRACT,
            "unstake_from_contract",
            &withdraw_to_contract,
        )
        .expect("Unstake from stake contract should succeed");
    }

    pub fn receive_unstake(&mut self, receive: ReceiveFromContract) {
        let withdraw: TransferWithdraw = rkyv::from_bytes(&receive.data)
            .expect("withdraw to be rkyv deserialized");
        // make call to the transfer contract to withdraw funds from this
        // contract into the receiver specified by the withdrawal.
        let _: () = rusk_abi::call(TRANSFER_CONTRACT, "withdraw", &withdraw)
            .expect("Withdrawing stake should succeed");
    }

    pub fn withdraw(&mut self, unstake: Withdraw) {
        let value = unstake.transfer_withdraw().value();
        let data =
            rkyv::to_bytes::<_, SCRATCH_BUF_BYTES>(unstake.transfer_withdraw())
                .expect("withdraw to be rkyv serialized")
                .to_vec();

        let withdraw_to_contract = WithdrawToContract::new(
            *unstake.account(),
            value,
            "receive_reward",
        )
        .with_data(data);

        let _: () = rusk_abi::call(
            STAKE_CONTRACT,
            "withdraw_from_contract",
            &withdraw_to_contract,
        )
        .expect("Withdraw rewards from stake contract should succeed");
    }

    pub fn receive_reward(&mut self, receive: ReceiveFromContract) {
        let withdraw: TransferWithdraw = rkyv::from_bytes(&receive.data)
            .expect("withdraw to be rkyv deserialized");
        // make call to the transfer contract to withdraw funds from this
        // contract into the receiver specified by the withdrawal.
        let _: () = rusk_abi::call(TRANSFER_CONTRACT, "withdraw", &withdraw)
            .expect("Withdrawing stake should succeed");
    }
}
