// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;

use wallet_core::keys::{
    derive_bls_sk, derive_multiple_phoenix_sk, derive_phoenix_pk,
    derive_phoenix_sk, derive_phoenix_vk,
};

const SEED: [u8; 64] = [0; 64];
const INDEX: u8 = 42;

#[test]
fn test_derive_phoenix_sk() {
    // it is important that we always derive the same key from a fixed seed
    let sk_bytes = [
        160, 210, 234, 8, 94, 23, 76, 60, 130, 143, 137, 225, 37, 83, 68, 218,
        207, 192, 171, 235, 252, 130, 133, 62, 18, 232, 6, 49, 245, 123, 220,
        12, 250, 111, 39, 88, 24, 41, 156, 174, 241, 14, 118, 173, 11, 53, 192,
        126, 7, 119, 70, 69, 212, 230, 124, 79, 223, 140, 93, 153, 33, 147,
        163, 0,
    ];
    assert_eq!(derive_phoenix_sk(&SEED, INDEX).to_bytes(), sk_bytes);
}

#[test]
fn test_derive_multiple_phoenix_sk() {
    // it is important that we always derive the same key from a fixed seed
    let sk_bytes_0 = [
        160, 210, 234, 8, 94, 23, 76, 60, 130, 143, 137, 225, 37, 83, 68, 218,
        207, 192, 171, 235, 252, 130, 133, 62, 18, 232, 6, 49, 245, 123, 220,
        12, 250, 111, 39, 88, 24, 41, 156, 174, 241, 14, 118, 173, 11, 53, 192,
        126, 7, 119, 70, 69, 212, 230, 124, 79, 223, 140, 93, 153, 33, 147,
        163, 0,
    ];
    let sk_bytes_1 = [
        0, 229, 97, 222, 152, 25, 163, 173, 84, 216, 211, 69, 205, 122, 63,
        227, 98, 234, 163, 66, 145, 71, 217, 221, 29, 78, 36, 77, 68, 29, 144,
        2, 235, 80, 237, 21, 95, 54, 16, 89, 74, 200, 124, 248, 119, 216, 38,
        167, 19, 129, 205, 138, 218, 57, 198, 4, 4, 202, 115, 62, 55, 213, 141,
        0,
    ];

    let keys = derive_multiple_phoenix_sk(&SEED, INDEX..INDEX + 2);
    assert_eq!(keys[0].to_bytes(), sk_bytes_0,);
    assert_eq!(keys[1].to_bytes(), sk_bytes_1,);
}

#[test]
fn test_derive_phoenix_pk() {
    // it is important that we always derive the same key from a fixed seed
    let pk_bytes = [
        59, 192, 170, 209, 99, 97, 60, 124, 218, 81, 61, 102, 25, 235, 14, 87,
        219, 234, 56, 102, 10, 111, 22, 189, 171, 101, 180, 168, 17, 70, 72,
        101, 135, 243, 55, 243, 138, 103, 185, 26, 196, 219, 84, 126, 33, 115,
        84, 60, 38, 41, 79, 104, 232, 222, 105, 2, 60, 185, 149, 50, 207, 43,
        89, 100,
    ];
    assert_eq!(derive_phoenix_pk(&SEED, INDEX).to_bytes(), pk_bytes);
}

#[test]
fn test_derive_phoenix_vk() {
    // it is important that we always derive the same key from a fixed seed
    let vk_bytes = [
        160, 210, 234, 8, 94, 23, 76, 60, 130, 143, 137, 225, 37, 83, 68, 218,
        207, 192, 171, 235, 252, 130, 133, 62, 18, 232, 6, 49, 245, 123, 220,
        12, 135, 243, 55, 243, 138, 103, 185, 26, 196, 219, 84, 126, 33, 115,
        84, 60, 38, 41, 79, 104, 232, 222, 105, 2, 60, 185, 149, 50, 207, 43,
        89, 100,
    ];
    assert_eq!(derive_phoenix_vk(&SEED, INDEX).to_bytes(), vk_bytes);
}

#[test]
fn test_derive_bls_sk() {
    // it is important that we always derive the same key from a fixed seed
    let sk_bytes = [
        130, 180, 24, 224, 131, 143, 97, 18, 120, 53, 37, 39, 251, 44, 121,
        168, 4, 248, 29, 176, 142, 136, 224, 188, 159, 246, 73, 6, 112, 174, 6,
        7,
    ];
    assert_eq!(derive_bls_sk(&SEED, INDEX).to_bytes(), sk_bytes);
}
