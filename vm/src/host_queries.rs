// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The host-queries registered on the Dusk VM

use alloc::vec::Vec;

use dusk_bytes::DeserializableSlice;
use dusk_core::groth16::bn254::{Bn254, G1Projective};
use dusk_core::groth16::serialize::CanonicalDeserialize;
use dusk_core::groth16::{
    Groth16, PreparedVerifyingKey, Proof as Groth16Proof,
};
use dusk_core::plonk::{Proof as PlonkProof, Verifier};
use dusk_core::signatures::bls::{
    MultisigPublicKey, MultisigSignature, PublicKey as BlsPublicKey,
    Signature as BlsSignature,
};
use dusk_core::signatures::schnorr::{
    PublicKey as SchnorrPublicKey, Signature as SchnorrSignature,
};
use dusk_core::BlsScalar;
use dusk_poseidon::{Domain, Hash as PoseidonHash};
use rkyv::ser::serializers::AllocSerializer;
use rkyv::{Archive, Deserialize, Serialize};

use crate::cache;

/// Computes a cryptographic hash of a byte vector.
///
/// This function uses the BLS12-381 scalar field to generate a deterministic
/// hash from the provided byte array. The result is a [`BlsScalar`], making it
/// suitable for cryptographic operations like zero-knowledge proofs and digital
/// signatures.
///
/// # Arguments
/// * `bytes` - A vector of bytes representing the input data to be hashed.
///
/// # Returns
/// A [`BlsScalar`] representing the cryptographic hash of the input bytes.
///
/// # References
/// For more details about BLS12-381 and its scalar operations, refer to:
/// <https://github.com/dusk-network/bls12_381>.
pub fn hash(bytes: Vec<u8>) -> BlsScalar {
    BlsScalar::hash_to_scalar(&bytes[..])
}

/// Computes the Poseidon hash of a vector of scalar values.
///
/// This function uses the Poseidon252 hashing algorithm to produce a
/// cryptographic hash. Poseidon is designed for efficiency in zk-SNARK
/// applications and operates over finite fields, making it well-suited for
/// blockchain and cryptographic use cases.
///
/// # Arguments
/// * `scalars` - A vector of [`BlsScalar`] values to be hashed. The input
///   values represent the data to be hashed into a single scalar output.
///
/// # Returns
/// A [`BlsScalar`] representing the Poseidon hash of the input values.
///
/// # References
/// For more details about Poseidon and its implementation, refer to:
/// <https://github.com/dusk-network/Poseidon252>.
pub fn poseidon_hash(scalars: Vec<BlsScalar>) -> BlsScalar {
    PoseidonHash::digest(Domain::Other, &scalars)[0]
}

/// Verifies a PLONK zero-knowledge proof.
///
/// This function verifies a proof generated by a PLONK proving system. It takes
/// in the verifier's key data, the proof itself, and the public inputs required
/// for verification. PLONK is a highly-efficient proof system used in
/// zk-SNARKs.
///
/// # Arguments
/// * `verifier_data` - A serialized representation of the verifier key.
/// * `proof` - A serialized representation of the proof to be verified.
/// * `public_inputs` - A vector of [`BlsScalar`] representing the public inputs
///   for the proof.
///
/// # Returns
/// A boolean indicating whether the proof is valid (`true`) or invalid
/// (`false`).
///
/// # References
/// <https://github.com/dusk-network/plonk>.
pub fn verify_plonk(
    verifier_data: Vec<u8>,
    proof: Vec<u8>,
    public_inputs: Vec<BlsScalar>,
) -> bool {
    let verifier = Verifier::try_from_bytes(verifier_data)
        .expect("Verifier data coming from the contract should be valid");
    let proof = PlonkProof::from_slice(&proof).expect("Proof should be valid");

    verifier.verify(&proof, &public_inputs[..]).is_ok()
}

/// Verifies a Groth16 zk-SNARK proof over the BN254 curve.
///
/// This function verifies a proof generated using the Groth16 proving system.
/// It takes in the prepared verifying key, the proof itself, and the public
/// inputs.
///
/// # Arguments
/// * `pvk` - A serialized representation of the prepared verifying key.
/// * `proof` - A serialized representation of the Groth16 proof.
/// * `inputs` - A serialized vector of public inputs for the proof.
///
/// # Returns
/// A boolean indicating whether the proof is valid (`true`) or invalid
/// (`false`).
///
/// # References
/// For more details about Groth16 and its implementation, refer to:
/// <https://docs.rs/ark-groth16/latest/ark_groth16/>.
pub fn verify_groth16_bn254(
    pvk: Vec<u8>,
    proof: Vec<u8>,
    inputs: Vec<u8>,
) -> bool {
    let pvk = PreparedVerifyingKey::deserialize_uncompressed(&pvk[..])
        .expect("verifying key must be valid");
    let proof = Groth16Proof::deserialize_compressed(&proof[..])
        .expect("proof must be valid");
    let inputs = G1Projective::deserialize_compressed(&inputs[..])
        .expect("inputs must be valid");

    Groth16::<Bn254>::verify_proof_with_prepared_inputs(&pvk, &proof, &inputs)
        .expect("verifying proof should succeed")
}

