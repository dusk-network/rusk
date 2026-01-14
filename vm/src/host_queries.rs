// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The host-queries registered on the Dusk VM

use rkyv::ser::serializers::AllocSerializer;
use rkyv::{Archive, Deserialize, Serialize};
use transfer::host_queries_flat::{hash, keccak256, poseidon_hash, verify_bls, verify_bls_multisig, verify_groth16_bn254, verify_plonk, verify_schnorr};

use crate::cache;


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

pub(crate) fn host_keccak256(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(arg_buf, arg_len, keccak256)
}
