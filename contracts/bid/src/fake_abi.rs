// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! This is just a placeholder for the host functions until they're implemented in
//! rusk-ABI.
//! See: https://github.com/dusk-network/rusk-abi/issues/2

#![allow(unused_variables)]
use dusk_pki::PublicKey;
use schnorr::Signature;
use dusk_bls12_381::BlsScalar;
use alloc::vec::Vec;

// Verify a PLONK proof given the Proof, VerifierKey and PublicInputs
pub(crate) fn verify_proof(
    proof: Vec<u8>,
    vk: Vec<u8>,
    label: Vec<u8>,
    pub_inp: Vec<u8>,
) -> bool {
    true
}

pub(crate) fn verify_schnorr_sig(
    pk: PublicKey,
    sig: Signature,
    msg: BlsScalar,
) -> bool {true}
