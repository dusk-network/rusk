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
use dusk_bls12_381_sign::{
    PublicKey as BlsPublicKey, Signature as BlsSignature,
};
use dusk_pki::PublicKey;
use dusk_schnorr::Signature;
use rusk_abi::{ModuleId, PaymentInfo, PublicInput, State};

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
        verifier_data: Vec<u8>,
        proof: Vec<u8>,
        public_inputs: Vec<PublicInput>,
    ) -> bool {
        rusk_abi::verify_proof(verifier_data, proof, public_inputs)
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
        pk: BlsPublicKey,
        sig: BlsSignature,
    ) -> bool {
        rusk_abi::verify_bls(msg, pk, sig)
    }

    pub fn block_height(&self) -> u64 {
        rusk_abi::block_height()
    }
}

#[no_mangle]
unsafe fn hash(arg_len: u32) -> u32 {
    rusk_abi::wrap_query(arg_len, |scalars| STATE.hash(scalars))
}

#[no_mangle]
unsafe fn poseidon_hash(arg_len: u32) -> u32 {
    rusk_abi::wrap_query(arg_len, |scalars| STATE.poseidon_hash(scalars))
}

#[no_mangle]
unsafe fn verify_proof(arg_len: u32) -> u32 {
    rusk_abi::wrap_query(arg_len, |(verifier_data, proof, public_inputs)| {
        STATE.verify_proof(verifier_data, proof, public_inputs)
    })
}

#[no_mangle]
unsafe fn verify_schnorr(arg_len: u32) -> u32 {
    rusk_abi::wrap_query(arg_len, |(msg, pk, sig)| {
        STATE.verify_schnorr(msg, pk, sig)
    })
}

#[no_mangle]
unsafe fn verify_bls(arg_len: u32) -> u32 {
    rusk_abi::wrap_query(arg_len, |(msg, pk, sig)| {
        STATE.verify_bls(msg, pk, sig)
    })
}

#[no_mangle]
unsafe fn block_height(arg_len: u32) -> u32 {
    rusk_abi::wrap_query(arg_len, |_: ()| STATE.block_height())
}

const PAYMENT_INFO: PaymentInfo = PaymentInfo::Transparent(None);

#[no_mangle]
fn payment_info(arg_len: u32) -> u32 {
    rusk_abi::wrap_query(arg_len, |_: ()| PAYMENT_INFO)
}
