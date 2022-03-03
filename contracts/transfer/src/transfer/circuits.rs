// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::TransferContract;

use dusk_abi::ContractId;
use dusk_bls12_381::BlsScalar;
use phoenix_core::{Crossover, Message};

const VD_STCT: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/647b6ad9caaa5874b6736196ddcbbc75743d91c086ab63ee8e8e0565c1d01d7f.vd"));
const VD_STCO: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/e46e8cdefa6886c993541672f0e3ccb604202d176cadb98a564524b000b365d3.vd"));
const VD_WDFT: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/74c6dd58dbbe98abd114055a9e822b377ea4128b2285843bc4686204c3c7db4b.vd"));
const VD_WDFO: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/9619d30b8afe56bb2dc9f60c3ed158a6bae69fde378e332e77f0481a6ec38725.vd"));

const VD_EXEC_1_0: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/9ab395060964aef3b2782ab40f2b79a52266d6e387c575f86f0c58a4e4054646.vd"));
const VD_EXEC_1_1: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/37c611d9e7b2ed5e7a36cb28e12429985e229a8ef14a7ab8e8a794b5ff357b7f.vd"));
const VD_EXEC_1_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/32d2127921621cab273663545165b834ad0c46bc7a54d962b48efe9d04b2a4ba.vd"));
const VD_EXEC_2_0: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/b98f2313407ff52d722fd18f9dceeada12120d38fca104f65377f96c12e39c18.vd"));
const VD_EXEC_2_1: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/148d977cc81fdef94248fdaa665667b8ff7fcb2fe452fa7d443f5677a7a010d5.vd"));
const VD_EXEC_2_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/adb3998d2b20d5edc17ef6c7f7b45d508577e8344aec4ae9d50404d94fbd1fab.vd"));
const VD_EXEC_3_0: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/215af52a899c4e1ff6634e9b1e9f5b065497e775a4eed74ce959d03a4c0ea894.vd"));
const VD_EXEC_3_1: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/4eddc9e9457085323fb19478bf63bb7017df8c49c9ffabc042adba1fc615e75c.vd"));
const VD_EXEC_3_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/b6acb4d7b430906d3dc899e8a47c0e6d7cfe7de559ad7b67126f417802557bf7.vd"));
const VD_EXEC_4_0: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/e6fe72b594613175fffd69aa9dc3114651e1431152ac90d9c345b40751c99ef3.vd"));
const VD_EXEC_4_1: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/30bbbf7f6fa0df339d9ac03a053aff097a9ccb7c574633788751ec19dc9f3bac.vd"));
const VD_EXEC_4_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/38e503db62995503f5a03498bbcd2826fb51e4d0b24fc34f92980458712f9066.vd"));

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
        m.push(rusk_abi::contract_to_scalar(address));

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
        m.push(rusk_abi::contract_to_scalar(address));

        #[cfg(not(target_arch = "wasm32"))]
        let message = dusk_poseidon::sponge::hash(m.as_slice());

        #[cfg(target_arch = "wasm32")]
        let message = rusk_abi::poseidon_hash(m);

        message
    }
}
