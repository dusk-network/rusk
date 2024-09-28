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
pub mod mem;
pub mod panic;

use crate::keys::{
    derive_bls_pk, derive_phoenix_pk, derive_phoenix_sk, derive_phoenix_vk,
};
use crate::notes::{self, owned, pick};
use crate::phoenix_balance;
use crate::Seed;
use error::ErrorCode;

use alloc::vec::Vec;
use core::{ptr, slice};
use dusk_bytes::{DeserializableSlice, Serializable};
use execution_core::{
    signatures::bls::PublicKey as BlsPublicKey,
    transfer::phoenix::{
        ArchivedNoteLeaf, NoteLeaf, NoteOpening, PublicKey as PhoenixPublicKey,
    },
    BlsScalar,
};
use zeroize::Zeroize;

use rkyv::to_bytes;

use rand_chacha::{rand_core::SeedableRng, ChaCha12Rng};

#[no_mangle]
static KEY_SIZE: usize = BlsScalar::SIZE;
#[no_mangle]
static ITEM_SIZE: usize = core::mem::size_of::<ArchivedNoteLeaf>();

/// The size of the scratch buffer used for parsing the notes.
const NOTES_BUFFER_SIZE: usize = 96 * 1024;

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

/// Generate a profile (account / address pair) for the given seed and index.
#[no_mangle]
pub unsafe extern "C" fn generate_profile(
    seed: &Seed,
    index: u8,
    profile: *mut [u8; PhoenixPublicKey::SIZE + BlsPublicKey::SIZE],
) -> ErrorCode {
    let ppk = derive_phoenix_pk(seed, index).to_bytes();
    let bpk = derive_bls_pk(seed, index).to_bytes();
    let bpk2 = derive_bls_pk(seed, index).to_raw_bytes();

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
    notes_ptr: *const u8,
    owned_ptr: *mut *mut u8,
    last_info_ptr: *mut [u8; 16],
) -> ErrorCode {
    use core::cmp::max;

    let keys = indexes_into_keys(seed, indexes, derive_phoenix_sk);
    let notes: Vec<NoteLeaf> = mem::from_buffer(notes_ptr)?;

    let (block_height, pos) =
        notes
            .iter()
            .fold((0u64, 0u64), |(block_height, pos), leaf| {
                (
                    max(block_height, leaf.block_height),
                    max(pos, *leaf.note.pos()),
                )
            });

    let owned = notes::owned::map(&keys, notes);

    keys.into_iter().for_each(|mut sk| sk.zeroize());

    let bytes = to_bytes::<_, NOTES_BUFFER_SIZE>(&owned)
        .or(Err(ErrorCode::ArchivingError))?;

    let len = bytes.len().to_le_bytes();

    let ptr = mem::malloc(4 + bytes.len() as u32);

    let ptr = ptr as *mut u8;

    *owned_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());
    ptr::copy_nonoverlapping(
        block_height.to_le_bytes().as_ptr(),
        &mut (*last_info_ptr)[0],
        8,
    );
    ptr::copy_nonoverlapping(
        pos.to_le_bytes().as_ptr(),
        &mut (*last_info_ptr)[8],
        8,
    );

    ErrorCode::Ok
}

#[no_mangle]
pub unsafe fn accounts_into_raw(
    accounts_ptr: *const u8,
    raws_ptr: *mut *mut u8,
) -> ErrorCode {
    let bytes: Vec<u8> = mem::read_buffer(accounts_ptr)
        .chunks(BlsPublicKey::SIZE)
        .map(BlsPublicKey::from_slice)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ErrorCode::DeserializationError)?
        .into_iter()
        .map(|bpk| to_bytes::<_, 256>(&bpk))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ErrorCode::ArchivingError)?
        .iter()
        .fold(Vec::new(), |mut vec, aligned| {
            vec.extend_from_slice(aligned.as_slice());
            vec
        });

    let len = bytes.len().to_le_bytes();
    let ptr = mem::malloc(4 + bytes.len() as u32);
    let ptr = ptr as *mut u8;

    *raws_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

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

    let notes: Vec<NoteLeaf> = mem::from_buffer(notes_ptr)?;

    let info = phoenix_balance(&vk, notes.iter());

    ptr::copy_nonoverlapping(
        info.to_bytes().as_ptr(),
        &mut (*balance_info_ptr)[0],
        16,
    );

    ErrorCode::Ok
}

/// Pick the notes to be used in a transaction from an owned notes list.
#[no_mangle]
pub unsafe fn pick_notes(
    seed: &Seed,
    index: u8,
    value: *const u64,
    notes_ptr: *mut u8,
) -> ErrorCode {
    let vk = derive_phoenix_vk(seed, index);

    let notes: owned::NoteList = mem::from_buffer(notes_ptr)?;

    let notes = pick::notes(&vk, notes, *value);

    let bytes = to_bytes::<_, NOTES_BUFFER_SIZE>(&notes)
        .or(Err(ErrorCode::ArchivingError))?;

    let len = bytes.len().to_le_bytes();

    ptr::copy_nonoverlapping(len.as_ptr(), notes_ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), notes_ptr.add(4), bytes.len());

    ErrorCode::Ok
}

/// Gets the bookmark from the given note.
#[no_mangle]
pub unsafe fn bookmark(leaf_ptr: *const u8, bookmark: *mut u64) -> ErrorCode {
    let leaf: NoteLeaf = mem::from_buffer(leaf_ptr)?;

    *bookmark = *leaf.note.pos();

    ErrorCode::Ok
}

#[no_mangle]
pub unsafe fn phoenix(
    rng: &[u8; 32],
    seed: &Seed,
    sender_index: u8,
    receiver: &[u8; PhoenixPublicKey::SIZE],
    inputs: *const u8,
    openings: *const u8,
    root: &[u8; BlsScalar::SIZE],
    transfer_value: *const u64,
    obfuscated_transaction: bool, // ?
    deposit: *const u64,          // ?
    gas_limit: *const u64,
    gas_price: *const u64,
    chain_id: u8,    // ?
    data: *const u8, // ?
) -> ErrorCode {
    let rng = ChaCha12Rng::from_seed(*rng);

    let sender_sk = derive_phoenix_sk(&seed, sender_index);
    let receiver_pk = PhoenixPublicKey::from_bytes(receiver);
    let root = BlsScalar::from_bytes(root);

    let NoteOpening = mem::from_buffer(opening)?;

    ErrorCode::Ok
}

// pub fn phoenix<R: RngCore + CryptoRng, P: Prove>(
//     rng: &mut R,
//     sender_sk: &PhoenixSecretKey,
//     change_pk: &PhoenixPublicKey,
//     receiver_pk: &PhoenixPublicKey,
//     inputs: Vec<(Note, NoteOpening)>,
//     root: BlsScalar,
//     transfer_value: u64,
//     obfuscated_transaction: bool,
//     deposit: u64,
//     gas_limit: u64,
//     gas_price: u64,
//     chain_id: u8,
//     data: Option<impl Into<TransactionData>>,
//     prover: &P,
// ) -> Result<Transaction, Error> {
//     Ok(PhoenixTransaction::new::<R, P>(
//         rng,
//         sender_sk,
//         change_pk,
//         receiver_pk,
//         inputs,
//         root,
//         transfer_value,
//         obfuscated_transaction,
//         deposit,
//         gas_limit,
//         gas_price,
//         chain_id,
//         data,
//         prover,
//     )?
//     .into())
