// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate alloc;

use alloc::vec::Vec;
use dusk_bls12_381::BlsScalar;
use dusk_pki::PublicKey;
use schnorr::Signature;

use canonical::{BridgeStore, Id32};
use dusk_abi::Module;

type BS = BridgeStore<Id32>;
type RuskModule = crate::RuskModule<BS>;

use crate::PublicInput;

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
