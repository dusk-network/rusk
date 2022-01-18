// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::TransferContract;

use dusk_abi::ContractId;
use dusk_bls12_381::BlsScalar;
use phoenix_core::{Crossover, Message};

const VD_STCT: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/8ac166a2a72b674cabbcd733d930c6338e06b94e140b73590a1bbf0e7b545084.vd"));
const VD_STCO: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/6f87c4432dd230e97f7271e9c8b963fef1d366cc17d86af02ee63fd6449f5cad.vd"));
const VD_WDFT: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/5a9af9f6134a25fde542ffc013815cefa1837a886edf49ae31f4f3e6d7a64347.vd"));
const VD_WDFO: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/9dc9477f10b5e02634063b2cfaf8eae313fec301bd9d2d5bc885eee87e96cfe2.vd"));

const VD_EXEC_1_0: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/4a00c40792f14a79c2c8e46f01b6902f72e46a8a496b5adf56015af32747a17d.vd"));
const VD_EXEC_1_1: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/89421c761c054e16ab6d5147defd4bd13a11bc59ca9446d998a69482243b6222.vd"));
const VD_EXEC_1_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/a51761659226b8ae0a4b9c7278855dc73e77f1f61f1d9c2f8b886d90774ea015.vd"));
const VD_EXEC_2_0: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/61ba64f55e2051959038e36ed295241483960fd656183ce9e17a4ddd4192378a.vd"));
const VD_EXEC_2_1: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/2d9a2b1efcb1bad8a8e2df65caa7d4f8976c7bf30d056e725755bda325b0af53.vd"));
const VD_EXEC_2_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/9d183609e7e9759b9b416b6befd30137f02ebf697798f57ca31072d38bb1e6a5.vd"));
const VD_EXEC_3_0: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/5c9eff4453df8a2a56bf2793c3f91ddc683c14e6b85e17d9260412fc7e713fb9.vd"));
const VD_EXEC_3_1: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/1dd42b3ad7e5b36a30130d4df07667a34a338688a0fd5c3abb9c5ddb11e5bdb3.vd"));
const VD_EXEC_3_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/f7d0f3cb8f0cdf01e6b8a32950c6988c927664d734f061cd52e0a262c63dce60.vd"));
const VD_EXEC_4_0: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/5d7fce62a298d18a91661cc867cc92db49c765720ff1562c51ca107a2e4e4e5c.vd"));
const VD_EXEC_4_1: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/ba02dd4b9a0eb8a48b1322ef18f7016468465c8748a14e711fc64f781b3e10b4.vd"));
const VD_EXEC_4_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/b37ed1bd9e1cd45e775b5b5a89887a01b1cba2bf74db29ea88557d5082eb79a0.vd"));

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
