// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! This module provides the foreign function interface (FFI) for exposing
//! public functions from the `wallet-core` Rust library to a WASM runtime.
//! In addition to cryptographic operations, it offers memory management
//! functions, such as `malloc` and `free`, for interacting with the WASM
//! memory.
//!
//! This FFI allows seamless integration between Rust code and a WASM runtime
//! while ensuring efficient memory handling and secure key management.

#[macro_use]
pub(crate) mod debug;

pub mod error;
pub mod panic;

use crate::keys::{
    derive_bls_pk, derive_phoenix_pk, derive_phoenix_sk, derive_phoenix_vk,
};
use crate::notes;
use crate::phoenix_balance;
use crate::Seed;
use error::ErrorCode;

use alloc::alloc::{alloc, dealloc, Layout};
use alloc::vec::Vec;
use core::{ptr, slice};
use dusk_bytes::Serializable;
use execution_core::{
    signatures::bls::PublicKey as BlsPublicKey,
    transfer::phoenix::{NoteLeaf, PublicKey as PhoenixPublicKey},
};
use zeroize::Zeroize;

use rkyv::{from_bytes, to_bytes};

/// The size of the scratch buffer used for parsing the notes.
/// It can roughly contains less than 128 serialized notes.
const NOTES_BUFFER_SIZE: usize = 96 * 1024;

/// The alignment of the memory allocated by the FFI.
///
/// This is 1 because we're not allocating any complex data structures, and
/// just interacting with the memory directly.
const ALIGNMENT: usize = 1;

/// Allocates a buffer of `len` bytes on the WASM memory.
#[no_mangle]
pub fn malloc(len: u32) -> u32 {
    unsafe {
        let layout = Layout::from_size_align_unchecked(len as usize, ALIGNMENT);
        let ptr = alloc(layout);
        ptr as _
    }
}

/// Frees a previously allocated buffer on the WASM memory.
#[no_mangle]
pub fn free(ptr: u32, len: u32) {
    unsafe {
        let layout = Layout::from_size_align_unchecked(len as usize, ALIGNMENT);
        dealloc(ptr as _, layout);
    }
}

/// Map a list of indexes into keys using the provided seed and callback.
unsafe fn indexes_into_keys<T, F>(
    seed: &Seed,
    indexes: *const u8,
    mut callback: F,
) -> Vec<T>
where
    F: FnMut(&Seed, u8) -> T,
{
    let len = *indexes as usize;
    let slice = slice::from_raw_parts(indexes.add(1), len);
    slice.iter().map(|&byte| callback(seed, byte)).collect()
}

unsafe fn read_buffer(ptr: *const u8) -> Vec<u8> {
    let len = slice::from_raw_parts(ptr, 4);
    let len = u32::from_le_bytes(len.try_into().unwrap()) as usize;
    slice::from_raw_parts(ptr.add(4), len).to_vec()
}

/// Generate a profile (account / address pair) for the given seed and index.
#[no_mangle]
pub unsafe extern "C" fn generate_profile(
    seed: &Seed,
    index: u8,
    profile: *mut [u8; PhoenixPublicKey::SIZE + BlsPublicKey::SIZE],
) -> ErrorCode {
    let ppk = derive_phoenix_pk(seed, index).to_bytes();
    let bpk = derive_bls_pk(seed, index).to_bytes();

    ptr::copy_nonoverlapping(
        &ppk[0],
        &mut (*profile)[0],
        PhoenixPublicKey::SIZE,
    );

    ptr::copy_nonoverlapping(
        &bpk[0],
        &mut (*profile)[PhoenixPublicKey::SIZE],
        BlsPublicKey::SIZE,
    );

    ErrorCode::Ok
}

/// Filter all notes and their block height that are owned by the given keys,
/// mapped to their nullifiers.
#[no_mangle]
pub unsafe fn map_owned(
    seed: &Seed,
    indexes: *const u8,
    notes_ptr: *mut u8,
) -> ErrorCode {
    let keys = indexes_into_keys(seed, indexes, derive_phoenix_sk);
    let notes = read_buffer(notes_ptr);
    let notes: Vec<NoteLeaf> = from_bytes::<Vec<NoteLeaf>>(&notes)
        .or(Err(ErrorCode::UnarchivingError))?;

    let owned = notes::owned::map(&keys, notes);

    keys.into_iter().for_each(|mut sk| sk.zeroize());

    let bytes = to_bytes::<_, NOTES_BUFFER_SIZE>(&owned)
        .or(Err(ErrorCode::ArchivingError))?;

    let len = bytes.len().to_le_bytes();

    ptr::copy_nonoverlapping(len.as_ptr(), notes_ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), notes_ptr.add(4), bytes.len());

    ErrorCode::Ok
}

/// Calculate the balance info for the phoenix address at the given index for
/// the given seed.
#[no_mangle]
pub unsafe fn balance(
    seed: &Seed,
    index: u8,
    notes_ptr: *const u8,
    balance_info_ptr: *mut [u8; 16],
) -> ErrorCode {
    let vk = derive_phoenix_vk(seed, index);

    let notes = read_buffer(notes_ptr);
    let notes: Vec<NoteLeaf> = from_bytes::<Vec<NoteLeaf>>(&notes)
        .or(Err(ErrorCode::UnarchivingError))?;

    let info = phoenix_balance(&vk, notes.iter());

    ptr::copy_nonoverlapping(
        info.to_bytes().as_ptr(),
        &mut (*balance_info_ptr)[0],
        16,
    );

    ErrorCode::Ok
}
