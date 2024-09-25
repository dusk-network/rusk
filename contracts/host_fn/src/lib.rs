// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]
#![feature(alloc_error_handler)]
#![deny(clippy::all)]

extern crate alloc;
use alloc::vec::Vec;

use dusk_bytes::Serializable;
use execution_core::{
    signatures::{
        bls::{PublicKey as BlsPublicKey, Signature as BlsSignature},
        schnorr::{
            PublicKey as SchnorrPublicKey, Signature as SchnorrSignature,
        },
    },
    BlsScalar,
};

static mut STATE: HostFnTest = HostFnTest;

#[derive(Clone, Debug, Default)]
pub struct HostFnTest;

impl HostFnTest {
    pub fn hash(&self, bytes: Vec<u8>) -> BlsScalar {
        rusk_abi::hash(bytes)
    }

    pub fn poseidon_hash(&self, scalars: Vec<BlsScalar>) -> BlsScalar {
        rusk_abi::poseidon_hash(scalars)
    }

    pub fn verify_plonk(
        &self,
        verifier_data: Vec<u8>,
        proof: Vec<u8>,
        public_inputs: Vec<BlsScalar>,
    ) -> bool {
        rusk_abi::verify_plonk(verifier_data, proof, public_inputs)
    }

    pub fn verify_groth16_bn254(
        &self,
        pvk: Vec<u8>,
        proof: Vec<u8>,
        inputs: Vec<u8>,
    ) -> bool {
        rusk_abi::verify_groth16_bn254(pvk, proof, inputs)
    }

    pub fn verify_schnorr(
        &self,
        msg: BlsScalar,
        pk: SchnorrPublicKey,
        sig: SchnorrSignature,
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

    pub fn chain_id(&self) -> u8 {
        rusk_abi::chain_id()
    }

    pub fn block_height(&self) -> u64 {
        rusk_abi::block_height()
    }

    pub fn owner(&self) -> BlsPublicKey {
        rusk_abi::self_owner()
    }

    pub fn owner_raw(&self) -> [u8; BlsPublicKey::SIZE] {
        rusk_abi::self_owner_raw()
    }
}

#[no_mangle]
unsafe fn hash(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |scalars| STATE.hash(scalars))
}

#[no_mangle]
unsafe fn poseidon_hash(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |scalars| STATE.poseidon_hash(scalars))
}

#[no_mangle]
unsafe fn verify_plonk(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(verifier_data, proof, public_inputs)| {
        STATE.verify_plonk(verifier_data, proof, public_inputs)
    })
}

#[no_mangle]
unsafe fn verify_groth16_bn254(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(pvk, proof, inputs)| {
        STATE.verify_groth16_bn254(pvk, proof, inputs)
    })
}

#[no_mangle]
unsafe fn verify_schnorr(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(msg, pk, sig)| {
        STATE.verify_schnorr(msg, pk, sig)
    })
}

#[no_mangle]
unsafe fn verify_bls(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(msg, pk, sig)| {
        STATE.verify_bls(msg, pk, sig)
    })
}

#[no_mangle]
unsafe fn chain_id(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| STATE.chain_id())
}

#[no_mangle]
unsafe fn block_height(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| STATE.block_height())
}

#[no_mangle]
unsafe fn contract_owner(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| STATE.owner())
}

#[no_mangle]
unsafe fn contract_owner_raw(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| STATE.owner_raw())
}
