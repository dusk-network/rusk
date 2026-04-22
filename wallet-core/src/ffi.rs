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
use dusk_core::BlsScalar;
use dusk_core::abi::ContractId;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::stake::{STAKE_CONTRACT, Stake};
use dusk_core::transfer::data::{
    ContractCall, ContractDeploy, TransactionData, gen_contract_id,
};
use dusk_core::transfer::moonlight::Transaction as MoonlightTransaction;
use dusk_core::transfer::phoenix::{
    ArchivedNoteLeaf, Note, NoteLeaf, NoteOpening, Prove,
    PublicKey as PhoenixPublicKey,
};
use dusk_core::transfer::withdraw::WithdrawReplayToken;
use dusk_core::transfer::{Transaction, phoenix};
use error::ErrorCode;
use rand_chacha::ChaCha12Rng;
use rand_chacha::rand_core::SeedableRng;
use rkyv::to_bytes;
use zeroize::Zeroize;

use crate::Seed;
use crate::keys::{
    derive_bls_pk, derive_bls_sk, derive_phoenix_pk, derive_phoenix_sk,
    derive_phoenix_vk,
};
use crate::notes::{self, balance, owned, pick};

#[unsafe(no_mangle)]
static KEY_SIZE: usize = BlsScalar::SIZE;
#[unsafe(no_mangle)]
static ITEM_SIZE: usize = core::mem::size_of::<ArchivedNoteLeaf>();

#[unsafe(no_mangle)]
static MINIMUM_STAKE: u64 = dusk_core::stake::DEFAULT_MINIMUM_STAKE;

#[repr(C)]
pub struct ContractIdBytes {
    bytes: [u8; 32],
}

/// The size of the scratch buffer used for parsing the notes.
const NOTES_BUFFER_SIZE: usize = 96 * 1024;

fn revert(value: &BlsScalar) -> String {
    // Unfortunately, the BlsScalar type had a display implementation that
    // does not follow the raw bytes format. Therefore the `tx.hash` display
    // DOES NOT match the `tx.hash` of the network.
    let displayed = alloc::format!("{}", &value);
    let displayed = displayed.chars().skip(2).collect::<Vec<_>>();

    displayed.chunks(2).rev().flatten().collect::<String>()
}

fn as_phoenix_transaction(
    tx: Transaction,
) -> Result<phoenix::Transaction, ErrorCode> {
    match tx {
        Transaction::Phoenix(tx) => Ok(tx),
        Transaction::Moonlight(_) => Err(ErrorCode::PhoenixTransactionError),
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn generate_profile(
    seed: &Seed,
    index: u8,
    profile: *mut [u8; PhoenixPublicKey::SIZE + BlsPublicKey::SIZE],
) -> ErrorCode {
    let ppk = derive_phoenix_pk(seed, index).to_bytes();
    let bpk = derive_bls_pk(seed, index).to_bytes();

    ptr::copy_nonoverlapping(
        &raw const ppk[0],
        &raw mut (*profile)[0],
        PhoenixPublicKey::SIZE,
    );

    ptr::copy_nonoverlapping(
        &raw const bpk[0],
        &raw mut (*profile)[PhoenixPublicKey::SIZE],
        BlsPublicKey::SIZE,
    );

    ErrorCode::Ok
}

/// Filter all notes and their block height that are owned by the given keys,
/// mapped to their nullifiers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn map_owned(
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

    for mut sk in keys {
        sk.zeroize();
    }

    let bytes = to_bytes::<_, NOTES_BUFFER_SIZE>(&owned)
        .or(Err(ErrorCode::ArchivingError))?;

    let len = bytes.len().to_le_bytes();

    let ptr_len = u32::try_from(bytes.len()).expect("bytes len to be u32");
    let ptr = mem::malloc(4 + ptr_len);
    let ptr = ptr as *mut u8;

    *owned_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());
    ptr::copy_nonoverlapping(
        block_height.to_le_bytes().as_ptr(),
        &raw mut (*last_info_ptr)[0],
        8,
    );
    ptr::copy_nonoverlapping(
        pos.to_le_bytes().as_ptr(),
        &raw mut (*last_info_ptr)[8],
        8,
    );

    ErrorCode::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn display_scalar(
    scalar_ptr: &[u8; 32],
    output: &mut [u8; 64],
) -> ErrorCode {
    let scalar: BlsScalar = mem::parse_buffer(scalar_ptr)?;
    let displayed = alloc::format!("{scalar}");
    let bytes = displayed.as_bytes();

    ptr::copy_nonoverlapping(bytes[2..].as_ptr(), output.as_mut_ptr(), 64);

    ErrorCode::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn accounts_into_raw(
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
    let ptr_len = u32::try_from(bytes.len()).expect("bytes len to be u32");
    let ptr = mem::malloc(4 + ptr_len);
    let ptr = ptr as *mut u8;

    *raws_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    ErrorCode::Ok
}

/// Calculate the balance info for the phoenix address at the given index for
/// the given seed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn balance(
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
        &raw mut (*balance_info_ptr)[0],
        16,
    );

    ErrorCode::Ok
}

