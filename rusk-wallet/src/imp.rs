// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{NodeClient, Store, Transaction};

use alloc::vec::Vec;
use canonical::CanonError;
use core::marker::{PhantomData, PhantomPinned};

use crate::tx::{
    TransactionSkeleton, UnprovenTransaction, UnprovenTransactionInput,
};
use bip39::Mnemonic;
use dusk_bytes::Error as BytesError;
use dusk_jubjub::{BlsScalar, JubJubScalar};
use dusk_pki::{PublicSpendKey, SecretSpendKey};
use phoenix_core::{Crossover, Error as PhoenixError, Fee, Note, NoteType};
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
    /// Originating from the transaction model.
    Phoenix(PhoenixError),
    /// The key with the given ID does not exist.
    NoSuchKey(S::Id),
    /// Not enough balance to perform transaction.
    NotEnoughBalance,
    /// Note combination for the given value is impossible given the maximum
    /// amount if inputs in a transaction.
    NoteCombinationProblem,
    /// Error generating or manipulating the mnemonic.
    Bip39(bip39::Error),
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

impl<S: Store, C: NodeClient> From<PhoenixError> for Error<S, C> {
    fn from(pe: PhoenixError) -> Self {
        Self::Phoenix(pe)
    }
}

impl<S: Store, C: NodeClient> From<CanonError> for Error<S, C> {
    fn from(ce: CanonError) -> Self {
        Self::Canon(ce)
    }
}

impl<S: Store, C: NodeClient> From<bip39::Error> for Error<S, C> {
    fn from(be: bip39::Error) -> Self {
        Self::Bip39(be)
    }
}

/// The language of a mnemonic.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Language {
    /// English language mnemonic.
    English,
    /// Simplified chinese mnemonic.
    SimplifiedChinese,
    /// Traditional chinese mnemonic.
    TraditionalChinese,
    /// Czech mnemonic.
    Czech,
    /// French mnemonic.
    French,
    /// Italian mnemonic.
    Italian,
    /// Japanese mnemonic.
    Japanese,
    /// Korean mnemonic.
    Korean,
    /// Spanish mnemonic.
    Spanish,
}

impl From<Language> for bip39::Language {
    fn from(lang: Language) -> bip39::Language {
        match lang {
            Language::English => bip39::Language::English,
            Language::SimplifiedChinese => bip39::Language::SimplifiedChinese,
            Language::TraditionalChinese => bip39::Language::TraditionalChinese,
            Language::Czech => bip39::Language::Czech,
            Language::French => bip39::Language::French,
            Language::Italian => bip39::Language::Italian,
            Language::Japanese => bip39::Language::Japanese,
            Language::Korean => bip39::Language::Korean,
            Language::Spanish => bip39::Language::Spanish,
        }
    }
}

/// A wallet implementation.
///
/// This is responsible for holding the keys, and performing operations like
/// creating transactions.
pub struct Wallet<S, C> {
    store: S,
    node: C,
}

impl<S, C> Wallet<S, C> {
    /// Create a new wallet given the underlying store and node client.
    pub const fn new(store: S, node: C) -> Self {
        Self { store, node }
    }
}

