// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Host queries registered on the Dusk VM.
//!
//! Several queries (`verify_bls`, `verify_bls_multisig`, PLONK verification,
//! memoization keys) depend on a **thread-local** [`HostQueryPolicy`] (PLONK
//! version + [`HardFork`]). The default is [`HardFork::PreFork`] with PLONK V2.
//!
//! A running Rusk node sets policy before block execution via the same public
//! [`set_host_query_policy`] API. Contract tests that use [`VM::ephemeral`](crate::VM::ephemeral)
//! must set policy explicitly when they need post-fork BLS semantics — see
//! `vm/tests/vm.rs` (`bls_signature`, `bls_multisig_signature`).

use alloc::vec::Vec;
use std::sync::LazyLock;

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
use secp256k1::ecdsa::RecoverableSignature;
use secp256k1::{Message, Secp256k1};
use sha2::{Digest as Sha2Digest, Sha256};
use sha3::Keccak256;
use tracing::warn;

mod cache;
mod pricing;

pub(crate) use pricing::{
    hash_host_query, keccak256_host_query, poseidon_hash_host_query,
    secp256k1_recover_host_query, sha256_host_query, verify_bls_host_query,
    verify_bls_multisig_host_query, verify_groth16_bn254_host_query,
    verify_kzg_proof_host_query, verify_plonk_host_query,
    verify_schnorr_host_query,
};

use self::cache::{CacheDomain, cache_key};
pub use self::cache::{
    HostQueryPolicy, HostQueryPolicyGuard, hard_fork, host_query_policy,
    plonk_version, set_host_query_policy,
};

static SECP256K1_CONTEXT: LazyLock<Secp256k1<secp256k1::All>> =
    LazyLock::new(Secp256k1::new);

/// Active hardfork context for host-query rule selection.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum HardFork {
    /// Behavior before any explicit hardfork activation.
    PreFork,
    /// Behavior after Aegis activation.
    Aegis,
    /// Behavior after Boreas activation.
    Boreas,
}

impl HardFork {
    /// Returns the BLS signature version for this hardfork.
    pub fn bls_version(&self) -> BlsVersion {
        match self {
            HardFork::Aegis | HardFork::Boreas => BlsVersion::V2,
            HardFork::PreFork => BlsVersion::V1,
        }
    }
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

