// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;
use blake2b_simd::Params;
use std::env;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use dusk_bytes::DeserializableSlice;
use dusk_plonk::prelude::{Proof, Verifier};
use dusk_poseidon::{Domain, Hash as PoseidonHash};
use execution_core::{
    BlsAggPublicKey, BlsPublicKey, BlsScalar, BlsSignature, SchnorrPublicKey,
    SchnorrSignature,
};
use lru::LruCache;
use rkyv::ser::serializers::AllocSerializer;
use rkyv::{Archive, Deserialize, Serialize};

pub use piecrust::*;

use crate::hash::Hasher;
use crate::{Metadata, PublicInput, Query};

/// Create a new session based on the given `vm`. The vm *must* have been
/// created using [`new_vm`] or [`new_ephemeral_vm`].
pub fn new_session(
    vm: &VM,
    base: [u8; 32],
    block_height: u64,
) -> Result<Session, Error> {
    vm.session(
        SessionData::builder()
            .base(base)
            .insert(Metadata::BLOCK_HEIGHT, block_height)?,
    )
}

/// Create a new genesis session based on the given `vm`. The vm *must* have
/// been created using [`new_vm`] or [`new_ephemeral_vm`].
pub fn new_genesis_session(vm: &VM) -> Session {
    vm.session(
        SessionData::builder()
            .insert(Metadata::BLOCK_HEIGHT, 0)
            .expect("Inserting block height in metadata should succeed"),
    )
    .expect("Creating a genesis session should always succeed")
}

/// Create a new [`VM`] compliant with Dusk's specification.
pub fn new_vm<P: AsRef<Path> + Into<PathBuf>>(
    root_dir: P,
) -> Result<VM, Error> {
    let mut vm = VM::new(root_dir)?;
    register_host_queries(&mut vm);
    Ok(vm)
}

/// Creates a new [`VM`] with a temporary directory.
pub fn new_ephemeral_vm() -> Result<VM, Error> {
    let mut vm = VM::ephemeral()?;
    register_host_queries(&mut vm);
    Ok(vm)
}

fn register_host_queries(vm: &mut VM) {
    vm.register_host_query(Query::HASH, host_hash);
    vm.register_host_query(Query::POSEIDON_HASH, host_poseidon_hash);
    vm.register_host_query(Query::VERIFY_PROOF, host_verify_proof);
    vm.register_host_query(Query::VERIFY_SCHNORR, host_verify_schnorr);
    vm.register_host_query(Query::VERIFY_BLS, host_verify_bls);
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

fn host_verify_proof(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(arg_buf, arg_len, |(vd, proof, pis)| {
        verify_proof(vd, proof, pis)
    })
}

fn host_verify_schnorr(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(arg_buf, arg_len, |(msg, pk, sig)| {
        verify_schnorr(msg, pk, sig)
    })
}

fn host_verify_bls(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(arg_buf, arg_len, |(msg, pk, sig)| verify_bls(msg, pk, sig))
}

/// Compute the blake2b hash of the given scalars, returning the resulting
/// scalar. The output of the hasher is truncated (last nibble) to fit onto a
/// scalar.
pub fn hash(bytes: Vec<u8>) -> BlsScalar {
    Hasher::digest(bytes)
}

/// Compute the poseidon hash of the given scalars
pub fn poseidon_hash(scalars: Vec<BlsScalar>) -> BlsScalar {
    PoseidonHash::digest(Domain::Other, &scalars)[0]
}

/// A simple LRU cache for plonk verification.
///
/// # Safety
/// `f` should not panic.
unsafe fn with_verification_cache<T, F>(f: F) -> T
where
    F: FnOnce(MutexGuard<LruCache<[u8; blake2b_simd::OUTBYTES], bool>>) -> T,
{
    const VERIFICATION_CACHE_SIZE: usize = 512;

    static CACHE: OnceLock<
        Mutex<LruCache<[u8; blake2b_simd::OUTBYTES], bool>>,
    > = OnceLock::new();

    CACHE
        .get_or_init(|| {
            let mut cache_size = None;

            if let Ok(s) = env::var("RUSK_ABI_PREFERIFY_CACHE_SIZE") {
                cache_size = s.parse().ok();
            }

            let mut cache_size = cache_size.unwrap_or(VERIFICATION_CACHE_SIZE);
            if cache_size == 0 {
                cache_size = VERIFICATION_CACHE_SIZE;
            }

            Mutex::new(LruCache::new(NonZeroUsize::new(cache_size).unwrap()))
        })
        .lock()
        .map(f)
        .unwrap()
}

fn get_cache(hash: [u8; blake2b_simd::OUTBYTES]) -> Option<bool> {
    // SAFETY: The cache never panics
    unsafe { with_verification_cache(|mut cache| cache.get(&hash).copied()) }
}

fn put_cache(hash: [u8; blake2b_simd::OUTBYTES], verified: bool) {
    // SAFETY: The cache never panics
    unsafe {
        with_verification_cache(|mut cache| {
            cache.put(hash, verified);
        });
    }
}

/// Verify a proof is valid for a given circuit type and public inputs
///
/// # Panics
/// This will panic if `verifier_data` is not valid.
pub fn verify_proof(
    verifier_data: Vec<u8>,
    proof: Vec<u8>,
    public_inputs: Vec<PublicInput>,
) -> bool {
    let mut hasher = Params::default().to_state();

    hasher.update(&verifier_data);
    hasher.update(&proof);
    public_inputs
        .iter()
        .for_each(|pi| pi.update_hasher(&mut hasher));

    let hash = *hasher.finalize().as_array();

    // If the proof verification has been memoized with the same arguments,
    // return the result
    if let Some(v) = get_cache(hash) {
        return v;
    }

    let verifier = Verifier::try_from_bytes(verifier_data)
        .expect("Verifier data coming from the contract should be valid");
    let proof = Proof::from_slice(&proof).expect("Proof should be valid");

    let n_pi = public_inputs.iter().fold(0, |num, pi| {
        num + match pi {
            PublicInput::Point(_) => 2,
            PublicInput::BlsScalar(_) => 1,
            PublicInput::JubJubScalar(_) => 1,
        }
    });

    let mut pis = Vec::with_capacity(n_pi);

    public_inputs.into_iter().for_each(|pi| match pi {
        PublicInput::Point(p) => pis.extend([p.get_u(), p.get_v()]),
        PublicInput::BlsScalar(s) => pis.push(s),
        PublicInput::JubJubScalar(s) => {
            let s: BlsScalar = s.into();
            pis.push(s)
        }
    });

    let verified = verifier.verify(&proof, &pis[..]).is_ok();
    if verified {
        put_cache(hash, verified);
    }
    verified
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
    let apk = BlsAggPublicKey::from(&pk);
    apk.verify(&sig, &msg).is_ok()
}