#[allow(clippy::too_many_arguments)]
impl<S, C> Wallet<S, C>
where
    S: Store,
    S::Id: Clone,
    C: NodeClient,
{
    /// Generates a random mnemonic. These mnemonics **are** the user's wallet.
    /// They should be treated with care.
    pub fn generate_mnemonic<Rng: RngCore + CryptoRng>(
        rng: &mut Rng,
        lang: Language,
    ) -> Result<Mnemonic, Error<S, C>> {
        let mut entropy = [0; 32];
        rng.try_fill_bytes(&mut entropy[..])?;
        Ok(Mnemonic::from_entropy_in(lang.into(), &entropy[..])?)
    }

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
        refund: &PublicSpendKey,
        receiver: &PublicSpendKey,
        value: u64,
        gas_limit: u64,
        gas_price: u64,
        ref_id: BlsScalar,
    ) -> Result<Transaction, Error<S, C>> {
        let sender = self.store.key(sender).map_err(Error::from_store_err)?;

        let input_notes = {
            let sender_vk = sender.view_key();

            // TODO find a way to get the block height from somewhere
            let mut notes = self
                .node
                .fetch_notes(0, &sender_vk)
                .map_err(Error::from_node_err)?;
            let mut notes_and_values = Vec::with_capacity(notes.len());

            let mut accumulated_value = 0;
            for note in notes.drain(..) {
                let val = note.value(Some(&sender_vk))?;
                let blinder = note.blinding_factor(Some(&sender_vk))?;
                accumulated_value += val;
                notes_and_values.push((note, val, blinder));
            }

            if accumulated_value < value {
                return Err(Error::NotEnoughBalance);
            }

            // This sorts the notes from least valuable to most valuable. It
            // helps in the minimum gas spent algorithm, where the largest notes
            // are "popped" first.
            notes_and_values
                .sort_by(|(_, aval, _), (_, bval, _)| aval.cmp(bval));

            let mut input_notes = Vec::with_capacity(notes.len());

            let mut accumulated_value = 0;
            while accumulated_value < value {
                // This unwrap is ok because at this point we can be sure there
                // is enough value in the notes.
                let (note, val, blinder) = notes_and_values.pop().unwrap();
                accumulated_value += val;
                input_notes.push((note, val, blinder));
            }

            if input_notes.len() > MAX_INPUT_NOTES {
                return Err(Error::NoteCombinationProblem);
            }

            input_notes
        };

        let nullifiers: Vec<BlsScalar> = input_notes
            .iter()
            .map(|(note, _, _)| note.gen_nullifier(&sender))
            .collect();

        let mut openings = Vec::with_capacity(input_notes.len());
        for (note, _, _) in &input_notes {
            let opening = self
                .node
                .fetch_opening(note)
                .map_err(Error::from_node_err)?;
            openings.push(opening);
        }

        let (output_note, output_blinder) =
            generate_obfuscated_note(rng, receiver, value, ref_id);

        // This is an implementation of sending funds from one key to another -
        // not calling a contract. This means there's one output note.
        let outputs = vec![
            // receiver note
            (output_note, value, output_blinder),
        ];

        let crossover = zero_crossover(rng);
        let fee = Fee::new(rng, gas_limit, gas_price, refund);
        let anchor = self.node.fetch_anchor().map_err(Error::from_node_err)?;

        let skel = TransactionSkeleton::new(
            nullifiers,
            vec![outputs[0].0],
            anchor,
            fee,
            crossover.0,
            None,
        );
        let hash = skel.hash();

        let inputs: Vec<UnprovenTransactionInput> = input_notes
            .into_iter()
            .zip(openings.into_iter())
            .map(|((note, value, blinder), opening)| {
                UnprovenTransactionInput::new(
                    rng, &sender, note, value, blinder, opening, hash,
                )
            })
            .collect();

        let utx = UnprovenTransaction::new(
            inputs, outputs, anchor, fee, crossover, None,
        );

        let proof = self
            .node
            .request_proof(&utx)
            .map_err(Error::from_node_err)?;
        Ok(Transaction::new(skel, proof))
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
    pub fn get_balance(&self, id: &S::Id) -> Result<u64, Error<S, C>> {
        let sender = self.store.key(id).map_err(Error::from_store_err)?;
        let vk = sender.view_key();

        let notes = self
            .node
            .fetch_notes(0, &vk)
            .map_err(|e| Error::from_node_err(e))?;

        let mut balance = 0;
        for note in notes.iter() {
            balance += note.value(Some(&vk))?;
        }

        Ok(balance)
    }
}

/// Since there is no link in the current circuit between the crossover
/// and the fee, we can generate one at random, and use only the value
/// commitment + value + blinder. We then generate one with value zero
/// and random blinder.
fn zero_crossover<Rng: RngCore + CryptoRng>(
    rng: &mut Rng,
) -> (Crossover, u64, JubJubScalar) {
    // FIXME Coupled to the logic of the circuit - should be solved by
    //  changing the `phoenix_core` API.
    let (a, b) = (
        dusk_jubjub::GENERATOR_EXTENDED * JubJubScalar::random(rng),
        dusk_jubjub::GENERATOR_EXTENDED * JubJubScalar::random(rng),
    );
    let psk = PublicSpendKey::new(a, b);

    let nonce = BlsScalar::random(rng);
    let (note, blinder) = generate_obfuscated_note(rng, &psk, 0, nonce);

    // This only verifies if the note is obfuscated. Another example of coupled
    // madness.
    let (_, crossover) = note.try_into().unwrap();

    (crossover, 0, blinder)
}

/// Generates an obfuscated note for the given public spend key.
fn generate_obfuscated_note<Rng: RngCore + CryptoRng>(
    rng: &mut Rng,
    psk: &PublicSpendKey,
    value: u64,
    nonce: BlsScalar,
) -> (Note, JubJubScalar) {
    let r = JubJubScalar::random(rng);
    let blinder = JubJubScalar::random(rng);

    (
        Note::deterministic(
            NoteType::Obfuscated,
            &r,
            nonce,
            psk,
            value,
            blinder,
        ),
        blinder,
    )
}
