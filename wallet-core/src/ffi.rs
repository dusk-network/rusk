// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The foreign function interface for the wallet.

use alloc::vec::Vec;

use core::mem;
use core::num::NonZeroU32;
use core::ptr;

use canonical::{Canon, Source};
use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::Write;
use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_jubjub::{BlsScalar, JubJubAffine, JubJubScalar};
use dusk_pki::{PublicSpendKey, ViewKey};
use dusk_plonk::prelude::Proof;
use dusk_poseidon::tree::PoseidonBranch;
use dusk_schnorr::Signature;
use phoenix_core::{Crossover, Fee, Note};
use rand_core::{
    impls::{next_u32_via_fill, next_u64_via_fill},
    CryptoRng, RngCore,
};

use crate::tx::UnprovenTransaction;
use crate::{
    Error, ProverClient, StakeInfo, StateClient, Store, Transaction, Wallet,
    POSEIDON_TREE_DEPTH,
};

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

    /// Asks the node to find the nullifiers that are already in the state and
    /// returns them.
    ///
    /// The nullifiers are to be serialized in sequence and written to
    /// `existing_nullifiers` and their number should be written to
    /// `existing_nullifiers_len`.
    fn fetch_existing_nullifiers(
        nullifiers: *const u8,
        nullifiers_len: u32,
        existing_nullifiers: *mut u8,
        existing_nullifiers_len: *mut u32,
    ) -> u8;

    /// Fetches the current anchor.
    fn fetch_anchor(anchor: *mut [u8; BlsScalar::SIZE]) -> u8;

    /// Fetches the current stake for a key.
    ///
    /// The value, eligibility, and created_at should be written in sequence,
    /// little endian, to the given buffer.
    fn fetch_stake(
        pk: *const [u8; PublicKey::SIZE],
        stake: *mut [u8; StakeInfo::SIZE],
    ) -> u8;

    /// Fetches the current block height from the node.
    fn fetch_block_height(height: &mut u64) -> u8;

    /// Request the node to prove the given unproven transaction.
    fn compute_proof_and_propagate(
        utx: *const u8,
        utx_len: u32,
        tx: *mut u8,
        tx_len: *mut u32,
    ) -> u8;

    /// Requests the node to prove STCT.
    fn request_stct_proof(
        inputs: *const [u8; STCT_INPUT_SIZE],
        proof: *mut [u8; Proof::SIZE],
    ) -> u8;

    /// Request the node to prove WFCT.
    fn request_wfct_proof(
        inputs: *const [u8; WFCT_INPUT_SIZE],
        proof: *mut [u8; Proof::SIZE],
    ) -> u8;
}

macro_rules! unwrap_or_bail {
    ($e: expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                return Error::<FfiStore, FfiStateClient, FfiProverClient>::from(e).into();
            }
        }
    };
}

type FfiWallet = Wallet<FfiStore, FfiStateClient, FfiProverClient>;
const WALLET: FfiWallet =
    Wallet::new(FfiStore, FfiStateClient, FfiProverClient);

/// Allocates memory with a given size.
#[no_mangle]
pub unsafe extern "C" fn malloc(cap: u32) -> *mut u8 {
    let mut buf = Vec::with_capacity(cap as usize);
    let ptr = buf.as_mut_ptr();
    mem::forget(buf);
    ptr
}

/// Free memory pointed to by the given `ptr`, and the given `cap`acity.
#[no_mangle]
pub unsafe extern "C" fn free(ptr: *mut u8, cap: u32) {
    Vec::from_raw_parts(ptr, 0, cap as usize);
}

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
pub unsafe extern "C" fn transfer(
    sender_index: u64,
    refund: *const [u8; PublicSpendKey::SIZE],
    receiver: *const [u8; PublicSpendKey::SIZE],
    value: u64,
    gas_limit: u64,
    gas_price: u64,
    ref_id: Option<&u64>,
) -> u8 {
    let refund = unwrap_or_bail!(PublicSpendKey::from_bytes(&*refund));
    let receiver = unwrap_or_bail!(PublicSpendKey::from_bytes(&*receiver));

    let ref_id = BlsScalar::from(
        ref_id.copied().unwrap_or_else(|| (&mut FfiRng).next_u64()),
    );

    unwrap_or_bail!(WALLET.transfer(
        &mut FfiRng,
        sender_index,
        &refund,
        &receiver,
        value,
        gas_price,
        gas_limit,
        ref_id
    ));

    0
}

