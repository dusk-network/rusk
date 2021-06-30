// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

const VD_EXEC_1_0: &'static [u8] = include_bytes!("../../../.rusk/keys/267001e9ffa9eac5ec297295006c5ef7d5b16aff094f1fd9b0f4f9fbf5cca2ad.vk");
const VD_EXEC_1_1: &'static [u8] = include_bytes!("../../../.rusk/keys/566328b5846e4cde15a115540bedcdb5117e40f832c704e00bc52ef5ac7e000f.vk");
const VD_EXEC_1_2: &'static [u8] = include_bytes!("../../../.rusk/keys/b3508d50de9ae432c9456104cf45621f85c7c4093dfda3b62a66d0443a31bd1c.vk");
const VD_EXEC_2_0: &'static [u8] = include_bytes!("../../../.rusk/keys/328f3187b176fc708c32a7ceda9ab39d642014fb4bcecac47f1eea47c6da1ca8.vk");
const VD_EXEC_2_1: &'static [u8] = include_bytes!("../../../.rusk/keys/9b2bd8e71148d4591a291a455742a4c4da65e29df94aa3bff3a007ed83159301.vk");
const VD_EXEC_2_2: &'static [u8] = include_bytes!("../../../.rusk/keys/3547ff8a07e40b2df42dfe3f3e3146d1fcf6b7366cb903e7807e9e1030efd26d.vk");
const VD_EXEC_3_0: &'static [u8] = include_bytes!("../../../.rusk/keys/a111c8aa9cafd6632020bc6fe1428b62f7c8d93c2029ea74c7be84f26243aaff.vk");
const VD_EXEC_3_1: &'static [u8] = include_bytes!("../../../.rusk/keys/f21eb7328458c8a4d5e56931d8aa9280b167065380c362830892075a3b7202f6.vk");
const VD_EXEC_3_2: &'static [u8] = include_bytes!("../../../.rusk/keys/23e2538b15d9017f0ab21ee9a615a93c45762a858184fd91a5e10905ab63a7ef.vk");
const VD_EXEC_4_0: &'static [u8] = include_bytes!("../../../.rusk/keys/7df368bf1521fe47757205bb89f5b6492cbcdab20d508fe63b85034c9a6f7e9f.vk");
const VD_EXEC_4_1: &'static [u8] = include_bytes!("../../../.rusk/keys/7b68d8b3eb8ad2ce27519a3dfb406ea156622a784d672ab0661da0d077a39096.vk");
const VD_EXEC_4_2: &'static [u8] = include_bytes!("../../../.rusk/keys/1818f3d5b7dbf6956798f03de44129bfce7d9df79f1df6fe65072291400ba2df.vk");

const VD_STCO: &'static [u8] = include_bytes!("../../../.rusk/keys/2d69d68ef18e0b2571e04efb3687eb2af69f6a3e63a2e6ea90cfa178e5a99087.vk");
const VD_STCT: &'static [u8] = include_bytes!("../../../.rusk/keys/1e1a90992626bfc4a4e5e449f4f6da3c441ad6897605479e29e65c8884f1b7ed.vk");

const VD_WDFO: &'static [u8] = include_bytes!("../../../.rusk/keys/2cde3577266aac35b7ad805138ca38db86b3743739c8d88d2814eb213a3de876.vk");

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
