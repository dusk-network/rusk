// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use dusk_wallet_core::keys::{
    derive_bls_sk, derive_multiple_phoenix_sk, derive_phoenix_pk,
    derive_phoenix_sk, derive_phoenix_vk,
};

const SEED: [u8; 64] = [0; 64];
const INDEX: u8 = 42;

#[test]
fn test_derive_phoenix_sk() {
    // it is important that we always derive the same key from a fixed seed
    let sk_bytes = [
        12, 16, 72, 188, 33, 76, 44, 178, 86, 123, 107, 153, 230, 149, 238,
        131, 87, 30, 94, 88, 52, 129, 247, 167, 30, 167, 163, 246, 68, 254, 14,
        9, 218, 135, 245, 104, 11, 190, 143, 129, 83, 202, 64, 179, 157, 248,
        175, 120, 157, 220, 98, 211, 141, 50, 224, 8, 1, 125, 29, 180, 206,
        195, 34, 0,
    ];
    assert_eq!(derive_phoenix_sk(&SEED, INDEX).to_bytes(), sk_bytes);
}

#[test]
fn test_derive_multiple_phoenix_sk() {
    // it is important that we always derive the same key from a fixed seed
    let sk_bytes_0 = [
        12, 16, 72, 188, 33, 76, 44, 178, 86, 123, 107, 153, 230, 149, 238,
        131, 87, 30, 94, 88, 52, 129, 247, 167, 30, 167, 163, 246, 68, 254, 14,
        9, 218, 135, 245, 104, 11, 190, 143, 129, 83, 202, 64, 179, 157, 248,
        175, 120, 157, 220, 98, 211, 141, 50, 224, 8, 1, 125, 29, 180, 206,
        195, 34, 0,
    ];
    let sk_bytes_1 = [
        185, 163, 40, 99, 29, 14, 67, 189, 145, 215, 252, 61, 146, 211, 135,
        55, 80, 69, 220, 183, 145, 4, 252, 186, 244, 79, 124, 177, 227, 35,
        209, 5, 100, 181, 254, 1, 111, 180, 155, 211, 140, 23, 252, 248, 103,
        44, 132, 14, 19, 18, 204, 101, 4, 200, 125, 185, 143, 68, 157, 251,
        129, 238, 137, 5,
    ];

    let keys = derive_multiple_phoenix_sk(&SEED, INDEX..INDEX + 2);
    assert_eq!(keys[0].to_bytes(), sk_bytes_0,);
    assert_eq!(keys[1].to_bytes(), sk_bytes_1,);
}

#[test]
fn test_derive_phoenix_pk() {
    // it is important that we always derive the same key from a fixed seed
    let pk_bytes = [
        51, 204, 45, 112, 212, 44, 118, 183, 148, 176, 254, 135, 253, 117, 230,
        62, 177, 139, 2, 57, 21, 150, 41, 86, 118, 239, 75, 194, 148, 129, 225,
        38, 132, 140, 106, 77, 181, 217, 196, 50, 135, 177, 158, 153, 43, 147,
        159, 217, 0, 160, 89, 95, 67, 160, 42, 74, 19, 1, 221, 216, 126, 204,
        206, 209,
    ];
    assert_eq!(derive_phoenix_pk(&SEED, INDEX).to_bytes(), pk_bytes);
}

#[test]
fn test_derive_phoenix_vk() {
    // it is important that we always derive the same key from a fixed seed
    let vk_bytes = [
        12, 16, 72, 188, 33, 76, 44, 178, 86, 123, 107, 153, 230, 149, 238,
        131, 87, 30, 94, 88, 52, 129, 247, 167, 30, 167, 163, 246, 68, 254, 14,
        9, 132, 140, 106, 77, 181, 217, 196, 50, 135, 177, 158, 153, 43, 147,
        159, 217, 0, 160, 89, 95, 67, 160, 42, 74, 19, 1, 221, 216, 126, 204,
        206, 209,
    ];
    assert_eq!(derive_phoenix_vk(&SEED, INDEX).to_bytes(), vk_bytes);
}

#[test]
fn test_derive_bls_sk() {
    // it is important that we always derive the same key from a fixed seed
    let sk_bytes = [
        95, 35, 167, 191, 106, 171, 71, 158, 159, 39, 84, 1, 132, 238, 152,
        235, 154, 5, 250, 158, 255, 195, 79, 95, 193, 58, 36, 189, 0, 99, 230,
        86,
    ];
    assert_eq!(derive_bls_sk(&SEED, INDEX).to_bytes(), sk_bytes);
}