/// Creates a stake transaction.
#[no_mangle]
pub unsafe extern "C" fn stake(
    sender_index: u64,
    staker_index: u64,
    refund: *const [u8; PublicSpendKey::SIZE],
    value: u64,
    gas_limit: u64,
    gas_price: u64,
) -> u8 {
    let refund = unwrap_or_bail!(PublicSpendKey::from_bytes(&*refund));

    unwrap_or_bail!(WALLET.stake(
        &mut FfiRng,
        sender_index,
        staker_index,
        &refund,
        value,
        gas_price,
        gas_limit
    ));

    0
}

/// Withdraw a key's stake.
#[no_mangle]
pub unsafe extern "C" fn withdraw_stake(
    sender_index: u64,
    staker_index: u64,
    refund: *const [u8; PublicSpendKey::SIZE],
    gas_limit: u64,
    gas_price: u64,
) -> u8 {
    let refund = unwrap_or_bail!(PublicSpendKey::from_bytes(&*refund));

    unwrap_or_bail!(WALLET.withdraw_stake(
        &mut FfiRng,
        sender_index,
        staker_index,
        &refund,
        gas_price,
        gas_limit
    ));

    0
}

/// Gets the balance of a secret spend key.
#[no_mangle]
pub unsafe extern "C" fn get_balance(ssk_index: u64, balance: *mut u64) -> u8 {
    *balance = unwrap_or_bail!(WALLET.get_balance(ssk_index));
    0
}

