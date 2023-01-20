// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;

use crate::hash::Hasher;
use crate::query::*;
use crate::PublicInput;

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{
    PublicKey as BlsPublicKey, Signature as BlsSignature, APK,
};
use dusk_bytes::DeserializableSlice;
use dusk_pki::PublicKey;
use dusk_plonk::prelude::*;
use dusk_schnorr::Signature;
use piecrust::{Session, VM};
use rkyv::ser::serializers::AllocSerializer;
use rkyv::{Archive, Deserialize, Serialize};

/// Set the block height for the given session.
pub fn set_block_height(session: &mut Session, block_height: u64) {
    session.set_meta(Metadata::BLOCK_HEIGHT, block_height);
}

/// Register the host queries offered by the ABI with the VM.
pub fn register_host_queries(vm: &mut VM) {
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
    dusk_poseidon::sponge::hash(&scalars)
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
    let verifier = Verifier::<DummyCircuit>::try_from_bytes(verifier_data)
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

    // FIXME: Plonk seems to be expecting `-pi`s, which is quite strange. Maybe
    //  some bug in Plonk?
    public_inputs.into_iter().for_each(|pi| match pi {
        PublicInput::Point(p) => pis.extend([-p.get_x(), -p.get_y()]),
        PublicInput::BlsScalar(s) => pis.push(-s),
        PublicInput::JubJubScalar(s) => {
            let s: BlsScalar = s.into();
            pis.push(-s)
        }
    });

    verifier.verify(&proof, &pis).is_ok()
}

/// Verify a schnorr signature is valid for the given public key and message
pub fn verify_schnorr(msg: BlsScalar, pk: PublicKey, sig: Signature) -> bool {
    sig.verify(&pk, msg)
}

/// Verify a BLS signature is valid for the given public key and message
pub fn verify_bls(msg: Vec<u8>, pk: BlsPublicKey, sig: BlsSignature) -> bool {
    let apk = APK::from(&pk);
    apk.verify(&sig, &msg).is_ok()
}

#[derive(Default)]
struct DummyCircuit;

impl Circuit for DummyCircuit {
    fn circuit<C>(&self, _: &mut C) -> Result<(), Error>
    where
        C: Composer,
    {
        unreachable!(
            "This circuit should never be compiled or proven, only verified"
        )
    }
}
