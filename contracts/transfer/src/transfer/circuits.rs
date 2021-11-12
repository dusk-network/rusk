// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::TransferContract;

use dusk_abi::ContractId;
use dusk_bls12_381::BlsScalar;
use phoenix_core::{Crossover, Message};

const VD_EXEC_1_0: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/cc6893178ef24743becdef0b4a076ccc1eb9d289dd2bc1ce781cd0886298ffa7.vd"));
const VD_EXEC_1_1: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/9102a739a4d9421e46d68f0f804ebe923618fa5219a6e6518301071b6d305682.vd"));
const VD_EXEC_1_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/10054c6f8c4db14adaa9e7781fab32035bb95b1403910b92f28f11fa32b3e730.vd"));
const VD_EXEC_2_0: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/fa741d95897d77fd5519c521bfbe4b4a015383b41302abd93a33e258bd4b8e63.vd"));
const VD_EXEC_2_1: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/66d564fa7e0c3539d20d30d09b58a179f462b82579c9ef021fa455b6e4048c30.vd"));
const VD_EXEC_2_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/e3111ac2549ed2c94a15959122e122be228e18ed99da0a192ce5af7bb1608a04.vd"));
const VD_EXEC_3_0: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/7c3baf0b6a6c5c18868f94152da5ac17038d7e6ddd4a51a29cd8535bcf53d975.vd"));
const VD_EXEC_3_1: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/74b1e904743aadc2d3521bb270e560a8a15132651f78e441bde8c1c9affc0d99.vd"));
const VD_EXEC_3_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/806435350ca58d106ebfc8ecd5d7ff28681408d033cfcf507e84dd348d72078d.vd"));
const VD_EXEC_4_0: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/d16fc8a82893cb5ff46c9ba209e8bb996d32a2f0f996d9a88b5d208f97b38872.vd"));
const VD_EXEC_4_1: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/6882ebd5829fad487422444655dcfcc74f8886b91b7e7919ed3cc6fc9d8bf927.vd"));
const VD_EXEC_4_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/4dd500add96f7a5f9a5ed3edf7532774d8c2679db357ec7ddce2723e4d12b798.vd"));

const VD_STCT: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/5e80819a6fb131d3eb9631d53cc31e17dcdf300d8f998ada58d972cba1bb666e.vd"));
const VD_STCO: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/119f07727cd7cb33aad7e0d53f22fdb398cbe7cb35dd0ad752e4d1fa5abf2799.vd"));
const VD_WDFT: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/ca00902797ac1d0cdf4d1b67e6959b2d79c4837c18bb60c49cffdfa5530f552a.vd"));
const VD_WDFO: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/00001fbf781109d2867196fc5afb9b687e1394a414cf2bb3510af2bc199b9033.vd"));

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
