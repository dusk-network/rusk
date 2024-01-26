// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub use piecrust_uplink::*;

/// Compute the blake2b hash of the given bytes, returning the resulting scalar.
/// The output of the hasher is truncated (last nibble) to fit onto a scalar.
#[cfg(feature = "abi")]
pub fn hash(bytes: alloc::vec::Vec<u8>) -> dusk_bls12_381::BlsScalar {
    use crate::Query;
    host_query(Query::HASH, bytes)
}

/// Compute the poseidon hash of the given scalars
#[cfg(feature = "abi")]
pub fn poseidon_hash(
    scalars: alloc::vec::Vec<dusk_bls12_381::BlsScalar>,
) -> dusk_bls12_381::BlsScalar {
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
    msg: dusk_bls12_381::BlsScalar,
    pk: jubjub_schnorr::PublicKey,
    sig: jubjub_schnorr::Signature,
) -> bool {
    use crate::Query;
    host_query(Query::VERIFY_SCHNORR, (msg, pk, sig))
}

/// Verify a BLS signature is valid for the given public key and message
#[cfg(feature = "abi")]
pub fn verify_bls(
    msg: alloc::vec::Vec<u8>,
    pk: bls12_381_bls::PublicKey,
    sig: bls12_381_bls::Signature,
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
