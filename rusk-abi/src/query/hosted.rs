// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate alloc;

use crate::{CircuitType, MetadataType};

use alloc::vec::Vec;
use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{Signature as BlsSignature, APK};
use dusk_pki::PublicKey;
use dusk_plonk::prelude::Proof;
use dusk_schnorr::Signature;

use crate::{PublicInput, QueryType};

/// Compute the blake2b hash of the given bytes, returning the resulting scalar.
/// The output of the hasher is truncated (last nibble) to fit onto a scalar.
pub fn hash(bytes: Vec<u8>) -> BlsScalar {
    let str = QueryType::Hash.as_str();
    piecrust_uplink::host_query(str, bytes)
}

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
    piecrust_uplink::host_query(str, (ty, proof, public_inputs))
}

/// Verify a schnorr signature is valid for the given public key and message
pub fn verify_schnorr(msg: BlsScalar, pk: PublicKey, sig: Signature) -> bool {
    let str = QueryType::VerifySchnorr.as_str();
    piecrust_uplink::host_query(str, (msg, pk, sig))
}

/// Verify a BLS signature is valid for the given public key and message
pub fn verify_bls(msg: Vec<u8>, apk: APK, sig: BlsSignature) -> bool {
    let str = QueryType::VerifyBls.as_str();
    piecrust_uplink::host_query(str, (msg, apk, sig))
}

/// Get the current block height.
pub fn block_height() -> u64 {
    let str = MetadataType::BlockHeight.as_str();
    piecrust_uplink::host_data(str)
}
