// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::TransferContract;

use dusk_abi::ContractId;
use dusk_bls12_381::BlsScalar;
use phoenix_core::{Crossover, Message};

const VD_EXEC_1_0: &'static [u8] = include_bytes!("../../../../.rusk/keys/9220aa7d7aa9ab5731063e10fbed1ecd6430df7921235519e99f5c7414c2f004.vd");
const VD_EXEC_1_1: &'static [u8] = include_bytes!("../../../../.rusk/keys/addd677cdd85b781be6f1baf76678b8617370296978df7437df1a1d6e5bd8bc1.vd");
const VD_EXEC_1_2: &'static [u8] = include_bytes!("../../../../.rusk/keys/7c845e256eef308949849e1f1988a2902d320d1d4575b8a1cd387c993ac4d850.vd");
const VD_EXEC_2_0: &'static [u8] = include_bytes!("../../../../.rusk/keys/9b7680ea27836033c8df81ecb8f0abf592d402cecc7392d4cf5b61ee4919cb5f.vd");
const VD_EXEC_2_1: &'static [u8] = include_bytes!("../../../../.rusk/keys/e8a222ee1d5d08a19b4676dde866fa630258663826714c01966ad30cee8d2ec3.vd");
const VD_EXEC_2_2: &'static [u8] = include_bytes!("../../../../.rusk/keys/702d241dd437e76f6fd38674e6d117466cd22b5129bc9e3f5faf1862542c70ba.vd");
const VD_EXEC_3_0: &'static [u8] = include_bytes!("../../../../.rusk/keys/e55ea5240e0001e39453bd98a5854317977e921c3ec2457a799ea5281fa872d8.vd");
const VD_EXEC_3_1: &'static [u8] = include_bytes!("../../../../.rusk/keys/1cede7d18208f247b08ed50d319ccbbbdbe2731d1f74b74801f1fb7587a1097a.vd");
const VD_EXEC_3_2: &'static [u8] = include_bytes!("../../../../.rusk/keys/2484ebace4e04500289c2af16402ec2b9aa83c25b26c36afcf54a9fb8c4331e2.vd");
const VD_EXEC_4_0: &'static [u8] = include_bytes!("../../../../.rusk/keys/1de86e260f13683380061a4ff439e2a667fd752b2f08df70aec0937c64d4e2b6.vd");
const VD_EXEC_4_1: &'static [u8] = include_bytes!("../../../../.rusk/keys/39966923862ad04073ac17c5bce63c4ed4025804433988b634c0eb3a46e4e0ef.vd");
const VD_EXEC_4_2: &'static [u8] = include_bytes!("../../../../.rusk/keys/f1fb693440ac177dc0d17c8b10ad9f231d85c6ae706af5d2106ba9c93c3e26a3.vd");

const VD_STCO: &'static [u8] = include_bytes!("../../../../.rusk/keys/51dd1bd597d3f665d0e98a027af9859292236f21f5bf3dcb4a80983529bc8fc3.vd");
const VD_STCT: &'static [u8] = include_bytes!("../../../../.rusk/keys/709f69ba81cd985c2c443cbb0ca7eaa7fe0db5b3aa8769ba3888ccc93053241d.vd");
const VD_WDFT: &'static [u8] = include_bytes!("../../../../.rusk/keys/f7e93e251f330e245573c103cc6fcd8f2a59ff6fcdaa437740c408da243054c7.vd");
const VD_WDFO: &'static [u8] = include_bytes!("../../../../.rusk/keys/ecf870e3a32f45ac0358769c0be66ded6c4d51ae7de715da013e3d327b4a384f.vd");

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
