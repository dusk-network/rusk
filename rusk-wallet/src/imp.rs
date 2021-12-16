// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{NodeClient, Store, Transaction, POSEIDON_BRANCH_DEPTH};

use alloc::vec::Vec;
use canonical::CanonError;

use crate::tx::{TransactionSkeleton, UnprovenTransactionInput};
use dusk_bytes::Error as BytesError;
use dusk_jubjub::{BlsScalar, JubJubScalar};
use dusk_pki::{PublicSpendKey, SecretSpendKey};
use dusk_poseidon::tree::PoseidonBranch;
use phoenix_core::{Crossover, Fee, Note, NoteType};
use rand_chacha::ChaCha12Rng;
use rand_core::{CryptoRng, Error as RngError, RngCore, SeedableRng};
use sha2::{Digest, Sha256};

const MAX_INPUT_NOTES: usize = 0x4;

/// The error type returned by this crate.
pub enum Error<S: Store, C: NodeClient> {
    /// Underlying store error.
    Store(S::Error),
    /// Error originating from the node client.
    Node(C::Error),
    /// Canonical stores.
    Canon(CanonError),
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

impl<S: Store, C: NodeClient> Error<S, C> {
    /// Returns an error from the underlying store error.
    pub fn from_store_err(se: S::Error) -> Self {
        Self::Store(se)
    }
    /// Returns an error from the underlying note finder error.
    pub fn from_node_err(ne: C::Error) -> Self {
        Self::Node(ne)
    }
}

impl<S: Store, C: NodeClient> From<RngError> for Error<S, C> {
    fn from(re: RngError) -> Self {
        Self::Rng(re)
    }
}

impl<S: Store, C: NodeClient> From<BytesError> for Error<S, C> {
    fn from(be: BytesError) -> Self {
        Self::Bytes(be)
    }
}

impl<S: Store, C: NodeClient> From<CanonError> for Error<S, C> {
    fn from(ce: CanonError) -> Self {
        Self::Canon(ce)
    }
}

/// A wallet implementation.
///
/// This is responsible for holding the keys, and performing operations like
/// creating transactions.
pub struct Wallet<S, C> {
    store: S,
    client: C,
}

impl<S, C> Wallet<S, C> {
    /// Creates a new wallet with the given backing store.
    pub const fn new(store: S, client: C) -> Self {
        Self { store, client }
    }
}

#[allow(clippy::too_many_arguments)]
impl<S: Store, C: NodeClient> Wallet<S, C>
where
    S::Id: Clone,
{
    /// Create a secret spend key given a seed and a store ID.
    ///
    /// This creates a key based on the number of keys that are already in the
    /// store. Calling this function with different seeds for the same store is
    /// heavily discouraged.
    pub fn create_secret_spend_key<B: AsRef<[u8]>>(
        &self,
        id: &S::Id,
        seed: B,
    ) -> Result<(), Error<S, C>> {
        let key_num = self.store.key_num().map_err(Error::from_store_err)?;

        let mut sha_256 = Sha256::new();
        sha_256.update(seed);
        sha_256.update(&(key_num as u32).to_le_bytes());
        let hash = sha_256.finalize();

        let mut rng = ChaCha12Rng::from_seed(hash.into());

        let ssk = SecretSpendKey::random(&mut rng);

        self.store
            .store_key(id, &ssk)
            .map_err(Error::from_store_err)?;
        Ok(())
    }

    /// Get the public spend key with the given ID.
    pub fn get_public_spend_key(
        &self,
        id: &S::Id,
    ) -> Result<PublicSpendKey, Error<S, C>> {
        self.store
            .key(id)
            .map(|ssk| ssk.public_spend_key())
            .map_err(Error::from_store_err)
    }

    /// Creates a transfer transaction.
    pub fn create_transfer_tx<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender: &S::Id,
        receiver: &PublicSpendKey,
        value: u64,
        gas_limit: u64,
        gas_price: u64,
        ref_id: BlsScalar,
    ) -> Result<Transaction, Error<S, C>> {
        let sender = self.store.key(sender).map_err(Error::from_store_err)?;
        let sender_psk = sender.public_spend_key();

        let input_notes = {
            let sender_vk = sender.view_key();

            // TODO find a way to get the block height from somewhere
            let mut notes = self
                .client
                .fetch_notes(0, &sender_vk)
                .map_err(Error::from_node_err)?;
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
                input_notes.push(note);
            }

            if input_notes.len() > MAX_INPUT_NOTES {
                return Err(Error::NoteCombinationProblem);
            }

            input_notes
        };

        let nullifiers: Vec<BlsScalar> = input_notes
            .iter()
            .map(|note| note.gen_nullifier(&sender))
            .collect();

        let mut openings = Vec::with_capacity(input_notes.len());
        for note in &input_notes {
            let opening = self
                .client
                .fetch_opening(note)
                .map_err(Error::from_node_err)?;
            openings.push(opening);
        }

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

        let anchor =
            self.client.fetch_anchor().map_err(Error::from_node_err)?;
        let fee = Fee::new(rng, gas_limit, gas_price, &sender_psk);

        let skel = TransactionSkeleton::new(
            nullifiers, outputs, anchor, fee, None, None,
        );
        let hash = skel.hash();

        let inputs: Vec<UnprovenTransactionInput> = input_notes
            .into_iter()
            .zip(openings.into_iter())
            .map(|(note, opening)| {
                UnprovenTransactionInput::new(rng, &sender, note, opening, hash)
            })
            .collect();

        todo!()
    }

    /// Creates a stake transaction.
    pub fn create_stake_tx(&self, id: &S::Id) -> Result<(), Error<S, C>> {
        todo!()
    }

    /// Stops staking for a key.
    pub fn stop_stake(&self) -> Result<(), Error<S, C>> {
        todo!()
    }

    /// Extends staking for a particular key.
    pub fn extend_stake(&self) -> Result<(), Error<S, C>> {
        todo!()
    }

    /// Withdraw a key's stake.
    pub fn withdraw_stake(&self) -> Result<(), Error<S, C>> {
        todo!()
    }

    /// Syncs the wallet with the blocks.
    pub fn sync(&self) -> Result<(), Error<S, C>> {
        todo!()
    }

    /// Gets the balance of a key.
    pub fn get_balance(&self) -> Result<(), Error<S, C>> {
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
