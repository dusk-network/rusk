// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate alloc;

use crate::CircuitType;

use alloc::vec::Vec;
use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{
    PublicKey as BlsPublicKey, Signature as BlsSignature,
};
use dusk_pki::PublicKey;
use dusk_plonk::prelude::Proof;
use dusk_schnorr::Signature;

use crate::PublicInput;

pub use crate::QueryType;

/// Compute the poseidon hash of the given scalars
pub fn poseidon_hash(scalars: Vec<BlsScalar>) -> BlsScalar {
    let str = QueryType::PoseidonHash.as_str();
    piecrust_uplink::host_query(str, scalars)
}

/// Verify a proof is valid for a given circuit type and public inputs
pub fn verify_proof(
    ty: CircuitType,
    proof: Proof,
    public_inputs: Vec<PublicInput>,
) -> bool {
    let str = QueryType::VerifyProof.as_str();
    piecrust_uplink::host_query(str, (ty, public_inputs, proof))
}

/// Verify a schnorr signature is valid for the given public key and message
pub fn verify_schnorr(msg: BlsScalar, pk: PublicKey, sig: Signature) -> bool {
    let str = QueryType::VerifySchnorr.as_str();
    piecrust_uplink::host_query(str, (msg, pk, sig))
}

/// Verify a BLS signature is valid for the given public key and message
pub fn verify_bls(msg: Vec<u8>, pk: BlsPublicKey, sig: BlsSignature) -> bool {
    let str = QueryType::VerifyBls.as_str();
    piecrust_uplink::host_query(str, (msg, pk, sig))
}
