// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The host-queries registered on the Dusk VM

use alloc::vec::Vec;

use core::cell::Cell;

use bytecheck::CheckBytes;
use c_kzg::{Bytes32 as KzgBytes32, Bytes48};
use dusk_bytes::DeserializableSlice;
use dusk_core::BlsScalar;
use dusk_core::groth16::bn254::{Bn254, G1Projective};
use dusk_core::groth16::serialize::CanonicalDeserialize;
use dusk_core::groth16::{
    Groth16, PreparedVerifyingKey, Proof as Groth16Proof,
};
use dusk_core::plonk::{PlonkVersion, Proof as PlonkProof, Verifier};
use dusk_core::signatures::bls::{
    self as bls, BlsVersion, MultisigSignature, PublicKey as BlsPublicKey,
    Signature as BlsSignature,
};
use dusk_core::signatures::schnorr::{
    PublicKey as SchnorrPublicKey, Signature as SchnorrSignature,
};
use dusk_core::transfer::data::BlobData;
use dusk_poseidon::{Domain, Hash as PoseidonHash};
use rkyv::ser::serializers::AllocSerializer;
use rkyv::validation::validators::DefaultValidator;
use rkyv::{Archive, Deserialize, Serialize};
use secp256k1::{Message, Secp256k1, ecdsa::RecoverableSignature};
use sha2::{Digest as Sha2Digest, Sha256};
use sha3::Keccak256;
use tracing::warn;

use crate::cache;

thread_local! {
    // Default to V2 for safety: if the node forgets to set a version for a
    // consensus-critical call path, we'd rather reject than accept.
    static PLONK_VERSION: Cell<PlonkVersion> = const { Cell::new(PlonkVersion::V2) };
    static HARD_FORK: Cell<HardFork> = const { Cell::new(HardFork::PreFork) };
}

/// Active hardfork context for host-query rule selection.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum HardFork {
    /// Behavior before any explicit hardfork activation.
    PreFork,
    /// Behavior after Aegis activation.
    Aegis,
}

impl HardFork {
    /// Returns the BLS signature version for this hardfork.
    pub fn bls_version(&self) -> BlsVersion {
        match self {
            HardFork::Aegis => BlsVersion::V2,
            HardFork::PreFork => BlsVersion::V1,
        }
    }
}

/// Guard that restores the previous PLONK version when dropped.
#[derive(Debug)]
pub struct PlonkVersionGuard {
    prev: PlonkVersion,
}

impl Drop for PlonkVersionGuard {
    fn drop(&mut self) {
        PLONK_VERSION.with(|m| m.set(self.prev));
    }
}

/// Returns the current thread's PLONK version (defaults to `V2`).
pub fn plonk_version() -> PlonkVersion {
    PLONK_VERSION.with(|m| m.get())
}

/// Sets the current thread's PLONK version.
///
/// The previous version is restored when the returned guard is dropped.
pub fn set_plonk_version(version: PlonkVersion) -> PlonkVersionGuard {
    let prev = PLONK_VERSION.with(|m| {
        let prev = m.get();
        m.set(version);
        prev
    });
    PlonkVersionGuard { prev }
}

/// Guard that restores the previous hardfork when dropped.
#[derive(Debug)]
pub struct HardForkGuard {
    prev: HardFork,
}

impl Drop for HardForkGuard {
    fn drop(&mut self) {
        HARD_FORK.with(|m| m.set(self.prev));
    }
}

/// Returns the active hardfork for this thread.
pub fn hard_fork() -> HardFork {
    HARD_FORK.with(|m| m.get())
}

