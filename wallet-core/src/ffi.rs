// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The foreign function interface for the wallet.

use crate::POSEIDON_TREE_DEPTH;

use alloc::vec::Vec;
use core::num::NonZeroU32;
use core::ptr;

use canonical::{Canon, Source};
use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_jubjub::BlsScalar;
use dusk_pki::{PublicSpendKey, ViewKey};
use dusk_plonk::prelude::Proof;
use dusk_poseidon::tree::PoseidonBranch;
use phoenix_core::Note;
use rand_core::{
    impls::{next_u32_via_fill, next_u64_via_fill},
    CryptoRng, RngCore,
};

use crate::tx::UnprovenTransaction;
use crate::{Error, NodeClient, Store, Wallet};

extern "C" {
    /// Retrieves the seed from the store.
    fn get_seed(seed: *mut [u8; 64]) -> u8;

    /// Fills a buffer with random numbers.
    fn fill_random(buf: *mut u8, buf_len: u32) -> u8;

    /// Asks the node to finds the notes for a specific view key, starting from
    /// a certain height.
    ///
    /// The notes are to be serialized in sequence and written to `notes`, and
    /// the number of notes written should be put in `notes_len`.
    fn fetch_notes(
        height: u64,
        vk: *const [u8; ViewKey::SIZE],
        notes: *mut u8,
        notes_len: *mut u32,
    ) -> u8;

    /// Queries the node to find the opening for a specific note.
    fn fetch_opening(
        note: *const [u8; Note::SIZE],
        opening: *mut u8,
        opening_len: *mut u32,
    ) -> u8;

    /// Fetches the current anchor.
    fn fetch_anchor(anchor: *mut [u8; BlsScalar::SIZE]) -> u8;

    /// Request the node to prove the given unproven transaction.
    fn request_proof(
        utx: *const u8,
        utx_len: u32,
        proof: *mut [u8; Proof::SIZE],
    ) -> u8;
}

macro_rules! unwrap_or_bail {
    ($e: expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                return Error::<FfiStore, FfiNodeClient>::from(e).into();
            }
        }
    };
}

type FfiWallet = Wallet<FfiStore, FfiNodeClient>;
const WALLET: FfiWallet = Wallet::new(FfiStore, FfiNodeClient);

/// Get the public spend key with the given index.
#[no_mangle]
pub unsafe extern "C" fn public_spend_key(
    index: u64,
    psk: *mut [u8; PublicSpendKey::SIZE],
) -> u8 {
    let key = unwrap_or_bail!(WALLET.public_spend_key(index)).to_bytes();
    ptr::copy_nonoverlapping(&key[0], &mut (*psk)[0], key.len());
    0
}

/// Creates a transfer transaction.
#[no_mangle]
pub unsafe extern "C" fn create_transfer_tx(
    sender_index: u64,
    refund: *const [u8; PublicSpendKey::SIZE],
    receiver: *const [u8; PublicSpendKey::SIZE],
    value: u64,
    gas_limit: u64,
    gas_price: u64,
    ref_id: Option<&u64>,
    tx_buf: *mut u8,
    tx_len: *mut u32,
) -> u8 {
    let refund = unwrap_or_bail!(PublicSpendKey::from_bytes(&*refund));
    let receiver = unwrap_or_bail!(PublicSpendKey::from_bytes(&*receiver));

    let ref_id = BlsScalar::from(
        ref_id.copied().unwrap_or_else(|| (&mut FfiRng).next_u64()),
    );

    let tx = unwrap_or_bail!(WALLET.create_transfer_tx(
        &mut FfiRng,
        sender_index,
        &refund,
        &receiver,
        value,
        gas_price,
        gas_limit,
        ref_id
    ));

    let tx_bytes = unwrap_or_bail!(tx.to_bytes());
    ptr::copy_nonoverlapping(&tx_bytes[0], tx_buf, tx_bytes.len());
    *tx_len = tx_bytes.len() as u32;

    0
}

/// Creates a stake transaction.
#[no_mangle]
pub unsafe extern "C" fn create_stake_tx() {
    unimplemented!()
}

/// Stops staking for a key.
#[no_mangle]
pub unsafe extern "C" fn stop_stake() {
    unimplemented!()
}

/// Extends staking for a particular key.
#[no_mangle]
pub unsafe extern "C" fn extend_stake() {
    unimplemented!()
}

/// Withdraw a key's stake.
#[no_mangle]
pub unsafe extern "C" fn withdraw_stake() {
    unimplemented!()
}

/// Syncs the wallet with the blocks.
#[no_mangle]
pub unsafe extern "C" fn sync_blocks() {
    unimplemented!()
}

/// Gets the balance of a key.
#[no_mangle]
pub unsafe extern "C" fn get_balance(key_index: u64, balance: *mut u64) -> u8 {
    *balance = unwrap_or_bail!(WALLET.get_balance(key_index));
    0
}

