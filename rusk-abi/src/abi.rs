// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;

use dusk_bytes::Serializable;
use execution_core::{
    signatures::{
        bls::{
            MultisigPublicKey, MultisigSignature, PublicKey as BlsPublicKey,
            Signature as BlsSignature,
        },
        schnorr::{
            PublicKey as SchnorrPublicKey, Signature as SchnorrSignature,
        },
    },
    BlsScalar, ContractId,
};
use piecrust_uplink::{host_query, meta_data};

use crate::{Metadata, Query};

/// Compute the blake2b hash of the given bytes, returning the resulting scalar.
/// The output of the hasher is truncated (last nibble) to fit onto a scalar.
pub fn hash(bytes: Vec<u8>) -> BlsScalar {
    host_query(Query::HASH, bytes)
}

/// Compute the poseidon hash of the given scalars
pub fn poseidon_hash(scalars: Vec<BlsScalar>) -> BlsScalar {
    host_query(Query::POSEIDON_HASH, scalars)
}

/// Verify a proof is valid for a given circuit type and public inputs
pub fn verify_proof(
    verifier_data: Vec<u8>,
    proof: Vec<u8>,
    public_inputs: Vec<BlsScalar>,
) -> bool {
    host_query(Query::VERIFY_PROOF, (verifier_data, proof, public_inputs))
}

/// Verify a schnorr signature is valid for the given public key and message
pub fn verify_schnorr(
    msg: BlsScalar,
    pk: SchnorrPublicKey,
    sig: SchnorrSignature,
) -> bool {
    host_query(Query::VERIFY_SCHNORR, (msg, pk, sig))
}

/// Verify a BLS signature is valid for the given public key and message
pub fn verify_bls(msg: Vec<u8>, pk: BlsPublicKey, sig: BlsSignature) -> bool {
    host_query(Query::VERIFY_BLS, (msg, pk, sig))
}

/// Verify a BLS signature is valid for the given public key and message
pub fn verify_bls_multisig(
    msg: Vec<u8>,
    pk: MultisigPublicKey,
    sig: MultisigSignature,
) -> bool {
    host_query(Query::VERIFY_BLS_MULTISIG, (msg, pk, sig))
}

/// Get the chain ID.
pub fn chain_id() -> u8 {
    meta_data(Metadata::CHAIN_ID).unwrap()
}

/// Get the current block height.
pub fn block_height() -> u64 {
    meta_data(Metadata::BLOCK_HEIGHT).unwrap()
}

/// Query owner of a given contract.
/// Returns none if contract is not found.
/// Panics if owner is not a valid public key (should never happen).
pub fn owner(contract: ContractId) -> Option<BlsPublicKey> {
    owner_raw(contract).map(|buf| {
        BlsPublicKey::from_bytes(&buf)
            .expect("Owner should deserialize correctly")
    })
}

/// Query self owner of a given contract.
/// Panics if owner is not a valid public key (should never happen).
pub fn self_owner() -> BlsPublicKey {
    let buf = self_owner_raw();
    BlsPublicKey::from_bytes(&buf).expect("Owner should deserialize correctly")
}

/// Query raw "to_bytes" serialization of the owner of a given contract.
pub fn owner_raw(contract: ContractId) -> Option<[u8; BlsPublicKey::SIZE]> {
    piecrust_uplink::owner(contract)
}

/// Query raw "to_bytes" serialization of the self owner.
pub fn self_owner_raw() -> [u8; BlsPublicKey::SIZE] {
    piecrust_uplink::self_owner()
}
