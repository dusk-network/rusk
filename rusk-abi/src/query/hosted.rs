// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::query::*;

use alloc::vec::Vec;

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{
    PublicKey as BlsPublicKey, Signature as BlsSignature,
};
use dusk_pki::{PublicKey, PublicSpendKey};
use dusk_schnorr::Signature;
use piecrust_uplink::{ContractError, ContractId};

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

/// Enum representing all possible payment configurations.
#[derive(Debug, Clone, Copy, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
#[repr(C)]
pub enum PaymentInfo {
    /// Only transparent notes are accepted.
    Transparent(Option<PublicSpendKey>),
    /// Only obfuscated notes are accepted.
    Obfuscated(Option<PublicSpendKey>),
    /// Any type of note is accepted.
    Any(Option<PublicSpendKey>),
}

/// Compute the blake2b hash of the given bytes, returning the resulting scalar.
/// The output of the hasher is truncated (last nibble) to fit onto a scalar.
pub fn hash(bytes: Vec<u8>) -> BlsScalar {
    piecrust_uplink::host_query(Query::HASH, bytes)
}

/// Compute the poseidon hash of the given scalars
pub fn poseidon_hash(scalars: Vec<BlsScalar>) -> BlsScalar {
    piecrust_uplink::host_query(Query::POSEIDON_HASH, scalars)
}

/// Verify a proof is valid for a given circuit type and public inputs
pub fn verify_proof(
    verifier_data: Vec<u8>,
    proof: Vec<u8>,
    public_inputs: Vec<PublicInput>,
) -> bool {
    piecrust_uplink::host_query(
        Query::VERIFY_PROOF,
        (verifier_data, proof, public_inputs),
    )
}

/// Verify a schnorr signature is valid for the given public key and message
pub fn verify_schnorr(msg: BlsScalar, pk: PublicKey, sig: Signature) -> bool {
    piecrust_uplink::host_query(Query::VERIFY_SCHNORR, (msg, pk, sig))
}

/// Verify a BLS signature is valid for the given public key and message
pub fn verify_bls(msg: Vec<u8>, pk: BlsPublicKey, sig: BlsSignature) -> bool {
    piecrust_uplink::host_query(Query::VERIFY_BLS, (msg, pk, sig))
}

/// Get the current block height.
pub fn block_height() -> u64 {
    piecrust_uplink::meta_data(Metadata::BLOCK_HEIGHT).unwrap()
}

/// Query a contract for the types of payment it accepts.
pub fn payment_info(
    contract: ContractId,
) -> Result<PaymentInfo, ContractError> {
    piecrust_uplink::call(contract, "payment_info", &())
}
