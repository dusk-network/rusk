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

use alloc::string::String;
use alloc::vec::Vec;
use core::{ptr, slice};

use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::stake::{Stake, STAKE_CONTRACT};
use dusk_core::transfer::data::{ContractCall, TransactionData};
use dusk_core::transfer::moonlight::Transaction as MoonlightTransaction;
use dusk_core::transfer::phoenix::{
    ArchivedNoteLeaf, Note, NoteLeaf, NoteOpening, Prove,
    PublicKey as PhoenixPublicKey,
};
use dusk_core::transfer::withdraw::WithdrawReplayToken;
use dusk_core::transfer::{phoenix, Transaction};
use dusk_core::BlsScalar;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha12Rng;
use rkyv::to_bytes;
use zeroize::Zeroize;

use crate::keys::{
    derive_bls_pk, derive_bls_sk, derive_phoenix_pk, derive_phoenix_sk,
    derive_phoenix_vk,
};
use crate::notes::{self, balance, owned, pick};
use crate::Seed;

use error::ErrorCode;

#[no_mangle]
static KEY_SIZE: usize = BlsScalar::SIZE;
#[no_mangle]
static ITEM_SIZE: usize = core::mem::size_of::<ArchivedNoteLeaf>();

#[no_mangle]
static MINIMUM_STAKE: u64 = dusk_core::stake::MINIMUM_STAKE;

/// The size of the scratch buffer used for parsing the notes.
const NOTES_BUFFER_SIZE: usize = 96 * 1024;

fn revert(value: &BlsScalar) -> String {
    // Unfortunately, the BlsScalar type had a display implementation that
    // does not follow the raw bytes format. Therefore the `tx.hash` display
    // DOES NOT match the `tx.hash` of the network.
    let displayed = alloc::format!("{}", &value);
    let displayed = displayed.chars().skip(2).collect::<Vec<_>>();
    let displayed = displayed.chunks(2).rev().flatten().collect::<String>();

    displayed
}

