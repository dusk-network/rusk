// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::abi::{self, ContractId};
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::stake::{Stake, StakeData, STAKE_CONTRACT};
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
        use rkyv;
        const SCRATCH_BUF_BYTES: usize = 256;
        const CHARLIE_ID: ContractId = ContractId::from_bytes([4; 32]);

        // adding a query to the transfer contract reproduces the wasm trap
        abi::call::<_, u64>(TRANSFER_CONTRACT, "root", &())
            .expect("quering the transfer contract should succeed");

        // adding a query to the stake contract doesn't reproduce the wasm trap
        // let provisioner = include_bytes!("../../../rusk/src/assets/dusk.cpk");
        // use dusk_bytes::Serializable;
        // let provisioner = BlsPublicKey::from_bytes(&provisioner)
        //     .expect("The pk should be a valid point");
        // abi::call::<_, Option<StakeData>>(
        //     STAKE_CONTRACT,
        //     "get_stake",
        //     &provisioner,
        // )
        // .expect("calling get_stake should succeed");

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

// fn ds_address(ds_str: &str) -> BlsPublicKey {
//     // let ds_pk_bytes = bs58::decode(ds_str)
//     //     .into_vec()
//     //     .expect("address string should be bs58 encoded");
//     let ds_pk_bytes = hex::decode(ds_str).expect("decoding hex should work");
//     let ds_pk_bytes: [u8; 96] = ds_pk_bytes
//         .try_into()
//         .expect("the pk should be exactly 96 bytes");
//     BlsPublicKey::from_bytes(&ds_pk_bytes)
//         .expect("The pk should be a valid point")
// }
