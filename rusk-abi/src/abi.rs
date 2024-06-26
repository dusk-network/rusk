// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[cfg(feature = "abi")]
use dusk_bytes::Serializable;
#[cfg(feature = "abi")]
use execution_core::{
    BlsPublicKey, BlsScalar, BlsSignature, PublicKey, SchnorrPublicKey,
    SchnorrSignature,
};

pub use piecrust_uplink::*;

/// Compute the blake2b hash of the given bytes, returning the resulting scalar.
/// The output of the hasher is truncated (last nibble) to fit onto a scalar.
#[cfg(feature = "abi")]
pub fn hash(bytes: alloc::vec::Vec<u8>) -> BlsScalar {
    use crate::Query;
    host_query(Query::HASH, bytes)
}

/// Compute the poseidon hash of the given scalars
#[cfg(feature = "abi")]
pub fn poseidon_hash(scalars: alloc::vec::Vec<BlsScalar>) -> BlsScalar {
    use crate::Query;
    host_query(Query::POSEIDON_HASH, scalars)
}

/// Verify a proof is valid for a given circuit type and public inputs
#[cfg(feature = "abi")]
pub fn verify_proof(
    verifier_data: alloc::vec::Vec<u8>,
    proof: alloc::vec::Vec<u8>,
    public_inputs: alloc::vec::Vec<crate::PublicInput>,
) -> bool {
    use crate::Query;
    host_query(Query::VERIFY_PROOF, (verifier_data, proof, public_inputs))
}

/// Verify a schnorr signature is valid for the given public key and message
#[cfg(feature = "abi")]
pub fn verify_schnorr(
    msg: BlsScalar,
    pk: SchnorrPublicKey,
    sig: SchnorrSignature,
) -> bool {
    use crate::Query;
    host_query(Query::VERIFY_SCHNORR, (msg, pk, sig))
}

/// Verify a BLS signature is valid for the given public key and message
#[cfg(feature = "abi")]
pub fn verify_bls(
    msg: alloc::vec::Vec<u8>,
    pk: BlsPublicKey,
    sig: BlsSignature,
) -> bool {
    use crate::Query;
    host_query(Query::VERIFY_BLS, (msg, pk, sig))
}

/// Get the current block height.
#[cfg(feature = "abi")]
pub fn block_height() -> u64 {
    use crate::Metadata;
    meta_data(Metadata::BLOCK_HEIGHT).unwrap()
}

/// Query a contract for the types of payment it accepts.
#[cfg(feature = "abi")]
pub fn payment_info(
    contract: ContractId,
) -> Result<crate::PaymentInfo, ContractError> {
    call(contract, "payment_info", &())
}

/// Query owner of a given contract.
/// Returns none if contract is not found.
/// Panics if owner is not a valid public key (should never happen).
#[cfg(feature = "abi")]
pub fn owner(contract: ContractId) -> Option<PublicKey> {
    owner_raw(contract).map(|buf| {
        PublicKey::from_bytes(&buf).expect("Owner should deserialize correctly")
    })
}

/// Query self owner of a given contract.
/// Panics if owner is not a valid public key (should never happen).
#[cfg(feature = "abi")]
pub fn self_owner() -> PublicKey {
    let buf = self_owner_raw();
    PublicKey::from_bytes(&buf).expect("Owner should deserialize correctly")
}

/// Query raw "to_bytes" serialization of the owner of a given contract.
#[cfg(feature = "abi")]
pub fn owner_raw(contract: ContractId) -> Option<[u8; PublicKey::SIZE]> {
    piecrust_uplink::owner(contract)
}

/// Query raw "to_bytes" serialization of the self owner.
#[cfg(feature = "abi")]
pub fn self_owner_raw() -> [u8; PublicKey::SIZE] {
    piecrust_uplink::self_owner()
}
