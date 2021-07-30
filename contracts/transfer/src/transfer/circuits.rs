// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::TransferContract;

use dusk_abi::ContractId;
use dusk_bls12_381::BlsScalar;
use phoenix_core::{Crossover, Message};

const VD_EXEC_1_0: &[u8] = include_bytes!("../../../../.rusk/keys/9d93a12bc51068173d276171e5d22aac6592d3646a78935f34794bdfbc4e9a51.vd");
const VD_EXEC_1_1: &[u8] = include_bytes!("../../../../.rusk/keys/25a68c05c6e96cee7d778c8968987b07c2f930a7f7ac2ae1468272a34eb62533.vd");
const VD_EXEC_1_2: &[u8] = include_bytes!("../../../../.rusk/keys/beaa9b7fdcc8f96597c02a302f1fb72514ab175c79ff0af7109445b41823d266.vd");
const VD_EXEC_2_0: &[u8] = include_bytes!("../../../../.rusk/keys/09127ee0fdaebb7098340dc6644be5ce3d50fd29b228cadab644bae714fd56a9.vd");
const VD_EXEC_2_1: &[u8] = include_bytes!("../../../../.rusk/keys/6a9714ce6781aaffa30d5244d7c0b83a761e7227cacc8120c3a0ae8032e44c8a.vd");
const VD_EXEC_2_2: &[u8] = include_bytes!("../../../../.rusk/keys/a20529bcceda3c111a03bca599dc37cde121734bf7b22a6a679e07284d3c5180.vd");
const VD_EXEC_3_0: &[u8] = include_bytes!("../../../../.rusk/keys/2af2c1219b8a081a3c19a3ab9985c5e66b9ade17ece0e8cfb133fcdb91179f2d.vd");
const VD_EXEC_3_1: &[u8] = include_bytes!("../../../../.rusk/keys/4f18cd9566fc8120111cc0f030068cd89fd45153e1b5bdfadac9ec94eeeaf5c3.vd");
const VD_EXEC_3_2: &[u8] = include_bytes!("../../../../.rusk/keys/31bad9476a4bc46b16ec857dc71379b951cf037d0cb5b70d650dc289be94c867.vd");
const VD_EXEC_4_0: &[u8] = include_bytes!("../../../../.rusk/keys/00e587333191c698d5b67d4798e0d1949882189841251ddf114d6d344132ed7f.vd");
const VD_EXEC_4_1: &[u8] = include_bytes!("../../../../.rusk/keys/97e94bc228fd0c63a209ba592a8de736561c1c88d4428945b078524c9229772d.vd");
const VD_EXEC_4_2: &[u8] = include_bytes!("../../../../.rusk/keys/98c54b36b7a25eb7ae653c178a15a26e56685edd7b2a60f0efe6cdcd885d21ab.vd");

const VD_STCT: &[u8] = include_bytes!("../../../../.rusk/keys/48c75d25c5d533bae041e13d923cfbdd508a7b59c9f95a2d6e0288fa180dc621.vd");
const VD_STCO: &[u8] = include_bytes!("../../../../.rusk/keys/79a55ad4c35352b9833843619258855bc280026636ea93433a8ed8fbf9447362.vd");
const VD_WDFT: &[u8] = include_bytes!("../../../../.rusk/keys/e8971b2fee918afb2eb3b4ce109a31cca90101b3faf54b978945b047c298d674.vd");
const VD_WDFO: &[u8] = include_bytes!("../../../../.rusk/keys/65ca2ad3de723ca909fe2777c2b7cdf7b4ad0a0974c52519bc430d811485888d.vd");

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
