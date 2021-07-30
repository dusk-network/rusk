// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::TransferContract;

use dusk_abi::ContractId;
use dusk_bls12_381::BlsScalar;
use phoenix_core::{Crossover, Message};

const VD_EXEC_1_0: &[u8] = include_bytes!("../../../../.rusk/keys/88fb91d79f5774b0bb0b085377f9e81ce68ed069bc7ac23da01eeae94203739a.vd");
const VD_EXEC_1_1: &[u8] = include_bytes!("../../../../.rusk/keys/38df655538f70e71281df999d4d406165bf920873553eed5e1af8683c7d31877.vd");
const VD_EXEC_1_2: &[u8] = include_bytes!("../../../../.rusk/keys/4f44949e1d6acf1d856551947b7a2e617bfab827a53c2802dc9d1b5b3eda1a55.vd");
const VD_EXEC_2_0: &[u8] = include_bytes!("../../../../.rusk/keys/823c0426e8afa38056cfe194f5fac7f66e902d3b772017d768adddc33ae9be4d.vd");
const VD_EXEC_2_1: &[u8] = include_bytes!("../../../../.rusk/keys/84198e73c6c3fdbb123677f6d253d5f3308a1242ffed06daca981d45fb3182e5.vd");
const VD_EXEC_2_2: &[u8] = include_bytes!("../../../../.rusk/keys/0e812998feda952262f95623820b40c3072ed75d8e522059e64e0723162d2dc9.vd");
const VD_EXEC_3_0: &[u8] = include_bytes!("../../../../.rusk/keys/54496755ba2a1b1b7593c2ceaab3730d766e3a51db4141b588846e33202d04b9.vd");
const VD_EXEC_3_1: &[u8] = include_bytes!("../../../../.rusk/keys/1588f493ecb38918fa4441b4f4296fa0568f33bd9b3b74e1c5a797ea514ae988.vd");
const VD_EXEC_3_2: &[u8] = include_bytes!("../../../../.rusk/keys/340a667f272f1e271ea7b1b2fcd57b848faa50d060e318f796f7485146a45f73.vd");
const VD_EXEC_4_0: &[u8] = include_bytes!("../../../../.rusk/keys/b7003e57fcdf34b616133203c6ec042909319cd7cb0a1148893b879a24c5aba5.vd");
const VD_EXEC_4_1: &[u8] = include_bytes!("../../../../.rusk/keys/fce4e8df9fe9ca007d867560bdb89b81c38ad65dd4537d273ea0afb9b34f1ffe.vd");
const VD_EXEC_4_2: &[u8] = include_bytes!("../../../../.rusk/keys/7fc41595f464fcfcf461c2db7e2bcb555c8043616f37914563e01246ea62ec81.vd");

const VD_STCT: &[u8] = include_bytes!("../../../../.rusk/keys/360bf8400f3ffbeb80b4b2d8b802707677f1fca0cc80ca1e6d1b9d7fa7f849d6.vd");
const VD_STCO: &[u8] = include_bytes!("../../../../.rusk/keys/e94171031efc0ea71f83684d2f828c96578d9a26a73005532e3fd662d38f6d55.vd");
const VD_WDFT: &[u8] = include_bytes!("../../../../.rusk/keys/8a6fba4cdb27a01425409ed0b81c561684eff5b6c42460ab6307879346a8c33c.vd");
const VD_WDFO: &[u8] = include_bytes!("../../../../.rusk/keys/ca723891b3e7ad1bf8dd5238eff1f3483e12e2b2ac782eb18221124b812e6907.vd");

impl TransferContract {
    pub const fn verifier_data_execute(
        inputs: usize,
        outputs: usize,
    ) -> &'static [u8] {
        match (inputs, outputs) {
            (1, 0) => VD_EXEC_1_0,
            (1, 1) => VD_EXEC_1_1,
            (1, 2) => VD_EXEC_1_2,
            (2, 0) => VD_EXEC_2_0,
            (2, 1) => VD_EXEC_2_1,
            (2, 2) => VD_EXEC_2_2,
            (3, 0) => VD_EXEC_3_0,
            (3, 1) => VD_EXEC_3_1,
            (3, 2) => VD_EXEC_3_2,
            (4, 0) => VD_EXEC_4_0,
            (4, 1) => VD_EXEC_4_1,
            (4, 2) => VD_EXEC_4_2,
            _ => &[],
        }
    }

    pub const fn verifier_data_stco() -> &'static [u8] {
        VD_STCO
    }

    pub const fn verifier_data_stct() -> &'static [u8] {
        VD_STCT
    }

    pub const fn verifier_data_wdft() -> &'static [u8] {
        VD_WDFT
    }

    pub const fn verifier_data_wdfo() -> &'static [u8] {
        VD_WDFO
    }

    pub fn sign_message_stct(
        crossover: &Crossover,
        value: u64,
        address: &ContractId,
    ) -> BlsScalar {
        let mut m = crossover.to_hash_inputs().to_vec();

        m.push(value.into());
        m.push(Self::contract_to_scalar(address));

        #[cfg(not(target_arch = "wasm32"))]
        let message = dusk_poseidon::sponge::hash(m.as_slice());

        #[cfg(target_arch = "wasm32")]
        let message = rusk_abi::poseidon_hash(m);

        message
    }

    pub fn sign_message_stco(
        crossover: &Crossover,
        message: &Message,
        address: &ContractId,
    ) -> BlsScalar {
        let mut m = crossover.to_hash_inputs().to_vec();

        m.extend(&message.to_hash_inputs());
        m.push(Self::contract_to_scalar(address));

        #[cfg(not(target_arch = "wasm32"))]
        let message = dusk_poseidon::sponge::hash(m.as_slice());

        #[cfg(target_arch = "wasm32")]
        let message = rusk_abi::poseidon_hash(m);

        message
    }
}