/// Verifies a Schnorr signature.
///
/// This function verifies a Schnorr signature using the Jubjub elliptic curve.
/// It takes in the message, the public key of the signer, and the signature to
/// verify the validity of the signature.
///
/// # Arguments
/// * `msg` - A [`BlsScalar`] representing the hashed message.
/// * `pk` - A [`SchnorrPublicKey`] representing the signer's public key.
/// * `sig` - A [`SchnorrSignature`] representing the signature to be verified.
///
/// # Returns
/// A boolean indicating whether the signature is valid (`true`) or invalid
/// (`false`).
///
/// # References
/// For more details about Schnorr signatures and their implementation, refer
/// to: <https://github.com/dusk-network/jubjub-schnorr>.
pub fn verify_schnorr(
    msg: BlsScalar,
    pk: SchnorrPublicKey,
    sig: SchnorrSignature,
) -> bool {
    pk.verify(&sig, msg).is_ok()
}

/// Verifies a BLS signature.
///
/// This function verifies a BLS signature using the BLS12-381 elliptic curve.
/// It takes in the message, the signer's public key, and the signature to
/// validate the integrity of the signed data.
///
/// # Arguments
/// * `msg` - A vector of bytes representing the original message.
/// * `pk` - A [`BlsPublicKey`] representing the signer's public key.
/// * `sig` - A [`BlsSignature`] representing the signature to be verified.
///
/// # Returns
/// A boolean indicating whether the signature is valid (`true`) or invalid
/// (`false`).
///
/// # References
/// For more details about BLS signatures and their implementation, refer to:
/// <https://github.com/dusk-network/bls12_381-bls>.
pub fn verify_bls(msg: Vec<u8>, pk: BlsPublicKey, sig: BlsSignature) -> bool {
    pk.verify(&sig, &msg).is_ok()
}

/// Verifies a BLS multi-signature.
///
/// This function verifies a multi-signature created using the BLS signature
/// scheme over the BLS12-381 elliptic curve. It validates the integrity of the
/// message signed by multiple participants.
///
/// # Arguments
/// * `msg` - A vector of bytes representing the original message.
/// * `keys` - A vector of [`BlsPublicKey`] instances representing the
///   participants' public keys.
/// * `sig` - A [`MultisigSignature`] representing the combined multi-signature
///   to be verified.
///
/// # Returns
/// A boolean indicating whether the multi-signature is valid (`true`) or
/// invalid (`false`).
///
/// # References
/// For more details about BLS multi-signatures and their implementation, refer
/// to: <https://github.com/dusk-network/bls12_381-bls>.
pub fn verify_bls_multisig(
    msg: Vec<u8>,
    keys: Vec<BlsPublicKey>,
    sig: MultisigSignature,
) -> bool {
    let len = keys.len();
    if len < 1 {
        panic!("must have at least one key");
    }

    let akey = MultisigPublicKey::aggregate(&keys)
        .expect("aggregation should succeed");

    akey.verify(&sig, &msg).is_ok()
}

fn wrap_host_query<A, R, F>(arg_buf: &mut [u8], arg_len: u32, closure: F) -> u32
where
    F: FnOnce(A) -> R,
    A: Archive,
    A::Archived: Deserialize<A, rkyv::Infallible>,
    R: Serialize<AllocSerializer<1024>>,
{
    let root =
        unsafe { rkyv::archived_root::<A>(&arg_buf[..arg_len as usize]) };
    let arg: A = root.deserialize(&mut rkyv::Infallible).unwrap();

    let result = closure(arg);

    let bytes = rkyv::to_bytes::<_, 1024>(&result).unwrap();

    arg_buf[..bytes.len()].copy_from_slice(&bytes);
    bytes.len() as u32
}

pub(crate) fn host_hash(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(arg_buf, arg_len, hash)
}

pub(crate) fn host_poseidon_hash(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(arg_buf, arg_len, poseidon_hash)
}

pub(crate) fn host_verify_plonk(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    let hash = *blake2b_simd::blake2b(&arg_buf[..arg_len as usize]).as_array();
    let cached = cache::get_plonk_verification(hash);

    wrap_host_query(arg_buf, arg_len, |(vd, proof, pis)| {
        let is_valid = cached.unwrap_or_else(|| verify_plonk(vd, proof, pis));
        cache::put_plonk_verification(hash, is_valid);
        is_valid
    })
}

pub(crate) fn host_verify_groth16_bn254(
    arg_buf: &mut [u8],
    arg_len: u32,
) -> u32 {
    let hash = *blake2b_simd::blake2b(&arg_buf[..arg_len as usize]).as_array();
    let cached = cache::get_groth16_verification(hash);

    wrap_host_query(arg_buf, arg_len, |(pvk, proof, inputs)| {
        let is_valid =
            cached.unwrap_or_else(|| verify_groth16_bn254(pvk, proof, inputs));
        cache::put_groth16_verification(hash, is_valid);
        is_valid
    })
}

pub(crate) fn host_verify_schnorr(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(arg_buf, arg_len, |(msg, pk, sig)| {
        verify_schnorr(msg, pk, sig)
    })
}

pub(crate) fn host_verify_bls(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    let hash = *blake2b_simd::blake2b(&arg_buf[..arg_len as usize]).as_array();
    let cached = cache::get_bls_verification(hash);

    wrap_host_query(arg_buf, arg_len, |(msg, pk, sig)| {
        let is_valid = cached.unwrap_or_else(|| verify_bls(msg, pk, sig));
        cache::put_bls_verification(hash, is_valid);
        is_valid
    })
}

pub(crate) fn host_verify_bls_multisig(
    arg_buf: &mut [u8],
    arg_len: u32,
) -> u32 {
    wrap_host_query(arg_buf, arg_len, |(msg, keys, sig)| {
        verify_bls_multisig(msg, keys, sig)
    })
}
