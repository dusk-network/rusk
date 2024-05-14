// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The functions contained in this module output the messages signed over for
//! each method of the contract.

use alloc::vec::Vec;

use dusk_bls12_381::BlsScalar;
use jubjub_schnorr::PublicKey as NotePublicKey;

const SCRATCH_SIZE: usize = 128;

pub type Transfer = (
    Option<NotePublicKey>, // from
    Option<NotePublicKey>, // to
    u64,                   // amount
    u64,                   // timestamp
);

pub fn transfer_msg(seed: BlsScalar, batch: &Vec<Transfer>) -> Vec<u8> {
    rkyv::to_bytes::<_, SCRATCH_SIZE>(&(0u8, seed, batch.clone()))
        .expect("Serializing should be infallible")
        .to_vec()
}

pub fn fee_msg(seed: BlsScalar, batch: &Vec<Transfer>) -> Vec<u8> {
    rkyv::to_bytes::<_, SCRATCH_SIZE>(&(1u8, seed, batch.clone()))
        .expect("Serializing should be infallible")
        .to_vec()
}

pub fn mint_msg(
    seed: BlsScalar,
    address: NotePublicKey,
    amount: u64,
) -> Vec<u8> {
    rkyv::to_bytes::<_, SCRATCH_SIZE>(&(0u8, seed, address, amount))
        .expect("Serializing should be infallible")
        .to_vec()
}

pub fn burn_msg(
    seed: BlsScalar,
    address: NotePublicKey,
    amount: u64,
) -> Vec<u8> {
    rkyv::to_bytes::<_, SCRATCH_SIZE>(&(1u8, seed, address, amount))
        .expect("Serializing should be infallible")
        .to_vec()
}

pub fn pause_msg(seed: BlsScalar) -> Vec<u8> {
    rkyv::to_bytes::<_, SCRATCH_SIZE>(&(0u8, seed))
        .expect("Serializing should be infallible")
        .to_vec()
}

pub fn unpause_msg(seed: BlsScalar) -> Vec<u8> {
    rkyv::to_bytes::<_, SCRATCH_SIZE>(&(1u8, seed))
        .expect("Serializing should be infallible")
        .to_vec()
}
