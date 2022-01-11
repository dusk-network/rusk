// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate alloc;

use crate::RuskModule;
use alloc::vec::Vec;
use dusk_abi::ContractId;
use dusk_abi::Module;
use dusk_bls12_381::BlsScalar;
use dusk_pki::PublicKey;
use dusk_schnorr::Signature;

use crate::{PaymentInfo, PublicInput, PAYMENT_INFO};

pub fn hash(bytes: Vec<u8>) -> BlsScalar {
    dusk_abi::query(&RuskModule::id(), &(RuskModule::HASH, bytes))
        .expect("query RuskModule for Hash should not fail")
}

pub fn poseidon_hash(scalars: Vec<BlsScalar>) -> BlsScalar {
    dusk_abi::query(&RuskModule::id(), &(RuskModule::POSEIDON_HASH, scalars))
        .expect("query RuskModule for Poseidon Hash should not fail")
}

pub fn verify_proof(
    proof: Vec<u8>,
    verifier_data: Vec<u8>,
    pi: Vec<PublicInput>,
) -> bool {
    dusk_abi::query(
        &RuskModule::id(),
        &(RuskModule::VERIFY_PROOF, proof, verifier_data, pi),
    )
    .expect("query RuskModule for verify a proof should not fail")
}

pub fn verify_schnorr_sign(
    sign: Signature,
    pk: PublicKey,
    message: BlsScalar,
) -> bool {
    dusk_abi::query(
        &RuskModule::id(),
        &(RuskModule::VERIFY_SCHNORR_SIGN, sign, pk, message),
    )
    .expect("query RuskModule for verifying schnorr signature should not fail")
}

/// FIXME: Until this is not moved to be part of Cake! we will reserve the 0
/// Query idx for this payable info.
pub fn payment_info(addr: ContractId) -> PaymentInfo {
    dusk_abi::query(&addr, &PAYMENT_INFO)
        .expect("Failed to retrieve payment info from the specified address")
}
