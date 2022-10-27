// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]
#![feature(core_intrinsics, lang_items, alloc_error_handler)]
#![deny(clippy::all)]

extern crate alloc;
use alloc::vec::Vec;

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{Signature as BlsSignature, APK};
use dusk_pki::PublicKey;
use dusk_plonk::proof_system::Proof;
use dusk_schnorr::Signature;
use piecrust_uplink::{ModuleId, State};
use rusk_abi::{CircuitType, PublicInput};

#[no_mangle]
static SELF_ID: ModuleId = ModuleId::uninitialized();

static mut STATE: State<HostFnTest> = State::new(HostFnTest);

#[derive(Clone, Debug, Default)]
pub struct HostFnTest;

impl HostFnTest {
    pub fn hash(&self, bytes: Vec<u8>) -> BlsScalar {
        rusk_abi::hash(bytes)
    }

    pub fn poseidon_hash(&self, scalars: Vec<BlsScalar>) -> BlsScalar {
        rusk_abi::poseidon_hash(scalars)
    }

    pub fn verify_proof(
        &self,
        ty: CircuitType,
        proof: Proof,
        public_inputs: Vec<PublicInput>,
    ) -> bool {
        rusk_abi::verify_proof(ty, proof, public_inputs)
    }

    pub fn verify_schnorr(
        &self,
        msg: BlsScalar,
        pk: PublicKey,
        sig: Signature,
    ) -> bool {
        rusk_abi::verify_schnorr(msg, pk, sig)
    }

    pub fn verify_bls(
        &self,
        msg: Vec<u8>,
        apk: APK,
        sig: BlsSignature,
    ) -> bool {
        rusk_abi::verify_bls(msg, apk, sig)
    }
}

#[no_mangle]
unsafe fn hash(arg_len: u32) -> u32 {
    piecrust_uplink::wrap_query(arg_len, |scalars| STATE.hash(scalars))
}

#[no_mangle]
unsafe fn poseidon_hash(arg_len: u32) -> u32 {
    piecrust_uplink::wrap_query(arg_len, |scalars| STATE.poseidon_hash(scalars))
}

#[no_mangle]
unsafe fn verify_proof(arg_len: u32) -> u32 {
    piecrust_uplink::wrap_query(arg_len, |(ty, proof, public_inputs)| {
        STATE.verify_proof(ty, proof, public_inputs)
    })
}

#[no_mangle]
unsafe fn verify_schnorr(arg_len: u32) -> u32 {
    piecrust_uplink::wrap_query(arg_len, |(msg, pk, sig)| {
        STATE.verify_schnorr(msg, pk, sig)
    })
}

#[no_mangle]
unsafe fn verify_bls(arg_len: u32) -> u32 {
    piecrust_uplink::wrap_query(arg_len, |(msg, pk, sig)| {
        STATE.verify_bls(msg, pk, sig)
    })
}