/// Pick the notes to be used in a transaction from an owned notes list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pick_notes(
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bookmarks(
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

    let ptr =
        mem::malloc(u32::try_from(bytes.len()).expect("bytes len to be u32"));
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn into_proven(
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

    let ptr_len = u32::try_from(bytes.len()).expect("bytes len to be u32");
    let ptr = mem::malloc(4 + ptr_len);
    let ptr = ptr as *mut u8;

    *proven_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    let displayed = revert(&tx.hash());
    let bytes = displayed.as_bytes();

    ptr::copy_nonoverlapping(bytes.as_ptr(), hash_ptr.as_mut_ptr(), 64);

    ErrorCode::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn phoenix(
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

    let sender_sk = derive_phoenix_sk(seed, sender_index);
    let change_pk = PhoenixPublicKey::from(&sender_sk);
    let receiver_pk = PhoenixPublicKey::from_bytes(receiver)
        .or(Err(ErrorCode::DeserializationError))?;

    let root: BlsScalar = mem::parse_buffer(root)?;

    let openings: Vec<Option<NoteOpening>> = mem::from_buffer(openings)?;

    let notes: Vec<NoteLeaf> = mem::from_buffer(inputs)?;

    let inputs: Vec<(Note, NoteOpening)> = notes
        .into_iter()
        .map(|note_leaf| note_leaf.note)
        .zip(openings)
        .filter_map(|(note, opening)| opening.map(|op| (note, op)))
        .collect();

    let data: Option<TransactionData> = if data.is_null() {
        None
    } else {
        let buffer = mem::read_buffer(data);
        let transaction_data: TransactionData = mem::parse_buffer(buffer)?;
        Some(transaction_data)
    };

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

    let ptr_len = u32::try_from(bytes.len()).expect("bytes len to be u32");
    let ptr = mem::malloc(4 + ptr_len);
    let ptr = ptr as *mut u8;

    *tx_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    let bytes = prover.circuits.into_inner();
    let len = bytes.len().to_le_bytes();

    let ptr_len = u32::try_from(bytes.len()).expect("bytes len to be u32");
    let ptr = mem::malloc(4 + ptr_len);
    let ptr = ptr as *mut u8;

    *proof_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    ErrorCode::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn moonlight(
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
    let sender_sk = derive_bls_sk(seed, sender_index);

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
        let transaction_data: TransactionData = mem::parse_buffer(buffer)?;
        Some(transaction_data)
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

    let ptr_len = u32::try_from(bytes.len()).expect("bytes len to be u32");
    let ptr = mem::malloc(4 + ptr_len);
    let ptr = ptr as *mut u8;

    *tx_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    let displayed = revert(&tx.hash());
    let bytes = displayed.as_bytes();

    ptr::copy_nonoverlapping(bytes.as_ptr(), hash_ptr.as_mut_ptr(), 64);

    ErrorCode::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn phoenix_to_moonlight(
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

    let phoenix_sender_sk = derive_phoenix_sk(seed, profile_index);
    let moonlight_receiver_sk = derive_bls_sk(seed, profile_index);

    let root: BlsScalar = mem::parse_buffer(root)?;

    let openings: Vec<Option<NoteOpening>> = mem::from_buffer(openings)?;
    let nullifiers: Vec<BlsScalar> = mem::from_buffer(nullifiers)?;
    let notes: Vec<NoteLeaf> = mem::from_buffer(inputs)?;

    let inputs: Vec<(Note, NoteOpening, BlsScalar)> = notes
        .into_iter()
        .map(|note_leaf| note_leaf.note)
        .zip(openings)
        .zip(nullifiers)
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

    let ptr_len = u32::try_from(bytes.len()).expect("bytes len to be u32");
    let ptr = mem::malloc(4 + ptr_len);
    let ptr = ptr as *mut u8;

    *tx_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    let bytes = prover.circuits.into_inner();
    let len = bytes.len().to_le_bytes();

    let ptr_len = u32::try_from(bytes.len()).expect("bytes len to be u32");
    let ptr = mem::malloc(4 + ptr_len);
    let ptr = ptr as *mut u8;

    *proof_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    ErrorCode::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn moonlight_to_phoenix(
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

    let moonlight_sender_sk = derive_bls_sk(seed, profile_index);
    let phoenix_receiver_sk = derive_phoenix_sk(seed, profile_index);

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

    let ptr_len = u32::try_from(bytes.len()).expect("bytes len to be u32");
    let ptr = mem::malloc(4 + ptr_len);
    let ptr = ptr as *mut u8;

    *tx_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    let displayed = revert(&tx.hash());
    let bytes = displayed.as_bytes();

    ptr::copy_nonoverlapping(bytes.as_ptr(), hash_ptr.as_mut_ptr(), 64);

    ErrorCode::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn moonlight_stake(
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

    let sender_sk = derive_bls_sk(seed, sender_index);
    let stake_sk = sender_sk.clone();

    let stake = Stake::new(&stake_sk, &stake_sk, *stake_value, chain_id);

    let contract_call = ContractCall::new(STAKE_CONTRACT, "stake")
        .with_args(&stake)
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

    let ptr_len = u32::try_from(bytes.len()).expect("bytes len to be u32");
    let ptr = mem::malloc(4 + ptr_len);
    let ptr = ptr as *mut u8;

    *tx_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    let displayed = revert(&tx.hash());
    let bytes = displayed.as_bytes();

    ptr::copy_nonoverlapping(bytes.as_ptr(), hash_ptr.as_mut_ptr(), 64);

    ErrorCode::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn moonlight_unstake(
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

    let sender_sk = derive_bls_sk(seed, sender_index);
    let stake_sk = sender_sk.clone();

    let transfer_value = 0;
    let deposit = 0;

    let gas_payment_token = WithdrawReplayToken::Moonlight(*nonce);

    let contract_call = crate::transaction::unstake_to_moonlight(
        &mut rng,
        &sender_sk,
        &stake_sk,
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

    let ptr_len = u32::try_from(bytes.len()).expect("bytes len to be u32");
    let ptr = mem::malloc(4 + ptr_len);
    let ptr = ptr as *mut u8;

    *tx_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    let displayed = revert(&tx.hash());
    let bytes = displayed.as_bytes();

    ptr::copy_nonoverlapping(bytes.as_ptr(), hash_ptr.as_mut_ptr(), 64);

    ErrorCode::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn moonlight_stake_reward(
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

    let sender_sk = derive_bls_sk(seed, sender_index);
    let stake_sk = sender_sk.clone();

    let transfer_value = 0;
    let deposit = 0;

    let gas_payment_token = WithdrawReplayToken::Moonlight(*nonce);

    let contract_call = crate::transaction::stake_reward_to_moonlight(
        &mut rng,
        &sender_sk,
        &stake_sk,
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

    let ptr_len = u32::try_from(bytes.len()).expect("bytes len to be u32");
    let ptr = mem::malloc(4 + ptr_len);
    let ptr = ptr as *mut u8;

    *tx_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    let displayed = revert(&tx.hash());
    let bytes = displayed.as_bytes();

    ptr::copy_nonoverlapping(bytes.as_ptr(), hash_ptr.as_mut_ptr(), 64);

    ErrorCode::Ok
}

#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn create_tx_data(
    fn_name_len: *const u32,
    fn_name_buf: *mut u8,
    fn_args_len: *const u32,
    fn_args_buf: *mut u8,
    contract_id: ContractIdBytes,
    memo_len: *const u32,
    memo_buf: *mut u8,
    rkyv_ptr: *mut *mut u8,
) -> ErrorCode {
    let tx_data = if memo_len.is_null() || memo_buf.is_null() {
        let fn_name = String::from(str::from_utf8_unchecked(
            slice::from_raw_parts(fn_name_buf, *fn_name_len as usize),
        ));

        let fn_args =
            slice::from_raw_parts(fn_args_buf, *fn_args_len as usize).into();
        let contract = ContractId::from_bytes(contract_id.bytes);

        let contract_call = ContractCall {
            fn_name,
            fn_args,
            contract,
        };
        TransactionData::Call(contract_call)
    } else {
        let memo = slice::from_raw_parts(memo_buf, *memo_len as usize).into();
        TransactionData::Memo(memo)
    };
    let bytes = match rkyv::to_bytes::<_, 4096>(&tx_data) {
        Ok(v) => v.to_vec(),
        Err(_) => return ErrorCode::ArchivingError,
    };
    let len = bytes.len().to_le_bytes();

    let ptr_len = u32::try_from(bytes.len()).expect("bytes len to be u32");
    let ptr = mem::malloc(4 + ptr_len);
    let ptr = ptr as *mut u8;

    *rkyv_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    ErrorCode::Ok
}

#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn create_deploy_tx_data(
    bytecode_len: *const u32,
    bytecode_buf: *mut u8,
    owner_len: *const u32,
    owner_buf: *mut u8,
    init_args_len: *const u32,
    init_args_buf: *mut u8,
    deploy_nonce: *const u64,
    rkyv_ptr: *mut *mut u8,
) -> ErrorCode {
    if bytecode_len.is_null()
        || bytecode_buf.is_null()
        || owner_len.is_null()
        || owner_buf.is_null()
        || deploy_nonce.is_null()
    {
        return ErrorCode::DeserializationError;
    }

    let bytecode =
        slice::from_raw_parts(bytecode_buf, *bytecode_len as usize).to_vec();
    let owner = slice::from_raw_parts(owner_buf, *owner_len as usize).to_vec();
    let init_args = if init_args_len.is_null() || init_args_buf.is_null() {
        None
    } else {
        Some(
            slice::from_raw_parts(init_args_buf, *init_args_len as usize)
                .to_vec(),
        )
    };

    let tx_data = TransactionData::Deploy(ContractDeploy::new(
        bytecode,
        owner,
        init_args,
        *deploy_nonce,
    ));

    let bytes = match rkyv::to_bytes::<_, 4096>(&tx_data) {
        Ok(v) => v.to_vec(),
        Err(_) => return ErrorCode::ArchivingError,
    };
    let len = bytes.len().to_le_bytes();

    let ptr_len = u32::try_from(bytes.len()).expect("bytes len to be u32");
    let ptr = mem::malloc(4 + ptr_len);
    let ptr = ptr as *mut u8;

    *rkyv_ptr = ptr;

    ptr::copy_nonoverlapping(len.as_ptr(), ptr, 4);
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());

    ErrorCode::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn contract_id(
    bytecode_len: *const u32,
    bytecode_buf: *mut u8,
    owner_len: *const u32,
    owner_buf: *mut u8,
    deploy_nonce: *const u64,
    contract_id_ptr: *mut [u8; 32],
) -> ErrorCode {
    if bytecode_len.is_null()
        || bytecode_buf.is_null()
        || owner_len.is_null()
        || owner_buf.is_null()
        || deploy_nonce.is_null()
    {
        return ErrorCode::DeserializationError;
    }

    let bytecode = slice::from_raw_parts(bytecode_buf, *bytecode_len as usize);
    let owner = slice::from_raw_parts(owner_buf, *owner_len as usize);

    let bytes = gen_contract_id(bytecode, *deploy_nonce, owner);
    ptr::copy_nonoverlapping(
        bytes.as_bytes().as_ptr(),
        &raw mut (*contract_id_ptr)[0],
        32,
    );

    ErrorCode::Ok
}
