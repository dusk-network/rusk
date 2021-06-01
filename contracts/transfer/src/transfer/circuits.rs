// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::TransferContract;

use dusk_bls12_381::BlsScalar;
use phoenix_core::{Crossover, Message};

const VD_EXEC_1_0: &'static [u8] = include_bytes!("../../target/verifier-data/4d8856febb142846ca4fd53fac02b68c94a496349bb32164942b701330fd919d.vd");
const VD_EXEC_1_1: &'static [u8] = include_bytes!("../../target/verifier-data/3f10d21902ec0a62be3115ce783c50aecd495f1a6451c31d5e2394e588bb72a9.vd");
const VD_EXEC_1_2: &'static [u8] = include_bytes!("../../target/verifier-data/1772c30c88303a0870ff153e1b7d75ec13891a27a9a22482dbb48ed9031383f8.vd");
const VD_EXEC_2_0: &'static [u8] = include_bytes!("../../target/verifier-data/f5a298124eee8960a8548e305269d1537dec38ef36d6a5a2a3a697a736f771cb.vd");
const VD_EXEC_2_1: &'static [u8] = include_bytes!("../../target/verifier-data/b0f8e8c199bb054deaeb56ce55fd029660a482d71150affa3fd42e9f2eafeafb.vd");
const VD_EXEC_2_2: &'static [u8] = include_bytes!("../../target/verifier-data/ba668d3b462ac7bdf0ae52fdecb536d524fd9f165802e38e92a6a7c150fd711a.vd");
const VD_EXEC_3_0: &'static [u8] = include_bytes!("../../target/verifier-data/e60c65af8089d5554fc58a641f22597161932fdbdc2bb4e887d73743da2c8e97.vd");
const VD_EXEC_3_1: &'static [u8] = include_bytes!("../../target/verifier-data/fff75501261223a3c142b01d740802900c2b9b3d1fd5922757d3903442b612e6.vd");
const VD_EXEC_3_2: &'static [u8] = include_bytes!("../../target/verifier-data/a8443ba09fb72112671a9656000fa3f13e4f2d1311722a1f7264a98e069dcd51.vd");
const VD_EXEC_4_0: &'static [u8] = include_bytes!("../../target/verifier-data/ebee91e2d7ed06691b7993034b1aa9b537ed184d85fb99068cb37bfc0c9314df.vd");
const VD_EXEC_4_1: &'static [u8] = include_bytes!("../../target/verifier-data/e14a93ac1e76d78b21da4b59747c023fa7368fdb5a4a23e58390efc91b7ffb32.vd");
const VD_EXEC_4_2: &'static [u8] = include_bytes!("../../target/verifier-data/8c77a850f1bf52b2b369409d4d462051e1f0fdeefd4e5fcf1cfe949a85185bec.vd");

const VD_STCO: &'static [u8] = include_bytes!("../../target/verifier-data/ba5945c7eacae75ac4fa46a982e8e47de6dfc9b23ff2a3dae77a43b222605e75.vd");
const VD_STCT: &'static [u8] = include_bytes!("../../target/verifier-data/e207aed29089b74f34977908bc3774c025d2d5c5e04d21341e9d7d0c3fe73a81.vd");
const VD_WDFT: &'static [u8] = include_bytes!("../../target/verifier-data/27e50bdeaa558008de848e495b1735f7f881eeee0ffb773f47990683c1b7f4cd.vd");
const VD_WDFO: &'static [u8] = include_bytes!("../../target/verifier-data/a38b7e89221f90b2e66201d9211c711f0b35284edfe69322342e84db7517bf16.vd");

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