fn as_phoenix_transaction(
    tx: Transaction,
) -> Result<phoenix::Transaction, ErrorCode> {
    match tx {
        Transaction::Phoenix(tx) => Ok(tx),
        _ => Err(ErrorCode::PhoenixTransactionError),
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
pub unsafe fn display_scalar(
    scalar_ptr: &[u8; 32],
    output: &mut [u8; 64],
) -> ErrorCode {
    let scalar: BlsScalar =
        rkyv::from_bytes(scalar_ptr).or(Err(ErrorCode::UnarchivingError))?;
    let displayed = alloc::format!("{}", scalar);
    let bytes = displayed.as_bytes();

    ptr::copy_nonoverlapping(bytes[2..].as_ptr(), output.as_mut_ptr(), 64);

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

    let info = balance::calculate_unchecked(&vk, notes.iter());

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
pub unsafe fn bookmarks(
    notes_ptr: *const u8,
    bookmarks_ptr: *mut *mut u8,
) -> ErrorCode {
    let notes: Vec<NoteLeaf> = mem::from_buffer(notes_ptr)?;

    let bookmarks: Vec<u64> =
        notes.into_iter().map(|leaf| *leaf.note.pos()).collect();

    let bytes: Vec<u8> = bookmarks
        .iter()
        .flat_map(|&num| num.to_le_bytes())
        .collect();

    let ptr = mem::malloc(bytes.len() as u32);
    let ptr = ptr as *mut u8;

    *bookmarks_ptr = ptr;

    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());

    ErrorCode::Ok
}

#[derive(Default)]
struct NoOpProver {
    circuits: core::cell::RefCell<Vec<u8>>,
}

impl Prove for NoOpProver {
    fn prove(&self, circuits: &[u8]) -> Result<Vec<u8>, dusk_core::Error> {
        *self.circuits.borrow_mut() = circuits.to_vec();

        Ok(circuits.to_vec())
    }
}

#[no_mangle]
pub unsafe fn into_proven(
    tx_ptr: *const u8,
    proof_ptr: *const u8,
    proven_ptr: *mut *mut u8,
    hash_ptr: &mut [u8; 64],
) -> ErrorCode {
    let tx = mem::read_buffer(tx_ptr);
    let mut tx: phoenix::Transaction = mem::parse_buffer(tx)?;
    let proof = mem::read_buffer(proof_ptr);

    tx.set_proof(proof.to_vec());

    let bytes = Transaction::Phoenix(tx.clone()).to_var_bytes();

    let len = bytes.len().to_le_bytes();

    let ptr = mem::malloc(4 + bytes.len() as u32);
    let ptr = ptr as *mut u8;

    *proven_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    let displayed = revert(&tx.hash());
    let bytes = displayed.as_bytes();

    ptr::copy_nonoverlapping(bytes.as_ptr(), hash_ptr.as_mut_ptr(), 64);

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
    obfuscated_transaction: bool,
    deposit: *const u64,
    gas_limit: *const u64,
    gas_price: *const u64,
    chain_id: u8,
    data: *const u8,
    tx_ptr: *mut *mut u8,
    proof_ptr: *mut *mut u8,
) -> ErrorCode {
    let mut rng = ChaCha12Rng::from_seed(*rng);

    let sender_sk = derive_phoenix_sk(&seed, sender_index);
    let change_pk = PhoenixPublicKey::from(&sender_sk);
    let receiver_pk = PhoenixPublicKey::from_bytes(receiver)
        .or(Err(ErrorCode::DeserializationError))?;

    let root: BlsScalar =
        rkyv::from_bytes(root).or(Err(ErrorCode::UnarchivingError))?;

    let openings: Vec<Option<NoteOpening>> = mem::from_buffer(openings)?;

    let notes: Vec<NoteLeaf> = mem::from_buffer(inputs)?;

    let inputs: Vec<(Note, NoteOpening)> = notes
        .into_iter()
        .map(|note_leaf| note_leaf.note)
        .zip(openings.into_iter())
        .filter_map(|(note, opening)| opening.map(|op| (note, op)))
        .collect();

    let data: Option<TransactionData> =
        if data.is_null() { None } else { todo!() };

    let prover = NoOpProver::default();

    let tx = phoenix::Transaction::new(
        &mut rng,
        &sender_sk,
        &change_pk,
        &receiver_pk,
        inputs,
        root,
        *transfer_value,
        obfuscated_transaction,
        *deposit,
        *gas_limit,
        *gas_price,
        chain_id,
        data,
        &prover,
    )
    .or(Err(ErrorCode::PhoenixTransactionError))?;

    let bytes = to_bytes::<_, 4096>(&tx).or(Err(ErrorCode::ArchivingError))?;
    let len = bytes.len().to_le_bytes();

    let ptr = mem::malloc(4 + bytes.len() as u32);
    let ptr = ptr as *mut u8;

    *tx_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    let bytes = prover.circuits.into_inner();
    let len = bytes.len().to_le_bytes();

    let ptr = mem::malloc(4 + bytes.len() as u32);
    let ptr = ptr as *mut u8;

    *proof_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    ErrorCode::Ok
}

#[no_mangle]
pub unsafe fn moonlight(
    seed: &Seed,
    sender_index: u8,
    receiver: *const [u8; BlsPublicKey::SIZE],
    transfer_value: *const u64,
    deposit: *const u64,
    gas_limit: *const u64,
    gas_price: *const u64,
    nonce: *const u64,
    chain_id: u8,
    data: *const u8,
    tx_ptr: *mut *mut u8,
    hash_ptr: &mut [u8; 64],
) -> ErrorCode {
    let sender_sk = derive_bls_sk(&seed, sender_index);

    let receiver_pk = if receiver.is_null() {
        None
    } else {
        Some(
            BlsPublicKey::from_bytes(&*receiver)
                .or(Err(ErrorCode::DeserializationError))?,
        )
    };

    let data: Option<TransactionData> = if data.is_null() {
        None
    } else {
        let buffer = mem::read_buffer(data);

        Some(buffer[1..].to_vec().into())
    };

    let tx = MoonlightTransaction::new(
        &sender_sk,
        receiver_pk,
        *transfer_value,
        *deposit,
        *gas_limit,
        *gas_price,
        *nonce,
        chain_id,
        data,
    )
    .or(Err(ErrorCode::MoonlightTransactionError))?;

    let bytes = Transaction::Moonlight(tx.clone()).to_var_bytes();
    let len = bytes.len().to_le_bytes();

    let ptr = mem::malloc(4 + bytes.len() as u32);
    let ptr = ptr as *mut u8;

    *tx_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    let displayed = revert(&tx.hash());
    let bytes = displayed.as_bytes();

    ptr::copy_nonoverlapping(bytes.as_ptr(), hash_ptr.as_mut_ptr(), 64);

    ErrorCode::Ok
}

#[no_mangle]
pub unsafe fn phoenix_to_moonlight(
    rng: &[u8; 32],
    seed: &Seed,
    profile_index: u8,
    inputs: *const u8,
    openings: *const u8,
    nullifiers: *const u8,
    root: &[u8; BlsScalar::SIZE],
    allocate_value: *const u64,
    gas_limit: *const u64,
    gas_price: *const u64,
    chain_id: u8,
    tx_ptr: *mut *mut u8,
    proof_ptr: *mut *mut u8,
) -> ErrorCode {
    let mut rng = ChaCha12Rng::from_seed(*rng);

    let phoenix_sender_sk = derive_phoenix_sk(&seed, profile_index);
    let moonlight_receiver_sk = derive_bls_sk(&seed, profile_index);

    let root: BlsScalar =
        rkyv::from_bytes(root).or(Err(ErrorCode::UnarchivingError))?;

    let openings: Vec<Option<NoteOpening>> = mem::from_buffer(openings)?;
    let nullifiers: Vec<BlsScalar> = mem::from_buffer(nullifiers)?;
    let notes: Vec<NoteLeaf> = mem::from_buffer(inputs)?;

    let inputs: Vec<(Note, NoteOpening, BlsScalar)> = notes
        .into_iter()
        .map(|note_leaf| note_leaf.note)
        .zip(openings.into_iter())
        .zip(nullifiers.into_iter())
        .filter_map(|((note, opening), nullifier)| {
            opening.map(|op| (note, op, nullifier))
        })
        .collect();

    let prover = NoOpProver::default();

    let tx = crate::transaction::phoenix_to_moonlight(
        &mut rng,
        &phoenix_sender_sk,
        &moonlight_receiver_sk,
        inputs,
        root,
        *allocate_value,
        *gas_limit,
        *gas_price,
        chain_id,
        &prover,
    )
    .or(Err(ErrorCode::PhoenixTransactionError))?;

    let tx = as_phoenix_transaction(tx)?;

    let bytes = to_bytes::<_, 4096>(&tx).or(Err(ErrorCode::ArchivingError))?;
    let len = bytes.len().to_le_bytes();

    let ptr = mem::malloc(4 + bytes.len() as u32);
    let ptr = ptr as *mut u8;

    *tx_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    let bytes = prover.circuits.into_inner();
    let len = bytes.len().to_le_bytes();

    let ptr = mem::malloc(4 + bytes.len() as u32);
    let ptr = ptr as *mut u8;

    *proof_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    ErrorCode::Ok
}

#[no_mangle]
pub unsafe fn moonlight_to_phoenix(
    rng: &[u8; 32],
    seed: &Seed,
    profile_index: u8,
    allocate_value: *const u64,
    gas_limit: *const u64,
    gas_price: *const u64,
    nonce: *const u64,
    chain_id: u8,
    tx_ptr: *mut *mut u8,
    hash_ptr: &mut [u8; 64],
) -> ErrorCode {
    let mut rng = ChaCha12Rng::from_seed(*rng);

    let moonlight_sender_sk = derive_bls_sk(&seed, profile_index);
    let phoenix_receiver_sk = derive_phoenix_sk(&seed, profile_index);

    let tx = crate::transaction::moonlight_to_phoenix(
        &mut rng,
        &moonlight_sender_sk,
        &phoenix_receiver_sk,
        *allocate_value,
        *gas_limit,
        *gas_price,
        *nonce,
        chain_id,
    )
    .or(Err(ErrorCode::MoonlightTransactionError))?;

    let bytes = tx.to_var_bytes();
    let len = bytes.len().to_le_bytes();

    let ptr = mem::malloc(4 + bytes.len() as u32);
    let ptr = ptr as *mut u8;

    *tx_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    let displayed = revert(&tx.hash());
    let bytes = displayed.as_bytes();

    ptr::copy_nonoverlapping(bytes.as_ptr(), hash_ptr.as_mut_ptr(), 64);

    ErrorCode::Ok
}

#[no_mangle]
pub unsafe fn moonlight_stake(
    seed: &Seed,
    sender_index: u8,
    stake_value: *const u64,
    gas_limit: *const u64,
    gas_price: *const u64,
    nonce: *const u64,
    chain_id: u8,
    tx_ptr: *mut *mut u8,
    hash_ptr: &mut [u8; 64],
) -> ErrorCode {
    let transfer_value = 0;
    let deposit = *stake_value;

    let sender_sk = derive_bls_sk(&seed, sender_index);
    let stake_sk = sender_sk.clone();

    let stake = Stake::new(&stake_sk, *stake_value, chain_id);

    let contract_call = ContractCall::new(STAKE_CONTRACT, "stake", &stake)
        .or(Err(ErrorCode::ContractCallError))?;

    let tx = crate::transaction::moonlight(
        &sender_sk,
        None,
        transfer_value,
        deposit,
        *gas_limit,
        *gas_price,
        *nonce,
        chain_id,
        Some(contract_call),
    )
    .or(Err(ErrorCode::MoonlightTransactionError))?;

    let bytes = tx.to_var_bytes();
    let len = bytes.len().to_le_bytes();

    let ptr = mem::malloc(4 + bytes.len() as u32);
    let ptr = ptr as *mut u8;

    *tx_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    let displayed = revert(&tx.hash());
    let bytes = displayed.as_bytes();

    ptr::copy_nonoverlapping(bytes.as_ptr(), hash_ptr.as_mut_ptr(), 64);

    ErrorCode::Ok
}

#[no_mangle]
pub unsafe fn moonlight_unstake(
    rng: &[u8; 32],
    seed: &Seed,
    sender_index: u8,
    unstake_value: *const u64,
    gas_limit: *const u64,
    gas_price: *const u64,
    nonce: *const u64,
    chain_id: u8,
    tx_ptr: *mut *mut u8,
    hash_ptr: &mut [u8; 64],
) -> ErrorCode {
    let mut rng = ChaCha12Rng::from_seed(*rng);

    let sender_sk = derive_bls_sk(&seed, sender_index);
    let stake_sk = sender_sk.clone();

    let transfer_value = 0;
    let deposit = 0;

    let gas_payment_token = WithdrawReplayToken::Moonlight(*nonce);

    let contract_call = crate::transaction::unstake_to_moonlight(
        &mut rng,
        &sender_sk,
        &stake_sk,
        gas_payment_token,
        *unstake_value,
    )
    .or(Err(ErrorCode::ContractCallError))?;

    let tx = crate::transaction::moonlight(
        &sender_sk,
        None,
        transfer_value,
        deposit,
        *gas_limit,
        *gas_price,
        *nonce,
        chain_id,
        Some(contract_call),
    )
    .or(Err(ErrorCode::MoonlightTransactionError))?;

    let bytes = tx.to_var_bytes();
    let len = bytes.len().to_le_bytes();

    let ptr = mem::malloc(4 + bytes.len() as u32);
    let ptr = ptr as *mut u8;

    *tx_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    let displayed = revert(&tx.hash());
    let bytes = displayed.as_bytes();

    ptr::copy_nonoverlapping(bytes.as_ptr(), hash_ptr.as_mut_ptr(), 64);

    ErrorCode::Ok
}

#[no_mangle]
pub unsafe fn moonlight_stake_reward(
    rng: &[u8; 32],
    seed: &Seed,
    sender_index: u8,
    reward_amount: *const u64,
    gas_limit: *const u64,
    gas_price: *const u64,
    nonce: *const u64,
    chain_id: u8,
    tx_ptr: *mut *mut u8,
    hash_ptr: &mut [u8; 64],
) -> ErrorCode {
    let mut rng = ChaCha12Rng::from_seed(*rng);

    let sender_sk = derive_bls_sk(&seed, sender_index);
    let stake_sk = sender_sk.clone();

    let transfer_value = 0;
    let deposit = 0;

    let gas_payment_token = WithdrawReplayToken::Moonlight(*nonce);

    let contract_call = crate::transaction::stake_reward_to_moonlight(
        &mut rng,
        &sender_sk,
        &stake_sk,
        gas_payment_token,
        *reward_amount,
    )
    .or(Err(ErrorCode::ContractCallError))?;

    let tx = crate::transaction::moonlight(
        &sender_sk,
        None,
        transfer_value,
        deposit,
        *gas_limit,
        *gas_price,
        *nonce,
        chain_id,
        Some(contract_call),
    )
    .or(Err(ErrorCode::MoonlightTransactionError))?;

    let bytes = tx.to_var_bytes();
    let len = bytes.len().to_le_bytes();

    let ptr = mem::malloc(4 + bytes.len() as u32);
    let ptr = ptr as *mut u8;

    *tx_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    let displayed = revert(&tx.hash());
    let bytes = displayed.as_bytes();

    ptr::copy_nonoverlapping(bytes.as_ptr(), hash_ptr.as_mut_ptr(), 64);

    ErrorCode::Ok
}
