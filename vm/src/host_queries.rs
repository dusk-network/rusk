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

/// Compute the blake2b hash of the given scalars, returning the resulting
/// scalar. The hash is computed in such a way that it will always return a
/// valid scalar.
pub fn hash(bytes: Vec<u8>) -> BlsScalar {
    BlsScalar::hash_to_scalar(&bytes[..])
}

/// Compute the poseidon hash of the given scalars
pub fn poseidon_hash(scalars: Vec<BlsScalar>) -> BlsScalar {
    PoseidonHash::digest(Domain::Other, &scalars)[0]
}

/// Verify a Plonk proof is valid for a given circuit type and public inputs
///
/// # Panics
/// This will panic if `verifier_data` or `proof` are not valid.
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

/// Verify that a Groth16 proof in the BN254 pairing is valid for a given
/// circuit and inputs.
///
/// `proof` and `inputs` should be in compressed form, while `pvk` uncompressed.
///
/// # Panics
/// This will panic if `pvk`, `proof` or `inputs` are not valid.
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

/// Verify a schnorr signature is valid for the given public key and message
pub fn verify_schnorr(
    msg: BlsScalar,
    pk: SchnorrPublicKey,
    sig: SchnorrSignature,
) -> bool {
    pk.verify(&sig, msg).is_ok()
}

/// Verify a BLS signature is valid for the given public key and message
pub fn verify_bls(msg: Vec<u8>, pk: BlsPublicKey, sig: BlsSignature) -> bool {
    pk.verify(&sig, &msg).is_ok()
}

/// Verify a BLS signature is valid for the given public key and message
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