struct FfiStore;

impl Store for FfiStore {
    type Error = u8;

    fn get_seed(&self) -> Result<[u8; 64], Self::Error> {
        let mut seed = [0; 64];
        unsafe {
            let r = get_seed(&mut seed);
            if r != 0 {
                return Err(r);
            }
        }
        Ok(seed)
    }
}

// 1 MB for a buffer.
const NOTES_BUF_SIZE: usize = 0x100000;
// 512 KB for a buffer.
const OPENING_BUF_SIZE: usize = 0x10000;

struct FfiNodeClient;

impl NodeClient for FfiNodeClient {
    type Error = u8;

    fn fetch_notes(
        &self,
        height: u64,
        vk: &ViewKey,
    ) -> Result<Vec<Note>, Self::Error> {
        let mut notes_buf = [0u8; NOTES_BUF_SIZE];

        let mut num_notes = 0;

        unsafe {
            let r = fetch_notes(
                height,
                &vk.to_bytes(),
                &mut notes_buf[0],
                &mut num_notes,
            );
            if r != 0 {
                return Err(r);
            }
        };

        let mut notes = Vec::with_capacity(num_notes as usize);

        let mut buf = &notes_buf[..];
        for _ in 0..num_notes {
            notes.push(
                Note::from_reader(&mut buf)
                    .map_err(Error::<FfiStore, FfiNodeClient>::from)?,
            );
        }

        Ok(notes)
    }

    fn fetch_anchor(&self) -> Result<BlsScalar, Self::Error> {
        let mut scalar_buf = [0; BlsScalar::SIZE];
        unsafe {
            let r = fetch_anchor(&mut scalar_buf);
            if r != 0 {
                return Err(r);
            }
        }
        let scalar = BlsScalar::from_bytes(&scalar_buf)
            .map_err(Error::<FfiStore, FfiNodeClient>::from)?;

        Ok(scalar)
    }

    fn fetch_opening(
        &self,
        note: &Note,
    ) -> Result<PoseidonBranch<POSEIDON_TREE_DEPTH>, Self::Error> {
        let mut opening_buf = [0u8; OPENING_BUF_SIZE];

        let mut opening_len = 0;

        let note = note.to_bytes();
        unsafe {
            let r = fetch_opening(&note, &mut opening_buf[0], &mut opening_len);
            if r != 0 {
                return Err(r);
            }
        }

        let mut source = Source::new(&opening_buf[..opening_len as usize]);
        let branch = PoseidonBranch::decode(&mut source)
            .map_err(Error::<FfiStore, FfiNodeClient>::from)?;

        Ok(branch)
    }

    fn request_proof(
        &self,
        utx: &UnprovenTransaction,
    ) -> Result<Proof, Self::Error> {
        let utx_bytes = utx
            .to_bytes()
            .map_err(Error::<FfiStore, FfiNodeClient>::from)?;
        let mut proof_buf = [0; Proof::SIZE];

        unsafe {
            let r = request_proof(
                &utx_bytes[0],
                utx_bytes.len() as u32,
                &mut proof_buf,
            );
            if r != 0 {
                return Err(r);
            }
        }

        let utx = Proof::from_bytes(&proof_buf)
            .map_err(Error::<FfiStore, FfiNodeClient>::from)?;
        Ok(utx)
    }
}

struct FfiRng;

impl CryptoRng for FfiRng {}

impl RngCore for FfiRng {
    fn next_u32(&mut self) -> u32 {
        next_u32_via_fill(self)
    }

    fn next_u64(&mut self) -> u64 {
        next_u64_via_fill(self)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.try_fill_bytes(dest).ok();
    }

    fn try_fill_bytes(
        &mut self,
        dest: &mut [u8],
    ) -> Result<(), rand_core::Error> {
        let buf = dest.as_mut_ptr();
        let len = dest.len();

        // SAFETY: this is unsafe since the passed function is not guaranteed to
        // be a CSPRNG running in a secure context. We therefore consider it the
        // responsibility of the user to pass a good generator.
        unsafe {
            match fill_random(buf, len as u32) {
                0 => Ok(()),
                v => {
                    let nzu = NonZeroU32::new(v as u32).unwrap();
                    Err(rand_core::Error::from(nzu))
                }
            }
        }
    }
}

impl<S: Store, C: NodeClient> From<Error<S, C>> for u8 {
    fn from(e: Error<S, C>) -> Self {
        match e {
            Error::Store(_) => 255,
            Error::Rng(_) => 254,
            Error::Bytes(_) => 253,
            Error::Node(_) => 252,
            Error::NotEnoughBalance => 251,
            Error::NoteCombinationProblem => 250,
            Error::Canon(_) => 249,
            Error::Phoenix(_) => 248,
        }
    }
}