    let pk = match SECP256K1_CONTEXT.recover_ecdsa(msg, &sig) {
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

fn memoize_host_query<R, FPut, FCompute>(
    cached: Option<R>,
    put: FPut,
    compute: FCompute,
) -> R
where
    R: Clone,
    FPut: FnOnce(R),
    FCompute: FnOnce() -> R,
{
    match cached {
        Some(result) => result,
        None => {
            let result = compute();
            put(result.clone());
            result
        }
    }
}

pub(crate) fn host_hash(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    let cache_key = cache_key(CacheDomain::Hash, &arg_buf[..arg_len as usize]);
    let cached = cache::get_hash(cache_key);

    wrap_host_query(
        arg_buf,
        arg_len,
        "host_hash",
        &BlsScalar::default(),
        |arg| {
            memoize_host_query(
                cached,
                |result| cache::put_hash(cache_key, result),
                || hash(arg),
            )
        },
    )
}

pub(crate) fn host_poseidon_hash(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    let hash =
        cache_key(CacheDomain::PoseidonHash, &arg_buf[..arg_len as usize]);
    let cached = cache::get_poseidon_hash(hash);

    wrap_host_query(
        arg_buf,
        arg_len,
        "host_poseidon_hash",
        &BlsScalar::default(),
        |arg| {
            memoize_host_query(
                cached,
                |result| cache::put_poseidon_hash(hash, result),
                || poseidon_hash(arg),
            )
        },
    )
}

pub(crate) fn host_verify_plonk(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    let version = plonk_version();
    let hash = cache_key(CacheDomain::Plonk, &arg_buf[..arg_len as usize]);
    let cached = cache::get_plonk_verification(hash);

    wrap_host_query(
        arg_buf,
        arg_len,
        "host_verify_plonk",
        &false,
        |(vd, proof, pis)| {
            memoize_host_query(
                cached,
                |is_valid| cache::put_plonk_verification(hash, is_valid),
                || verify_plonk_with_version(version, vd, proof, pis),
            )
        },
    )
}

pub(crate) fn host_verify_groth16_bn254(
    arg_buf: &mut [u8],
    arg_len: u32,
) -> u32 {
    let hash =
        cache_key(CacheDomain::Groth16Bn254, &arg_buf[..arg_len as usize]);
    let cached = cache::get_groth16_verification(hash);

    wrap_host_query(
        arg_buf,
        arg_len,
        "host_verify_groth16_bn254",
        &false,
        |(pvk, proof, inputs)| {
            memoize_host_query(
                cached,
                |is_valid| cache::put_groth16_verification(hash, is_valid),
                || verify_groth16_bn254(pvk, proof, inputs),
            )
        },
    )
}

pub(crate) fn host_verify_schnorr(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    let hash = cache_key(CacheDomain::Schnorr, &arg_buf[..arg_len as usize]);
    let cached = cache::get_schnorr_verification(hash);

    wrap_host_query(
        arg_buf,
        arg_len,
        "host_verify_schnorr",
        &false,
        |(msg, pk, sig)| {
            memoize_host_query(
                cached,
                |is_valid| cache::put_schnorr_verification(hash, is_valid),
                || verify_schnorr(msg, pk, sig),
            )
        },
    )
}

pub(crate) fn host_verify_bls(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    let hash = cache_key(CacheDomain::Bls, &arg_buf[..arg_len as usize]);
    let cached = cache::get_bls_verification(hash);

    wrap_host_query(
        arg_buf,
        arg_len,
        "host_verify_bls",
        &false,
        |(msg, pk, sig)| {
            memoize_host_query(
                cached,
                |is_valid| cache::put_bls_verification(hash, is_valid),
                || verify_bls(msg, pk, sig),
            )
        },
    )
}

pub(crate) fn host_verify_bls_multisig(
    arg_buf: &mut [u8],
    arg_len: u32,
) -> u32 {
    let hash =
        cache_key(CacheDomain::BlsMultisig, &arg_buf[..arg_len as usize]);
    let cached = cache::get_bls_multisig_verification(hash);

    wrap_host_query(
        arg_buf,
        arg_len,
        "host_verify_bls_multisig",
        &false,
        |(msg, keys, sig)| {
            memoize_host_query(
                cached,
                |is_valid| cache::put_bls_multisig_verification(hash, is_valid),
                || verify_bls_multisig(msg, keys, sig),
            )
        },
    )
}

pub(crate) fn host_keccak256(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    let hash = cache_key(CacheDomain::Keccak256, &arg_buf[..arg_len as usize]);
    let cached = cache::get_keccak256(hash);

    wrap_host_query(arg_buf, arg_len, "host_keccak256", &[0u8; 32], |arg| {
        memoize_host_query(
            cached,
            |output| cache::put_keccak256(hash, output),
            || keccak256(arg),
        )
    })
}

pub(crate) fn host_sha256(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    let hash = cache_key(CacheDomain::Sha256, &arg_buf[..arg_len as usize]);
    let cached = cache::get_sha256(hash);

    wrap_host_query(arg_buf, arg_len, "host_sha256", &[0u8; 32], |arg| {
        memoize_host_query(
            cached,
            |output| cache::put_sha256(hash, output),
            || sha256(arg),
        )
    })
}

pub(crate) fn host_verify_kzg_proof(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    let hash = cache_key(CacheDomain::Kzg, &arg_buf[..arg_len as usize]);
    let cached = cache::get_kzg_verification(hash);

    wrap_host_query(
        arg_buf,
        arg_len,
        "host_verify_kzg_proof",
        &false,
        |(commitment, z, y, proof)| {
            memoize_host_query(
                cached,
                |is_valid| cache::put_kzg_verification(hash, is_valid),
                || verify_kzg_proof(commitment, z, y, proof),
            )
        },
    )
}

pub(crate) fn host_secp256k1_recover(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    let hash =
        cache_key(CacheDomain::Secp256k1Recover, &arg_buf[..arg_len as usize]);
    let cached = cache::get_secp256k1_recover(hash);

    wrap_host_query(
        arg_buf,
        arg_len,
        "host_secp256k1_recover",
        &Option::<[u8; 65]>::None,
        |(msg_hash, sig)| {
            memoize_host_query(
                cached,
                |recovered| cache::put_secp256k1_recover(hash, recovered),
                || secp256k1_recover(msg_hash, sig),
            )
        },
    )
}

#[cfg(test)]
mod tests {
    use core::cell::RefCell;
    use std::cell::Cell;

    use super::*;

    #[test]
    fn plonk_verifier_overflow_lengths_do_not_panic() {
        let mut bytes = Vec::with_capacity(48);
        bytes.extend_from_slice(&0u64.to_be_bytes()); // label_len
        bytes.extend_from_slice(&0u64.to_be_bytes()); // verifier_key_len
        bytes.extend_from_slice(&0u64.to_be_bytes()); // opening_key_len
        bytes.extend_from_slice(&u64::MAX.to_be_bytes()); // public_input_indexes_len
        bytes.extend_from_slice(&0u64.to_be_bytes()); // size
        bytes.extend_from_slice(&0u64.to_be_bytes()); // constraints

        let result =
            std::panic::catch_unwind(|| Verifier::try_from_bytes(&bytes));
        assert!(result.is_ok(), "Verifier::try_from_bytes panicked");
        assert!(matches!(
            result.expect("checked above"),
            Err(dusk_core::plonk::Error::NotEnoughBytes)
        ));
    }

    #[test]
    fn memoize_host_query_skips_compute_and_store_on_hit() {
        let stores = Cell::new(0);
        let computes = Cell::new(0);
        let expected = hash(vec![1, 2, 3]);

        let result = memoize_host_query(
            Some(expected),
            |_| stores.set(stores.get() + 1),
            || {
                computes.set(computes.get() + 1);
                hash(vec![4, 5, 6])
            },
        );

        assert_eq!(result, expected);
        assert_eq!(stores.get(), 0);
        assert_eq!(computes.get(), 0);
    }

    #[test]
    fn memoize_host_query_stores_non_boolean_result_on_miss() {
        let stored = RefCell::new(Option::<[u8; 65]>::None);
        let computes = Cell::new(0);
        let expected = Some([7u8; 65]);

        let result = memoize_host_query(
            None,
            |recovered| *stored.borrow_mut() = recovered,
            || {
                computes.set(computes.get() + 1);
                expected
            },
        );

        assert_eq!(result, expected);
        assert_eq!(*stored.borrow(), expected);
        assert_eq!(computes.get(), 1);
    }
}
