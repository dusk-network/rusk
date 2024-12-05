// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;
use std::path::{Path, PathBuf};

use dusk_bytes::DeserializableSlice;
use dusk_poseidon::{Domain, Hash as PoseidonHash};
use execution_core::groth16::bn254::{Bn254, G1Projective};
use execution_core::groth16::serialize::CanonicalDeserialize;
use execution_core::groth16::{
    Groth16, PreparedVerifyingKey, Proof as Groth16Proof,
};
use execution_core::plonk::{Proof as PlonkProof, Verifier};
use execution_core::signatures::bls::{
    MultisigPublicKey, MultisigSignature, PublicKey as BlsPublicKey,
    Signature as BlsSignature,
};
use execution_core::signatures::schnorr::{
    PublicKey as SchnorrPublicKey, Signature as SchnorrSignature,
};
use execution_core::BlsScalar;
use piecrust::{Error as PiecrustError, Session, SessionData, VM};
use rkyv::ser::serializers::AllocSerializer;
use rkyv::{Archive, Deserialize, Serialize};

mod cache;

use crate::{Metadata, Query};

/// Create a new session based on the given `vm`. The vm *must* have been
/// created using [`new_vm`] or [`new_ephemeral_vm`].
pub fn new_session(
    vm: &VM,
    base: [u8; 32],
    chain_id: u8,
    block_height: u64,
) -> Result<Session, PiecrustError> {
    vm.session(
        SessionData::builder()
            .base(base)
            .insert(Metadata::CHAIN_ID, chain_id)?
            .insert(Metadata::BLOCK_HEIGHT, block_height)?,
    )
}

/// Create a new genesis session based on the given `vm`. The vm *must* have
/// been created using [`new_vm`] or [`new_ephemeral_vm`].
pub fn new_genesis_session(vm: &VM, chain_id: u8) -> Session {
    vm.session(
        SessionData::builder()
            .insert(Metadata::CHAIN_ID, chain_id)
            .expect("Inserting chain ID in metadata should succeed")
            .insert(Metadata::BLOCK_HEIGHT, 0)
            .expect("Inserting block height in metadata should succeed"),
    )
    .expect("Creating a genesis session should always succeed")
}

/// Create a new [`VM`] compliant with Dusk's specification.
pub fn new_vm<P: AsRef<Path> + Into<PathBuf>>(
    root_dir: P,
) -> Result<VM, PiecrustError> {
    let mut vm = VM::new(root_dir)?;
    register_host_queries(&mut vm);
    Ok(vm)
}

/// Creates a new [`VM`] with a temporary directory.
pub fn new_ephemeral_vm() -> Result<VM, PiecrustError> {
    let mut vm = VM::ephemeral()?;
    register_host_queries(&mut vm);
    Ok(vm)
}

fn register_host_queries(vm: &mut VM) {
    vm.register_host_query(Query::HASH, host_hash);
    vm.register_host_query(Query::POSEIDON_HASH, host_poseidon_hash);
    vm.register_host_query(Query::VERIFY_PLONK, host_verify_plonk);
    vm.register_host_query(
        Query::VERIFY_GROTH16_BN254,
        host_verify_groth16_bn254,
    );
    vm.register_host_query(Query::VERIFY_SCHNORR, host_verify_schnorr);
    vm.register_host_query(Query::VERIFY_BLS, host_verify_bls);
    vm.register_host_query(
        Query::VERIFY_BLS_MULTISIG,
        host_verify_bls_multisig,
    );
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

fn host_hash(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(arg_buf, arg_len, hash)
}

fn host_poseidon_hash(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(arg_buf, arg_len, poseidon_hash)
}

fn host_verify_plonk(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    let hash = *blake2b_simd::blake2b(&arg_buf[..arg_len as usize]).as_array();
    let cached = cache::get_plonk_verification(hash);

    wrap_host_query(arg_buf, arg_len, |(vd, proof, pis)| {
        let is_valid = cached.unwrap_or_else(|| verify_plonk(vd, proof, pis));
        cache::put_plonk_verification(hash, is_valid);
        is_valid
    })
}

fn host_verify_groth16_bn254(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    let hash = *blake2b_simd::blake2b(&arg_buf[..arg_len as usize]).as_array();
    let cached = cache::get_groth16_verification(hash);

    wrap_host_query(arg_buf, arg_len, |(pvk, proof, inputs)| {
        let is_valid =
            cached.unwrap_or_else(|| verify_groth16_bn254(pvk, proof, inputs));
        cache::put_groth16_verification(hash, is_valid);
        is_valid
    })
}

fn host_verify_schnorr(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(arg_buf, arg_len, |(msg, pk, sig)| {
        verify_schnorr(msg, pk, sig)
    })
}

fn host_verify_bls(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    let hash = *blake2b_simd::blake2b(&arg_buf[..arg_len as usize]).as_array();
    let cached = cache::get_bls_verification(hash);

    wrap_host_query(arg_buf, arg_len, |(msg, pk, sig)| {
        let is_valid = cached.unwrap_or_else(|| verify_bls(msg, pk, sig));
        cache::put_bls_verification(hash, is_valid);
        is_valid
    })
}

fn host_verify_bls_multisig(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(arg_buf, arg_len, |(msg, keys, sig)| {
        verify_bls_multisig(msg, keys, sig)
    })
}

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
