// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The Application Binary Interface (ABI) for the Dusk network.

pub use piecrust_uplink::{
    ContractError, ContractId, Event, StandardBufSerializer, ARGBUF_LEN,
    CONTRACT_ID_BYTES,
};

#[cfg(feature = "abi")]
pub use self::host_queries::*;

/// Enum storing the metadata identifiers.
pub enum Metadata {}

impl Metadata {
    /// The chain-id of the network.
    pub const CHAIN_ID: &'static str = "chain_id";
    /// The current block-height.
    pub const BLOCK_HEIGHT: &'static str = "block_height";
}

/// Enum storing the available host-queries.
pub enum Query {}

impl Query {
    /// Host-function name to compute the hash of some input-data.
    pub const HASH: &'static str = "hash";
    /// Host-function name to compute the poseidon-hash of some input-data.
    pub const POSEIDON_HASH: &'static str = "poseidon_hash";
    /// Host-function name to verify a plonk-proof.
    pub const VERIFY_PLONK: &'static str = "verify_plonk";
    /// Host-function name to verify a groth16-bn254 proof.
    pub const VERIFY_GROTH16_BN254: &'static str = "verify_groth16_bn254";
    /// Host-function name to verify a schnorr-signature.
    pub const VERIFY_SCHNORR: &'static str = "verify_schnorr";
    /// Host-function name to verify a bls-signature.
    pub const VERIFY_BLS: &'static str = "verify_bls";
    /// Host-function name to verify a bls-multisig.
    pub const VERIFY_BLS_MULTISIG: &'static str = "verify_bls_multisig";
}

#[cfg(feature = "abi")]
pub(crate) mod host_queries {
    #[cfg(feature = "abi-debug")]
    pub use piecrust_uplink::debug as piecrust_debug;
    pub use piecrust_uplink::{
        call, call_raw, call_raw_with_limit, call_with_limit, caller,
        callstack, emit, emit_raw, feed, limit, self_id, spent, wrap_call,
        wrap_call_unchecked, /* maybe use for our Transaction in
                              * spend_and_execute */
    };

    use alloc::vec::Vec;

    use dusk_bytes::Serializable;
    use piecrust_uplink::{host_query, meta_data};

    use crate::abi::{ContractId, Metadata, Query};
    use crate::signatures::bls::{
        MultisigSignature, PublicKey as BlsPublicKey, Signature as BlsSignature,
    };
    use crate::signatures::schnorr::{
        PublicKey as SchnorrPublicKey, Signature as SchnorrSignature,
    };
    use crate::BlsScalar;

    /// Compute the blake2b hash of the given bytes, returning the resulting
    /// scalar. The output of the hasher is truncated (last nibble) to fit
    /// onto a scalar.
    #[must_use]
    pub fn hash(bytes: Vec<u8>) -> BlsScalar {
        host_query(Query::HASH, bytes)
    }

    /// Compute the poseidon hash of the given scalars
    #[must_use]
    pub fn poseidon_hash(scalars: Vec<BlsScalar>) -> BlsScalar {
        host_query(Query::POSEIDON_HASH, scalars)
    }

    /// Verify that a Groth16 proof in the BN254 pairing is valid for a given
    /// circuit and inputs.
    ///
    /// `proof` and `inputs` should be in compressed form, while `pvk`
    /// uncompressed.
    #[must_use]
    pub fn verify_groth16_bn254(
        pvk: Vec<u8>,
        proof: Vec<u8>,
        inputs: Vec<u8>,
    ) -> bool {
        host_query(Query::VERIFY_GROTH16_BN254, (pvk, proof, inputs))
    }

    /// Verify a Plonk proof is valid for a given circuit type and public inputs
    #[must_use]
    pub fn verify_plonk(
        verifier_data: Vec<u8>,
        proof: Vec<u8>,
        public_inputs: Vec<BlsScalar>,
    ) -> bool {
        host_query(Query::VERIFY_PLONK, (verifier_data, proof, public_inputs))
    }

    /// Verify a schnorr signature is valid for the given public key and message
    #[must_use]
    pub fn verify_schnorr(
        msg: BlsScalar,
        pk: SchnorrPublicKey,
        sig: SchnorrSignature,
    ) -> bool {
        host_query(Query::VERIFY_SCHNORR, (msg, pk, sig))
    }

    /// Verify a BLS signature is valid for the given public key and message
    #[must_use]
    pub fn verify_bls(
        msg: Vec<u8>,
        pk: BlsPublicKey,
        sig: BlsSignature,
    ) -> bool {
        host_query(Query::VERIFY_BLS, (msg, pk, sig))
    }

    /// Verify a BLS signature is valid for the given public key and message
    #[must_use]
    pub fn verify_bls_multisig(
        msg: Vec<u8>,
        keys: Vec<BlsPublicKey>,
        sig: MultisigSignature,
    ) -> bool {
        host_query(Query::VERIFY_BLS_MULTISIG, (msg, keys, sig))
    }

    /// Get the chain ID.
    ///
    /// # Panics
    /// Panics if the chain doesn't store a `u8` `CHAIN_ID` in the metadata.
    #[must_use]
    pub fn chain_id() -> u8 {
        meta_data(Metadata::CHAIN_ID).unwrap()
    }

    /// Get the current block height.
    ///
    /// # Panics
    /// Panics if the chain doesn't store a `u64` `BLOCK_HEIGHT` in the
    /// metadata.
    #[must_use]
    pub fn block_height() -> u64 {
        meta_data(Metadata::BLOCK_HEIGHT).unwrap()
    }

    /// Query owner of a given contract.
    /// Returns none if contract is not found.
    ///
    /// # Panics
    /// Panics if owner is not a valid public key (should never happen).
    #[must_use]
    pub fn owner(contract: ContractId) -> Option<BlsPublicKey> {
        owner_raw(contract).map(|buf| {
            BlsPublicKey::from_bytes(&buf)
                .expect("Owner should deserialize correctly")
        })
    }

    /// Query self owner of a given contract.
    ///
    /// # Panics
    /// Panics if owner is not a valid public key (should never happen).
    #[must_use]
    pub fn self_owner() -> BlsPublicKey {
        let buf = self_owner_raw();
        BlsPublicKey::from_bytes(&buf)
            .expect("Owner should deserialize correctly")
    }

    /// Query raw `to_bytes` serialization of the owner of a given contract.
    #[must_use]
    pub fn owner_raw(contract: ContractId) -> Option<[u8; BlsPublicKey::SIZE]> {
        piecrust_uplink::owner(contract)
    }

    /// Query raw `to_bytes` serialization of the self owner.
    #[must_use]
    pub fn self_owner_raw() -> [u8; BlsPublicKey::SIZE] {
        piecrust_uplink::self_owner()
    }
}
