// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::TransferContract;

use dusk_abi::ContractId;
use dusk_bls12_381::BlsScalar;
use phoenix_core::{Crossover, Message};

const VD_EXEC_1_0: &'static [u8] = include_bytes!("../../../../.rusk/keys/a5c853e944fb47020b6fcf98c5d03046af6b5f8129bcd75f95603099b3b7e99f.vd");
const VD_EXEC_1_1: &'static [u8] = include_bytes!("../../../../.rusk/keys/3e6d01794d32bcf30ea57c325d934056f98f61549202c58e0874470bed573470.vd");
const VD_EXEC_1_2: &'static [u8] = include_bytes!("../../../../.rusk/keys/39327f93d5c9d952cf190fe4b5af878eacce5d887de9d3fd2775dbbf5c3e55fd.vd");
const VD_EXEC_2_0: &'static [u8] = include_bytes!("../../../../.rusk/keys/c6dd8004be10946f636217e623a8eb17f3d6a1fce609b9ace528938eb8739091.vd");
const VD_EXEC_2_1: &'static [u8] = include_bytes!("../../../../.rusk/keys/925858370cd6669b5e1acc4d273c4cc20d371e66691c8cfb3c7125cc345c4c0a.vd");
const VD_EXEC_2_2: &'static [u8] = include_bytes!("../../../../.rusk/keys/8d7bc723bc84ee98ae0f756e65dcf6ff195e057fd2475e3aea229f4f37d91693.vd");
const VD_EXEC_3_0: &'static [u8] = include_bytes!("../../../../.rusk/keys/61d84bf281d573f6f97dfdaa286b2a2ee2573a28130639f593719dccf8b35c7c.vd");
const VD_EXEC_3_1: &'static [u8] = include_bytes!("../../../../.rusk/keys/ecabf4a709a5429ee910bc9a64f5884b4afda82823a9ccec0452e40eb6964796.vd");
const VD_EXEC_3_2: &'static [u8] = include_bytes!("../../../../.rusk/keys/8664b8c5c82c6a16178874348b39441db5935e023ae6eb0b995216b262cb8375.vd");
const VD_EXEC_4_0: &'static [u8] = include_bytes!("../../../../.rusk/keys/4dffee2d24ed9488140ec0b5cf811b2d35e7d0e3b3245212b8bfd88bd9c8eeaf.vd");
const VD_EXEC_4_1: &'static [u8] = include_bytes!("../../../../.rusk/keys/f6d928c85be254137a6e60ae4e35022f0f5e4771c54d1d14ea38220c7968c450.vd");
const VD_EXEC_4_2: &'static [u8] = include_bytes!("../../../../.rusk/keys/bbc7a808769bd867ece4a8af005b8dfa61e22830c673d1e893994c097df2e1a3.vd");

const VD_STCT: &'static [u8] = include_bytes!("../../../../.rusk/keys/438ebc8572f9b5a072bed2647f2d36764e2a12dda56762624ffe42ab52e678f8.vd");
const VD_STCO: &'static [u8] = include_bytes!("../../../../.rusk/keys/86fe9e513cf12715b5b3f3c1205ac24d66a77cf1a71b8cd759bee2646282ecbb.vd");
const VD_WDFT: &'static [u8] = include_bytes!("../../../../.rusk/keys/5ee0c318587c95823bf101e871d1d0115bf33c192d295e987c928111fa070c66.vd");
const VD_WDFO: &'static [u8] = include_bytes!("../../../../.rusk/keys/d776aebf0ebd3367d51bc67c5f6341d5b070a84ba17ceab825c11b5fced8e460.vd");

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