/// Gets the stake of a key. The value, eligibility, and created_at are written
/// in sequence to the given buffer.
#[no_mangle]
pub unsafe extern "C" fn get_stake(
    sk_index: u64,
    stake: *mut [u8; StakeInfo::SIZE],
) -> u8 {
    let s = unwrap_or_bail!(WALLET.get_stake(sk_index)).to_bytes();
    ptr::copy_nonoverlapping(&s[0], &mut (*stake)[0], s.len());
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

const STCT_INPUT_SIZE: usize = Fee::SIZE
    + Crossover::SIZE
    + u64::SIZE
    + JubJubScalar::SIZE
    + BlsScalar::SIZE
    + Signature::SIZE;

const WFCT_INPUT_SIZE: usize =
    JubJubAffine::SIZE + u64::SIZE + JubJubScalar::SIZE;

struct FfiStateClient;

impl StateClient for FfiStateClient {
    type Error = u8;

    fn fetch_notes(
        &self,
        height: u64,
        vk: &ViewKey,
    ) -> Result<Vec<Note>, Self::Error> {
        let mut notes_buf = vec![0u8; NOTES_BUF_SIZE];

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
            notes.push(Note::from_reader(&mut buf).map_err(
                Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
            )?);
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
        let scalar = BlsScalar::from_bytes(&scalar_buf).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;

        Ok(scalar)
    }

    fn fetch_existing_nullifiers(
        &self,
        nullifiers: &[BlsScalar],
    ) -> Result<Vec<BlsScalar>, Self::Error> {
        let nullifiers_len = nullifiers.len();
        let mut nullifiers_buf = vec![0u8; BlsScalar::SIZE * nullifiers_len];

        let mut writer = &mut nullifiers_buf[..];

        for nullifier in nullifiers {
            writer.write(&nullifier.to_bytes()).map_err(
                Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
            )?;
        }

        let mut existing_nullifiers_buf =
            vec![0u8; BlsScalar::SIZE * nullifiers_len];
        let mut existing_nullifiers_len = 0;

        unsafe {
            let r = fetch_existing_nullifiers(
                &nullifiers_buf[0],
                nullifiers_len as u32,
                &mut existing_nullifiers_buf[0],
                &mut existing_nullifiers_len,
            );
            if r != 0 {
                return Err(r);
            }
        };

        let mut existing_nullifiers =
            Vec::with_capacity(existing_nullifiers_len as usize);

        let mut reader = &existing_nullifiers_buf[..];
        for _ in 0..existing_nullifiers_len {
            existing_nullifiers.push(
                BlsScalar::from_reader(&mut reader).map_err(
                    Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
                )?,
            );
        }

        Ok(existing_nullifiers)
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
        let branch = PoseidonBranch::decode(&mut source).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;

        Ok(branch)
    }

    fn fetch_stake(&self, pk: &PublicKey) -> Result<StakeInfo, Self::Error> {
        let pk = pk.to_bytes();
        let mut stake_buf = [0u8; StakeInfo::SIZE];

        unsafe {
            let r = fetch_stake(&pk, &mut stake_buf);
            if r != 0 {
                return Err(r);
            }
        }

        let stake = StakeInfo::from_bytes(&stake_buf).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;

        Ok(stake)
    }

    fn fetch_block_height(&self) -> Result<u64, Self::Error> {
        let mut block_height = 0;

        unsafe {
            let r = fetch_block_height(&mut block_height);
            if r != 0 {
                return Err(r);
            }
        }

        Ok(block_height)
    }
}

struct FfiProverClient;

impl ProverClient for FfiProverClient {
    type Error = u8;

    fn compute_proof_and_propagate(
        &self,
        utx: &UnprovenTransaction,
    ) -> Result<Transaction, Self::Error> {
        let utx_bytes = utx.to_var_bytes();

        // A transaction is always smaller than an unproven transaction
        let mut tx_buf = vec![0; utx_bytes.len()];
        let mut tx_len = 0;

        unsafe {
            let r = compute_proof_and_propagate(
                &utx_bytes[0],
                utx_bytes.len() as u32,
                &mut tx_buf[0],
                &mut tx_len,
            );
            if r != 0 {
                return Err(r);
            }
        }

        let transaction = Transaction::from_slice(&tx_buf[..tx_len as usize])
            .map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;

        Ok(transaction)
    }

    fn request_stct_proof(
        &self,
        fee: &Fee,
        crossover: &Crossover,
        value: u64,
        blinder: JubJubScalar,
        address: BlsScalar,
        signature: Signature,
    ) -> Result<Proof, Self::Error> {
        let mut buf = [0; STCT_INPUT_SIZE];

        let mut writer = &mut buf[..];
        writer.write(&fee.to_bytes()).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;
        writer.write(&crossover.to_bytes()).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;
        writer.write(&value.to_bytes()).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;
        writer.write(&blinder.to_bytes()).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;
        writer.write(&address.to_bytes()).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;
        writer.write(&signature.to_bytes()).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;

        let mut proof_buf = [0; Proof::SIZE];

        unsafe {
            let r = request_stct_proof(&buf, &mut proof_buf);
            if r != 0 {
                return Err(r);
            }
        }

        let proof = Proof::from_bytes(&proof_buf).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;
        Ok(proof)
    }

    fn request_wfct_proof(
        &self,
        commitment: JubJubAffine,
        value: u64,
        blinder: JubJubScalar,
    ) -> Result<Proof, Self::Error> {
        let mut buf = [0; WFCT_INPUT_SIZE];

        let mut writer = &mut buf[..];
        writer.write(&commitment.to_bytes()).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;
        writer.write(&value.to_bytes()).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;
        writer.write(&blinder.to_bytes()).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;

        let mut proof_buf = [0; Proof::SIZE];

        unsafe {
            let r = request_wfct_proof(&buf, &mut proof_buf);
            if r != 0 {
                return Err(r);
            }
        }

        let proof = Proof::from_bytes(&proof_buf).map_err(
            Error::<FfiStore, FfiStateClient, FfiProverClient>::from,
        )?;
        Ok(proof)
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

impl<S: Store, SC: StateClient, PC: ProverClient> From<Error<S, SC, PC>>
    for u8
{
    fn from(e: Error<S, SC, PC>) -> Self {
        match e {
            Error::Store(_) => 255,
            Error::Rng(_) => 254,
            Error::Bytes(_) => 253,
            Error::State(_) => 252,
            Error::Prover(_) => 251,
            Error::NotEnoughBalance => 250,
            Error::NoteCombinationProblem => 249,
            Error::Canon(_) => 248,
            Error::Phoenix(_) => 247,
        }
    }
}
