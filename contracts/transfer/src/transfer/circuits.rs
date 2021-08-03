// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::TransferContract;

use dusk_abi::ContractId;
use dusk_bls12_381::BlsScalar;
use phoenix_core::{Crossover, Message};

const VD_EXEC_1_0: &[u8] = include_bytes!("../../../../.rusk/keys/1dd92c4b30c968882ef63d83bf4ec577cf3ce6ec18b3daa473806d68126b86f4.vd");
const VD_EXEC_1_1: &[u8] = include_bytes!("../../../../.rusk/keys/4f0a73fbf69a21736afc329204a7238cd36961446393e7d738d783e9ef3fd077.vd");
const VD_EXEC_1_2: &[u8] = include_bytes!("../../../../.rusk/keys/f93fc65b6411af4e04da721ca7278042a310bf6aa1974391b151aafa816fdd7d.vd");
const VD_EXEC_2_0: &[u8] = include_bytes!("../../../../.rusk/keys/029875010dd2c8b0b0fecf3529ddcd7f19b46b86316a81bcabd563c4763753f8.vd");
const VD_EXEC_2_1: &[u8] = include_bytes!("../../../../.rusk/keys/61389b94d7d67e11a97cbac1250b22d4e980be93f501ce6e6c7e525edcf014a3.vd");
const VD_EXEC_2_2: &[u8] = include_bytes!("../../../../.rusk/keys/11fe639e90369589f9e99fb2409dd2ad35393e219c93ca5c643cf49841067033.vd");
const VD_EXEC_3_0: &[u8] = include_bytes!("../../../../.rusk/keys/bd0495e848bdbdab87b93063bdff48170d3e4eba744a46754ec3f2fedc423614.vd");
const VD_EXEC_3_1: &[u8] = include_bytes!("../../../../.rusk/keys/4994c3360968bcb2ad6fd38077890132fc14e1aa3476518828b5d5ea2cc44400.vd");
const VD_EXEC_3_2: &[u8] = include_bytes!("../../../../.rusk/keys/7a86f4a1fa4ed53d9037ad4aa05d089d9ed241ff544b6fa3bbf638f8ab0e0fbf.vd");
const VD_EXEC_4_0: &[u8] = include_bytes!("../../../../.rusk/keys/f165ca779bb8986dff655c458a236dbf8eafd9aca91a07a20f012a8f7ec095cf.vd");
const VD_EXEC_4_1: &[u8] = include_bytes!("../../../../.rusk/keys/558477120c2acc7de591570f6852142cc422392e6b94b72fd53effdec8da5f74.vd");
const VD_EXEC_4_2: &[u8] = include_bytes!("../../../../.rusk/keys/6628b0b3a7c07ae4cd4c0af41482f088f44632812515c21b33ca8bb66036f681.vd");

const VD_STCT: &[u8] = include_bytes!("../../../../.rusk/keys/5c495cfe9175ccc797e3aa0e805c2cc693f9f0a345dc17cd36ace38576439758.vd");
const VD_STCO: &[u8] = include_bytes!("../../../../.rusk/keys/933d3ead2d0f9e6d4f1a7ca07b7efb4d6416bae88a9737dfd8e34f631bdf14a6.vd");
const VD_WDFT: &[u8] = include_bytes!("../../../../.rusk/keys/0056c0675668ba8650703a02293cfeeca3a5f14665fd8e74dc2a87aca559fa47.vd");
const VD_WDFO: &[u8] = include_bytes!("../../../../.rusk/keys/d2677e4075ecc36fbfa1fc35e57ad7825f5644a5a3b470200f87320086141174.vd");

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