/// Sets the active hardfork for this thread.
///
/// The previous value is restored when the returned guard is dropped.
pub fn set_hard_fork(hard_fork: HardFork) -> HardForkGuard {
    let prev = HARD_FORK.with(|m| {
        let prev = m.get();
        m.set(hard_fork);
        prev
    });
    HardForkGuard { prev }
}

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
/// If argument deserialization fails, returns `BlsScalar::default()`.
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
/// If argument deserialization fails, returns `BlsScalar::default()`.
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
/// * `version` - The PLONK version to use for verification.
/// * `verifier_data` - A serialized representation of the verifier key.
/// * `proof` - A serialized representation of the proof to be verified.
/// * `public_inputs` - A vector of [`BlsScalar`] representing the public inputs
///   for the proof.
///
/// # Returns
/// A boolean indicating whether the proof is valid (`true`) or invalid
/// (`false`). If argument deserialization fails, returns `false`.
///
/// # References
/// <https://github.com/dusk-network/plonk>.
pub fn verify_plonk_with_version(
    version: PlonkVersion,
    verifier_data: Vec<u8>,
    proof: Vec<u8>,
    public_inputs: Vec<BlsScalar>,
) -> bool {
    // Deserialize verifier key
    let verifier = match Verifier::try_from_bytes(verifier_data) {
        Ok(v) => v,
        Err(e) => {
            warn!("vm: couldn't deserialize plonk verifier: {e:?}");
            return false;
        }
    };

    // Deserialize proof
    let proof = match PlonkProof::from_slice(&proof) {
        Ok(p) => p,
        Err(e) => {
            warn!("vm: couldn't deserialize plonk proof: {e:?}");
            return false;
        }
    };

    // Verify and return boolean result (map errors to false)
    let result =
        verifier.verify_with_version(&proof, &public_inputs[..], version);
    match result {
        Ok(_) => true,
        Err(e) => {
            warn!("vm: plonk verification failed ({version:?}): {e:?}");
            false
        }
    }
}

fn plonk_cache_key(
    version: PlonkVersion,
    arg_buf: &[u8],
) -> [u8; blake2b_simd::OUTBYTES] {
    // Domain-separate the cache key by PLONK version.
    let mut state = blake2b_simd::Params::new()
        .hash_length(blake2b_simd::OUTBYTES)
        .to_state();
    let cache_tag = match version {
        PlonkVersion::V1 => 0,
        PlonkVersion::V2 => 1,
        PlonkVersion::V3 => 2,
        _ => u8::MAX,
    };
    state.update(&[cache_tag]);
    state.update(arg_buf);
    *state.finalize().as_array()
}

fn bls_cache_key(
    hard_fork: HardFork,
    arg_buf: &[u8],
) -> [u8; blake2b_simd::OUTBYTES] {
    // Domain-separate the cache key by active hardfork rule set.
    let mut state = blake2b_simd::Params::new()
        .hash_length(blake2b_simd::OUTBYTES)
        .to_state();
    let cache_tag = match hard_fork {
        HardFork::PreFork => 0u8,
        HardFork::Aegis => 1u8,
    };
    state.update(&[cache_tag]);
    state.update(arg_buf);
    *state.finalize().as_array()
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
/// (`false`). If argument deserialization fails, returns `false`.
///
/// # References
/// For more details about Groth16 and its implementation, refer to:
/// <https://docs.rs/ark-groth16/latest/ark_groth16/>.
pub fn verify_groth16_bn254(
    pvk: Vec<u8>,
    proof: Vec<u8>,
    inputs: Vec<u8>,
) -> bool {
    let pvk = match PreparedVerifyingKey::deserialize_uncompressed(&pvk[..]) {
        Ok(v) => v,
        Err(e) => {
            warn!("vm: couldn't deserialize groth16 verifiying key: {e}");
            return false;
        }
    };

    let proof = match Groth16Proof::deserialize_compressed(&proof[..]) {
        Ok(p) => p,
        Err(e) => {
            warn!("vm: couldn't deserialize groth16 proof: {e}");
            return false;
        }
    };

    let inputs = match G1Projective::deserialize_compressed(&inputs[..]) {
        Ok(i) => i,
        Err(e) => {
            warn!("vm: couldn't deserialize groth16 inputs: {e}");
            return false;
        }
    };

    match Groth16::<Bn254>::verify_proof_with_prepared_inputs(
        &pvk, &proof, &inputs,
    ) {
        Ok(valid) => valid,
        Err(e) => {
            warn!("vm: couldn't verify groth16: {e}");
            false
        }
    }
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
/// (`false`). If argument deserialization fails, returns `false`.
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
/// (`false`). If argument deserialization fails, returns `false`.
///
/// # References
/// For more details about BLS signatures and their implementation, refer to:
/// <https://github.com/dusk-network/bls12_381-bls>.
pub fn verify_bls(msg: Vec<u8>, pk: BlsPublicKey, sig: BlsSignature) -> bool {
    bls::verify(&pk, &sig, &msg, hard_fork().bls_version()).is_ok()
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
/// invalid (`false`). If argument deserialization fails, returns `false`.
///
/// # References
/// For more details about BLS multi-signatures and their implementation, refer
/// to: <https://github.com/dusk-network/bls12_381-bls>.
pub fn verify_bls_multisig(
    msg: Vec<u8>,
    keys: Vec<BlsPublicKey>,
    sig: MultisigSignature,
) -> bool {
    if keys.is_empty() {
        warn!("vm: bls multisig verification requires at least one key");
        return false;
    }

    let bls_version = hard_fork().bls_version();
    let akey = match bls::aggregate(&keys, bls_version) {
        Ok(k) => k,
        Err(e) => {
            warn!("vm: couldn't aggregate bls public-keys due to {e}");
            return false;
        }
    };
    bls::verify_multisig(&akey, &sig, &msg, bls_version).is_ok()
}

/// Computes keccak256 hash of a byte vector.
///
/// # Arguments
/// * `bytes` - A vector of bytes representing the input data to be hashed.
///
/// # Returns
/// An array (`[u8; 32]`) representing the keccak256 hash.
/// If argument deserialization fails, returns `[0u8; 32]`.
pub fn keccak256(bytes: Vec<u8>) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(bytes.as_slice());
    hasher.finalize().into()
}

/// Computes sha256 hash of a byte vector.
///
/// # Arguments
/// * `bytes` - A vector of bytes representing the input data to be hashed.
///
/// # Returns
/// An array (`[u8; 32]`) representing the sha256 hash.
/// If argument deserialization fails, returns `[0u8; 32]`.
pub fn sha256(bytes: Vec<u8>) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes.as_slice());
    hasher.finalize().into()
}

