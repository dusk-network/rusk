// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::TransferContract;

use dusk_bls12_381::BlsScalar;
use phoenix_core::{Crossover, Message};

const VD_EXEC_1_0: &'static [u8] = include_bytes!("../../target/verifier-data/ac821c81b8538e330616a83cba9af1cc7d8296201c648398d0028d0f2519f727.vd");
const VD_EXEC_1_1: &'static [u8] = include_bytes!("../../target/verifier-data/98157019c0d11441ab9a6a36e29d9178983fffa32aba593b61727ef46025f840.vd");
const VD_EXEC_1_2: &'static [u8] = include_bytes!("../../target/verifier-data/144557c8ee689d544220eaf61700f6e406b9673e6223e47a808e65415d6c27d4.vd");
const VD_EXEC_2_0: &'static [u8] = include_bytes!("../../target/verifier-data/37446b4c3acebcf27472ef87522ee36c7bfffce33a6447a9368c51d41b3cb3d1.vd");
const VD_EXEC_2_1: &'static [u8] = include_bytes!("../../target/verifier-data/e1954a07e78b5cb0f82b5e9ccf48790843cf82e7f6bc6d89557f50aa1ba11897.vd");
const VD_EXEC_2_2: &'static [u8] = include_bytes!("../../target/verifier-data/e56ca2455aa786c4b867c314a9892e505f0c6c3978e7235dd1193eedbfb34e7b.vd");
const VD_EXEC_3_0: &'static [u8] = include_bytes!("../../target/verifier-data/c74c1dbe923c3b6cfdeef6bb981136094aa4127cea5df0715fcc6888c3db5f13.vd");
const VD_EXEC_3_1: &'static [u8] = include_bytes!("../../target/verifier-data/8894d295a4261f048c2e529f6b781721c663581e02eac496e18031d079d20e50.vd");
const VD_EXEC_3_2: &'static [u8] = include_bytes!("../../target/verifier-data/d5993ad86816d6091a5499306108624c5b5a54cd2f88302e49cca311fbf4a6bd.vd");
const VD_EXEC_4_0: &'static [u8] = include_bytes!("../../target/verifier-data/d41c3efa33b98bd3d35cb990341ff8193059eaeacd9e8b87087814426e71520a.vd");
const VD_EXEC_4_1: &'static [u8] = include_bytes!("../../target/verifier-data/38a329a415077293671f4d867fe569fb62806a5d3747d2e152c458fd682d0322.vd");
const VD_EXEC_4_2: &'static [u8] = include_bytes!("../../target/verifier-data/625b011854b088a2f4b4b41eb72c40f18d584e7cb26a8d8ec1af47b635f4ca81.vd");

const VD_STCO: &'static [u8] = include_bytes!("../../target/verifier-data/37fad9011b7b9109728cdc2863bc045fbcafed4375c661e4c0bb1367333d99ae.vd");
const VD_STCT: &'static [u8] = include_bytes!("../../target/verifier-data/f54d85616b97ec86874f7dac0bcd7dbeb9dbd5113f703a043d9ccd0ab7da1fee.vd");
const VD_WDFT: &'static [u8] = include_bytes!("../../target/verifier-data/174423117055c3dddc823ab47f04d7853bb825ff249f65335be0cd958a8528ff.vd");
const VD_WDFO: &'static [u8] = include_bytes!("../../target/verifier-data/303b12fe2a84e6e6a10cbad17d3d50679b873b3eb81792d07bf3b9ae9f370692.vd");

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
        address: &BlsScalar,
    ) -> BlsScalar {
        let mut m = crossover.to_hash_inputs().to_vec();

        m.push(value.into());
        m.push(*address);

        #[cfg(not(target_arch = "wasm32"))]
        let message = dusk_poseidon::sponge::hash(m.as_slice());

        #[cfg(target_arch = "wasm32")]
        let message = rusk_abi::poseidon_hash(m);

        message
    }

    pub fn sign_message_stco(
        crossover: &Crossover,
        message: &Message,
        address: &BlsScalar,
    ) -> BlsScalar {
        let mut m = crossover.to_hash_inputs().to_vec();

        m.extend(&message.to_hash_inputs());
        m.push(*address);

        #[cfg(not(target_arch = "wasm32"))]
        let message = dusk_poseidon::sponge::hash(m.as_slice());

        #[cfg(target_arch = "wasm32")]
        let message = rusk_abi::poseidon_hash(m);

        message
    }
}
