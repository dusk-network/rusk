// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{NoteFinder, Store};

use alloc::vec::Vec;

use dusk_bytes::Error as BytesError;
use dusk_jubjub::{BlsScalar, JubJubScalar};
use dusk_pki::{PublicSpendKey, SecretSpendKey};
use phoenix_core::{Note, NoteType};
use rand_core::{CryptoRng, Error as RngError, RngCore};

const MAX_INPUT_NOTES: usize = 0x4;

/// The error type returned by this crate.
pub enum Error<S: Store, NF: NoteFinder> {
    /// Underlying store error.
    Store(S::Error),
    /// Find notes error.
    FindNotes(NF::Error),
    /// Random number generator error.
    Rng(RngError),
    /// Serialization and deserialization of Dusk types.
    Bytes(BytesError),
    /// The key with the given ID does not exist.
    NoSuchKey(S::Id),
    /// Not enough balance to perform transaction.
    NotEnoughBalance,
    /// Note combination for the given value is impossible given the maximum
    /// amount if inputs in a transaction.
    NoteCombinationProblem,
}

impl<S: Store, NF: NoteFinder> Error<S, NF> {
    /// Returns an error from the underlying store error.
    pub fn from_store_err(se: S::Error) -> Self {
        Self::Store(se)
    }
    /// Returns an error from the underlying note finder error.
    pub fn from_note_finder_err(nfe: NF::Error) -> Self {
        Self::FindNotes(nfe)
    }
}

impl<S: Store, NF: NoteFinder> From<RngError> for Error<S, NF> {
    fn from(re: RngError) -> Self {
        Self::Rng(re)
    }
}

impl<S: Store, NF: NoteFinder> From<BytesError> for Error<S, NF> {
    fn from(be: BytesError) -> Self {
        Self::Bytes(be)
    }
}

/// A wallet implementation.
///
/// This is responsible for holding the keys, and performing operations like
/// creating transactions.
pub struct Wallet<S, NF> {
    store: S,
    nf: NF,
}

impl<S, NF> Wallet<S, NF> {
    /// Creates a new wallet with the given backing store.
    pub const fn new(store: S, note_finder: NF) -> Self {
        Self {
            store,
            nf: note_finder,
        }
    }
}

#[allow(clippy::too_many_arguments)]
impl<S: Store, NF: NoteFinder> Wallet<S, NF>
where
    S::Id: Clone,
{
    /// Create a secret spend key.
    pub fn create_ssk<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        id: &S::Id,
    ) -> Result<(), Error<S, NF>> {
        let ssk = SecretSpendKey::random(rng);
        self.load_ssk(id, &ssk)
    }

    /// Loads a secret spend key into the wallet.
    pub fn load_ssk(
        &self,
        id: &S::Id,
        ssk: &SecretSpendKey,
    ) -> Result<(), Error<S, NF>> {
        self.store
            .store_key(id, ssk)
            .map_err(Error::from_store_err)?;
        Ok(())
    }

    /// Creates a transfer transaction.
    pub fn create_transfer_tx<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender: &S::Id,
        receiver: &PublicSpendKey,
        value: u64,
        gas_price: u64,
        gas_limit: u64,
        ref_id: BlsScalar,
    ) -> Result<(), Error<S, NF>> {
        let sender = self
            .store
            .key(sender)
            .map_err(Error::from_store_err)?
            .ok_or_else(|| Error::NoSuchKey(sender.clone()))?;
        let sender_psk = sender.public_spend_key();

        let inputs = {
            let sender_vk = sender.view_key();

            // TODO find a way to get the block height from somewhere
            let mut notes = self
                .nf
                .find_notes(0, &sender_vk)
                .map_err(Error::from_note_finder_err)?;
            let mut notes_and_values = Vec::with_capacity(notes.len());

            let mut accumulated_value = 0;
            for note in notes.drain(..) {
                let val = note.value(Some(&sender_vk)).unwrap();
                accumulated_value += val;
                notes_and_values.push((note, val));
            }

            if accumulated_value < value {
                return Err(Error::NotEnoughBalance);
            }

            // This sorts the notes from least valuable to most valuable. It
            // helps in the minimum gas spent algorithm, where the largest notes
            // are "popped" first.
            notes_and_values.sort_by(|(_, aval), (_, bval)| aval.cmp(bval));

            let mut input_notes = Vec::with_capacity(notes.len());

            let mut accumulated_value = 0;
            while accumulated_value < value {
                // This unwrap is ok because at this point we can be sure there
                // is enough value in the notes.
                let (note, val) = notes_and_values.pop().unwrap();
                accumulated_value += val;
                input_notes.push(note.gen_nullifier(&sender));
            }

            if input_notes.len() > MAX_INPUT_NOTES {
                return Err(Error::NoteCombinationProblem);
            }

            input_notes
        };

        let outputs = vec![
            // receiver note
            generate_obfuscated_note(rng, receiver, value, ref_id),
            // refund/fee note
            generate_obfuscated_note(
                rng,
                &sender_psk,
                gas_limit * gas_price,
                ref_id,
            ),
        ];

        Ok(())
    }

    /// Creates a stake transaction.
    pub fn create_stake_tx(&self, id: &S::Id) -> Result<(), Error<S, NF>> {
        todo!()
    }

    /// Stops staking for a key.
    pub fn stop_stake(&self) -> Result<(), Error<S, NF>> {
        todo!()
    }

    /// Extends staking for a particular key.
    pub fn extend_stake(&self) -> Result<(), Error<S, NF>> {
        todo!()
    }

    /// Withdraw a key's stake.
    pub fn withdraw_stake(&self) -> Result<(), Error<S, NF>> {
        todo!()
    }

    /// Syncs the wallet with the blocks.
    pub fn sync(&self) -> Result<(), Error<S, NF>> {
        todo!()
    }

    /// Gets the balance of a key.
    pub fn get_balance(&self) -> Result<(), Error<S, NF>> {
        todo!()
    }
}

/// Generates an obfuscated note for the given public spend key.
fn generate_obfuscated_note<Rng: RngCore + CryptoRng>(
    rng: &mut Rng,
    psk: &PublicSpendKey,
    value: u64,
    nonce: BlsScalar,
) -> Note {
    let r = JubJubScalar::random(rng);
    let blinding_factor = JubJubScalar::random(rng);

    Note::deterministic(
        NoteType::Obfuscated,
        &r,
        nonce,
        psk,
        value,
        blinding_factor,
    )
}
