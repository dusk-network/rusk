// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

const VD_EXEC_1_0: &'static [u8] = include_bytes!("../../target/verifier-keys/83485632a2eb89881f957ad43fe72632cb21c6027d62c25dc98da8b146f201fa.vk");
const VD_EXEC_1_1: &'static [u8] = include_bytes!("../../target/verifier-keys/3ecafbee6360c956416e8594403585101589745215e8643ab317154c2a219d65.vk");
const VD_EXEC_1_2: &'static [u8] = include_bytes!("../../target/verifier-keys/53e97ec348dd2f0d59731f8f1d305368c196c56ca9bbb58fa716929f56b1071b.vk");
const VD_EXEC_2_0: &'static [u8] = include_bytes!("../../target/verifier-keys/f03900bebbfde95b8c8218109efcf4801d02d8fb3e1d0b3f69c300ad1a857a1c.vk");
const VD_EXEC_2_1: &'static [u8] = include_bytes!("../../target/verifier-keys/04dc563f75b60af4ed1e036c2b2087dea9abfbc74f0ffbe8b0b684d3f02250d0.vk");
const VD_EXEC_2_2: &'static [u8] = include_bytes!("../../target/verifier-keys/d043f8554bb9fdb503971184c13d769952f4a49947b5676995371c8d73985987.vk");
const VD_EXEC_3_0: &'static [u8] = include_bytes!("../../target/verifier-keys/0a513eb3890c3d52428e311b162f0a576ed0cc12d879c5f296d637bb0ea5c8d5.vk");
const VD_EXEC_3_1: &'static [u8] = include_bytes!("../../target/verifier-keys/72acff34cd8d5e7c129508229a4ed9c08d3b5be4694cd4329481df63c67c1392.vk");
const VD_EXEC_3_2: &'static [u8] = include_bytes!("../../target/verifier-keys/f8126b228288a0b69f76d4e6764da2ae93973f8c40f9b5e26551de1a01817dac.vk");
const VD_EXEC_4_0: &'static [u8] = include_bytes!("../../target/verifier-keys/8b67f594e6bcdb1f153f6829d639d4cd3f6e6909e088048cd9205a28eca7ba4e.vk");
const VD_EXEC_4_1: &'static [u8] = include_bytes!("../../target/verifier-keys/622a22b81114da4f1546d52d5aabc1a61739f493143523a8f3f5c9ac409fceae.vk");
const VD_EXEC_4_2: &'static [u8] = include_bytes!("../../target/verifier-keys/500150fdb85ac0e8ed79dc69ec7f9649f0998589904ff3c3ac37c35b775d3985.vk");

const VD_STCO: &'static [u8] = include_bytes!("../../target/verifier-keys/36c7ebec4a6311dcdcfb5f6c4df22b0f5813c46e4efb5cf70ac3004a409cc17d.vk");
const VD_STCT: &'static [u8] = include_bytes!("../../target/verifier-keys/d61c0d1ce33ccb73fdb51248531cbfd2029041702d0235fd770192b1823e1fb0.vk");

const VD_WDFO: &'static [u8] = include_bytes!("../../target/verifier-keys/638e2314bceee160e275eb5de1df4c9342f380bd5ec9234265fcba8257849b60.vk");

pub const fn exec(inputs: usize, outputs: usize) -> &'static [u8] {
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

pub const fn stco() -> &'static [u8] {
    VD_STCO
}

pub const fn stct() -> &'static [u8] {
    VD_STCT
}

pub const fn wdfo() -> &'static [u8] {
    VD_WDFO
}
