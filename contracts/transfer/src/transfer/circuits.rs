// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::TransferContract;

use dusk_abi::ContractId;
use dusk_bls12_381::BlsScalar;
use phoenix_core::{Crossover, Message};

const VD_EXEC_1_0: &[u8] = include_bytes!("../../../../.rusk/keys/4bd17e342c5ab38a25b310fd2cef4f5ebb5710c6730e0c9cdaa375505029ab0e.vd");
const VD_EXEC_1_1: &[u8] = include_bytes!("../../../../.rusk/keys/b939217c0c58fbf773725485b950b894b069d5de8f0882a40e0546da5543d368.vd");
const VD_EXEC_1_2: &[u8] = include_bytes!("../../../../.rusk/keys/1aecec0fe06face95c3d1a265154a5cdb58a0cf1a5fdc6821521fe9fb7af4cff.vd");
const VD_EXEC_2_0: &[u8] = include_bytes!("../../../../.rusk/keys/a9da7b086adde6d24a45f630f7a8e21f4a21ad5be17df904b74f2a4226c8ce4b.vd");
const VD_EXEC_2_1: &[u8] = include_bytes!("../../../../.rusk/keys/5340f601a87b218a7e5314c1ccd2915311d6cb89afe9be5c9264e4d0f719a233.vd");
const VD_EXEC_2_2: &[u8] = include_bytes!("../../../../.rusk/keys/bce7835a908897406354490fb09842a21f44810e71c7236c4a113a7518bb7d21.vd");
const VD_EXEC_3_0: &[u8] = include_bytes!("../../../../.rusk/keys/f930be5554517cda7d6b1fece1cf756026a206ff9bc3c5f4263f45f1ce4d04a6.vd");
const VD_EXEC_3_1: &[u8] = include_bytes!("../../../../.rusk/keys/dbad4c2e7386eb64db5592c6083fc37df05e3459221e1d2a0567c17c5d9a9bcc.vd");
const VD_EXEC_3_2: &[u8] = include_bytes!("../../../../.rusk/keys/44c744e87c8c2932c10d5725fc7995a40a911d3192f54fb16cf204aa9d277bfa.vd");
const VD_EXEC_4_0: &[u8] = include_bytes!("../../../../.rusk/keys/6b9f8ff9fd36ff7aaa194d20e9f4eb5037d5b0b62cecec184f91fa0038c7847d.vd");
const VD_EXEC_4_1: &[u8] = include_bytes!("../../../../.rusk/keys/cd97067ce0842b13a18cc102615409f3148122886575ec620677d26982c49205.vd");
const VD_EXEC_4_2: &[u8] = include_bytes!("../../../../.rusk/keys/ce2f573dbba2ba0545008d18270e4e3ec2ca2458860f73c774f1ed9918027430.vd");

const VD_STCT: &[u8] = include_bytes!("../../../../.rusk/keys/86de5d6eec0ed8219d95f5978c830db6339d0daf7a27bbc976fc9535b3437cde.vd");
const VD_STCO: &[u8] = include_bytes!("../../../../.rusk/keys/cafb5352cc3b9878fb33c5a66257f8074cc5f0eff8ad7aeee09f6a7a04a41ad2.vd");
const VD_WDFT: &[u8] = include_bytes!("../../../../.rusk/keys/c6237ceeda92d9040d072ea27f0d8ad900db601dd6363940facec806937f69e3.vd");
const VD_WDFO: &[u8] = include_bytes!("../../../../.rusk/keys/6f0acf746efa8ec93eacd46701cf89485db8b0c4d8320989ea5038d1614eacdb.vd");

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