/// Verifies a KZG point-evaluation proof.
///
/// # Arguments
/// * `commitment` - The 48-byte KZG commitment.
/// * `z` - The evaluation point.
/// * `y` - The expected evaluation.
/// * `proof` - The 48-byte KZG proof.
///
/// # Returns
/// `true` if the proof is valid, `false` otherwise.
/// If argument deserialization fails, returns `false`.
pub fn verify_kzg_proof(
    commitment: [u8; 48],
    z: [u8; 32],
    y: [u8; 32],
    proof: [u8; 48],
) -> bool {
    let settings = BlobData::eth_kzg_settings(None);
    let commitment = Bytes48::new(commitment);
    let z = KzgBytes32::new(z);
    let y = KzgBytes32::new(y);
    let proof = Bytes48::new(proof);
    match settings.verify_kzg_proof(&commitment, &z, &y, &proof) {
        Ok(valid) => valid,
        Err(e) => {
            warn!("vm: kzg proof verification failed: {e}");
            false
        }
    }
}

/// Recover a secp256k1 public key from a message hash and signature.
///
/// Signature format: r(32) || s(32) || v(1), with v in {0,1,27,28}.
///
/// If argument deserialization fails, returns `None`.
pub fn secp256k1_recover(
    msg_hash: [u8; 32],
    sig: [u8; 65],
) -> Option<[u8; 65]> {
    let v_raw = sig[64];
    let v = match v_raw {
        0 | 1 => v_raw as i32,
        27 | 28 => (v_raw - 27) as i32,
        _ => {
            warn!("vm: secp256k1 recovery: invalid v byte {v_raw}");
            return None;
        }
    };

    let rec_id = match secp256k1::ecdsa::RecoveryId::try_from(v) {
        Ok(id) => id,
        Err(e) => {
            warn!("vm: secp256k1 recovery: invalid recovery id {v} ({e})");
            return None;
        }
    };

    let sig = match RecoverableSignature::from_compact(&sig[0..64], rec_id) {
        Ok(sig) => sig,
        Err(e) => {
            warn!("vm: secp256k1 recovery: invalid signature ({e})");
            return None;
        }
    };
    let msg = Message::from_digest(msg_hash);

    let secp = Secp256k1::new();
    let pk = match secp.recover_ecdsa(msg, &sig) {
        Ok(pk) => pk,
        Err(e) => {
            warn!("vm: secp256k1 recovery failed ({e})");
            return None;
        }
    };
    Some(pk.serialize_uncompressed())
}

fn write_to_arg_buf<R>(arg_buf: &mut [u8], result: &R) -> u32
where
    R: Serialize<AllocSerializer<1024>>,
{
    let bytes = rkyv::to_bytes::<_, 1024>(result).unwrap();
    arg_buf[..bytes.len()].copy_from_slice(&bytes);
    bytes.len() as u32
}

/// Deserializes the argument buffer, applies `closure`, and writes the result
/// back.
///
/// The argument bytes are validated with [`rkyv::check_archived_root`] before
/// deserialization. If validation fails (e.g. malformed or malicious archive
/// data), the function logs a warning and writes the `fallback` value to the
/// argument buffer **without** invoking `closure`.
fn wrap_host_query<A, R, F>(
    arg_buf: &mut [u8],
    arg_len: u32,
    name: &str,
    fallback: &R,
    closure: F,
) -> u32
where
    F: FnOnce(A) -> R,
    A: Archive,
    A::Archived: for<'a> CheckBytes<DefaultValidator<'a>>
        + Deserialize<A, rkyv::Infallible>,
    R: Serialize<AllocSerializer<1024>>,
{
    let Some(root) =
        rkyv::check_archived_root::<A>(&arg_buf[..arg_len as usize]).ok()
    else {
        warn!("vm: invalid archived data in {name}");
        return write_to_arg_buf(arg_buf, fallback);
    };
    let arg: A = root.deserialize(&mut rkyv::Infallible).unwrap();

    let result = closure(arg);
    write_to_arg_buf(arg_buf, &result)
}

pub(crate) fn host_hash(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(arg_buf, arg_len, "host_hash", &BlsScalar::default(), hash)
}

pub(crate) fn host_poseidon_hash(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(
        arg_buf,
        arg_len,
        "host_poseidon_hash",
        &BlsScalar::default(),
        poseidon_hash,
    )
}

pub(crate) fn host_verify_plonk(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    let version = plonk_version();
    let hash = plonk_cache_key(version, &arg_buf[..arg_len as usize]);
    let cached = cache::get_plonk_verification(hash);

    wrap_host_query(
        arg_buf,
        arg_len,
        "host_verify_plonk",
        &false,
        |(vd, proof, pis)| {
            let is_valid = cached.unwrap_or_else(|| {
                verify_plonk_with_version(version, vd, proof, pis)
            });
            cache::put_plonk_verification(hash, is_valid);
            is_valid
        },
    )
}

pub(crate) fn host_verify_groth16_bn254(
    arg_buf: &mut [u8],
    arg_len: u32,
) -> u32 {
    let hash = *blake2b_simd::blake2b(&arg_buf[..arg_len as usize]).as_array();
    let cached = cache::get_groth16_verification(hash);

    wrap_host_query(
        arg_buf,
        arg_len,
        "host_verify_groth16_bn254",
        &false,
        |(pvk, proof, inputs)| {
            let is_valid = cached
                .unwrap_or_else(|| verify_groth16_bn254(pvk, proof, inputs));
            cache::put_groth16_verification(hash, is_valid);
            is_valid
        },
    )
}

pub(crate) fn host_verify_schnorr(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(
        arg_buf,
        arg_len,
        "host_verify_schnorr",
        &false,
        |(msg, pk, sig)| verify_schnorr(msg, pk, sig),
    )
}

pub(crate) fn host_verify_bls(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    let current_hard_fork = hard_fork();
    let hash = bls_cache_key(current_hard_fork, &arg_buf[..arg_len as usize]);
    let cached = cache::get_bls_verification(hash);

    wrap_host_query(
        arg_buf,
        arg_len,
        "host_verify_bls",
        &false,
        |(msg, pk, sig)| {
            let is_valid = cached.unwrap_or_else(|| verify_bls(msg, pk, sig));
            cache::put_bls_verification(hash, is_valid);
            is_valid
        },
    )
}

pub(crate) fn host_verify_bls_multisig(
    arg_buf: &mut [u8],
    arg_len: u32,
) -> u32 {
    wrap_host_query(
        arg_buf,
        arg_len,
        "host_verify_bls_multisig",
        &false,
        |(msg, keys, sig)| verify_bls_multisig(msg, keys, sig),
    )
}

pub(crate) fn host_keccak256(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(arg_buf, arg_len, "host_keccak256", &[0u8; 32], keccak256)
}

pub(crate) fn host_sha256(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(arg_buf, arg_len, "host_sha256", &[0u8; 32], sha256)
}

pub(crate) fn host_verify_kzg_proof(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(
        arg_buf,
        arg_len,
        "host_verify_kzg_proof",
        &false,
        |(commitment, z, y, proof)| verify_kzg_proof(commitment, z, y, proof),
    )
}

pub(crate) fn host_secp256k1_recover(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(
        arg_buf,
        arg_len,
        "host_secp256k1_recover",
        &Option::<[u8; 65]>::None,
        |(msg_hash, sig)| secp256k1_recover(msg_hash, sig),
    )
}
