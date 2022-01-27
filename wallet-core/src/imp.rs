// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::tx::UnprovenTransaction;
use crate::{ProverClient, StateClient, Store};

use alloc::vec::Vec;

use canonical::CanonError;
use canonical::EncodeToVec;
use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::{Error as BytesError, Serializable};
use dusk_jubjub::{BlsScalar, JubJubScalar};
use dusk_pki::{PublicSpendKey, SecretSpendKey};
use dusk_poseidon::sponge;
use phoenix_core::{Crossover, Error as PhoenixError, Fee, Note, NoteType};
use rand_core::{CryptoRng, Error as RngError, RngCore};
use rusk_abi::ContractId;

const MAX_INPUT_NOTES: usize = 4;

/// The error type returned by this crate.
#[derive(Debug)]
pub enum Error<S: Store, SC: StateClient, PC: ProverClient> {
    /// Underlying store error.
    Store(S::Error),
    /// Error originating from the state client.
    State(SC::Error),
    /// Error originating from the prover client.
    Prover(PC::Error),
    /// Canonical stores.
    Canon(CanonError),
    /// Random number generator error.
    Rng(RngError),
    /// Serialization and deserialization of Dusk types.
    Bytes(BytesError),
    /// Originating from the transaction model.
    Phoenix(PhoenixError),
    /// Not enough balance to perform transaction.
    NotEnoughBalance,
    /// Note combination for the given value is impossible given the maximum
    /// amount if inputs in a transaction.
    NoteCombinationProblem,
}

impl<S: Store, SC: StateClient, PC: ProverClient> Error<S, SC, PC> {
    /// Returns an error from the underlying store error.
    pub fn from_store_err(se: S::Error) -> Self {
        Self::Store(se)
    }
    /// Returns an error from the underlying state client.
    pub fn from_state_err(se: SC::Error) -> Self {
        Self::State(se)
    }
    /// Returns an error from the underlying prover client.
    pub fn from_prover_err(pe: PC::Error) -> Self {
        Self::Prover(pe)
    }
}

impl<S: Store, SC: StateClient, PC: ProverClient> From<RngError>
    for Error<S, SC, PC>
{
    fn from(re: RngError) -> Self {
        Self::Rng(re)
    }
}

impl<S: Store, SC: StateClient, PC: ProverClient> From<BytesError>
    for Error<S, SC, PC>
{
    fn from(be: BytesError) -> Self {
        Self::Bytes(be)
    }
}

impl<S: Store, SC: StateClient, PC: ProverClient> From<PhoenixError>
    for Error<S, SC, PC>
{
    fn from(pe: PhoenixError) -> Self {
        Self::Phoenix(pe)
    }
}

impl<S: Store, SC: StateClient, PC: ProverClient> From<CanonError>
    for Error<S, SC, PC>
{
    fn from(ce: CanonError) -> Self {
        Self::Canon(ce)
    }
}

/// A wallet implementation.
///
/// This is responsible for holding the keys, and performing operations like
/// creating transactions.
pub struct Wallet<S, SC, PC> {
    store: S,
    state: SC,
    prover: PC,
}

impl<S, SC, PC> Wallet<S, SC, PC> {
    /// Create a new wallet given the underlying store and node client.
    pub const fn new(store: S, state: SC, prover: PC) -> Self {
        Self {
            store,
            state,
            prover,
        }
    }
}

const TX_STAKE: u8 = 0x00;
const TX_EXTEND: u8 = 0x01;
const TX_WITHDRAW: u8 = 0x02;

impl<S, SC, PC> Wallet<S, SC, PC>
where
    S: Store,
    SC: StateClient,
    PC: ProverClient,
{
    /// Retrieve the public spend key with the given index.
    pub fn public_spend_key(
        &self,
        index: u64,
    ) -> Result<PublicSpendKey, Error<S, SC, PC>> {
        self.store
            .retrieve_ssk(index)
            .map(|ssk| ssk.public_spend_key())
            .map_err(Error::from_store_err)
    }

    /// Retrieve the public key with the given index.
    pub fn public_key(
        &self,
        index: u64,
    ) -> Result<PublicKey, Error<S, SC, PC>> {
        self.store
            .retrieve_sk(index)
            .map(|sk| From::from(&sk))
            .map_err(Error::from_store_err)
    }

    /// Here we fetch the notes and perform a "minimum number of notes
    /// required" algorithm to select which ones to use for this TX. This is
    /// done by picking notes largest to smallest until they combined have
    /// enough accumulated value.
    #[allow(clippy::type_complexity)]
    fn minimum_inputs_with_value(
        &self,
        sender: &SecretSpendKey,
        value: u64,
    ) -> Result<Vec<(Note, u64, JubJubScalar)>, Error<S, SC, PC>> {
        let sender_vk = sender.view_key();

        // TODO find a way to get the block height from somewhere. Maybe it
        //  should be determined by the client?
        let mut notes = self
            .state
            .fetch_notes(0, &sender_vk)
            .map_err(Error::from_state_err)?;
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
        notes_and_values.sort_by(|(_, aval, _), (_, bval, _)| aval.cmp(bval));

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

        Ok(input_notes)
    }

    /// Transfer Dusk from one key to another.
    #[allow(clippy::too_many_arguments)]
    pub fn transfer<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u64,
        refund: &PublicSpendKey,
        receiver: &PublicSpendKey,
        value: u64,
        gas_limit: u64,
        gas_price: u64,
        ref_id: BlsScalar,
    ) -> Result<(), Error<S, SC, PC>> {
        let sender = self
            .store
            .retrieve_ssk(sender_index)
            .map_err(Error::from_store_err)?;

        let inputs = self.minimum_inputs_with_value(
            &sender,
            value + gas_limit * gas_price,
        )?;

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

        let utx = UnprovenTransaction::new(
            rng,
            &self.state,
            &sender,
            inputs,
            outputs,
            fee,
            crossover,
            None,
        )
        .map_err(Error::from_state_err)?;

        self.prover
            .compute_proof_and_propagate(&utx)
            .map_err(Error::from_prover_err)?;
        Ok(())
    }

    /// Stakes an amount of Dusk.
    #[allow(clippy::too_many_arguments)]
    pub fn stake<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u64,
        staker_index: u64,
        refund: &PublicSpendKey,
        value: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<(), Error<S, SC, PC>> {
        let sender = self
            .store
            .retrieve_ssk(sender_index)
            .map_err(Error::from_store_err)?;

        let inputs = self.minimum_inputs_with_value(
            &sender,
            value + gas_limit * gas_price,
        )?;

        // There's no outputs in a stake transaction.
        let outputs = vec![];

        let fee = Fee::new(rng, gas_limit, gas_price, refund);

        let blinder = JubJubScalar::random(rng);
        let note =
            Note::obfuscated(rng, &sender.public_spend_key(), value, blinder);
        let (_, crossover) = note
            .try_into()
            .expect("Obfuscated notes should always yield crossovers");

        let sk = self
            .store
            .retrieve_sk(staker_index)
            .map_err(Error::from_store_err)?;
        let pk = PublicKey::from(&sk);

        let contract_id = rusk_abi::stake_contract();
        let address = contract_to_scalar(&contract_id);

        let mut m = crossover.to_hash_inputs().to_vec();
        m.push(value.into());
        m.push(address);

        let signature = sk.sign(&pk, &sponge::hash(&m).to_bytes());

        let spend_proof = self
            .prover
            .request_stct_proof(
                &fee, &crossover, value, blinder, address, signature,
            )
            .map_err(Error::from_prover_err)?;

        let call_data =
            (TX_STAKE, pk, value, spend_proof.to_bytes()).encode_to_vec();

        let call = (contract_id, call_data);

        let utx = UnprovenTransaction::new(
            rng,
            &self.state,
            &sender,
            inputs,
            outputs,
            fee,
            (crossover, value, blinder),
            Some(call),
        )
        .map_err(Error::from_state_err)?;

        self.prover
            .compute_proof_and_propagate(&utx)
            .map_err(Error::from_prover_err)?;
        Ok(())
    }

    /// Extends staking for a particular key.
    pub fn extend_stake<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u64,
        staker_index: u64,
        refund: &PublicSpendKey,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<(), Error<S, SC, PC>> {
        let sender = self
            .store
            .retrieve_ssk(sender_index)
            .map_err(Error::from_store_err)?;

        let inputs =
            self.minimum_inputs_with_value(&sender, gas_limit * gas_price)?;

        // There's no outputs in an extend stake transaction.
        let outputs = vec![];

        let fee = Fee::new(rng, gas_limit, gas_price, refund);

        let (crossover, value, blinder) = zero_crossover(rng);

        let sk = self
            .store
            .retrieve_sk(staker_index)
            .map_err(Error::from_store_err)?;
        let pk = PublicKey::from(&sk);

        let (_, expiration) =
            self.state.fetch_stake(&pk).map_err(Error::from_state_err)?;
        let expiration = expiration.to_le_bytes();

        let signature = sk.sign(&pk, &expiration);

        let contract_id = rusk_abi::stake_contract();
        let call_data = (TX_EXTEND, pk, signature).encode_to_vec();

        let call = (contract_id, call_data);

        let utx = UnprovenTransaction::new(
            rng,
            &self.state,
            &sender,
            inputs,
            outputs,
            fee,
            (crossover, value, blinder),
            Some(call),
        )
        .map_err(Error::from_state_err)?;

        self.prover
            .compute_proof_and_propagate(&utx)
            .map_err(Error::from_prover_err)?;
        Ok(())
    }

    /// Withdraw a key's stake.
    pub fn withdraw_stake<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u64,
        staker_index: u64,
        refund: &PublicSpendKey,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<(), Error<S, SC, PC>> {
        let sender = self
            .store
            .retrieve_ssk(sender_index)
            .map_err(Error::from_store_err)?;

        let inputs =
            self.minimum_inputs_with_value(&sender, gas_limit * gas_price)?;

        let sk = self
            .store
            .retrieve_sk(staker_index)
            .map_err(Error::from_store_err)?;
        let pk = PublicKey::from(&sk);

        let (stake, expiration) =
            self.state.fetch_stake(&pk).map_err(Error::from_state_err)?;

        let output_note =
            Note::transparent(rng, &sender.public_spend_key(), stake);
        let note_blinder = output_note
            .blinding_factor(None)
            .expect("Note is transparent so blinding factor is unencrypted");

        let fee = Fee::new(rng, gas_limit, gas_price, refund);

        let blinder = JubJubScalar::random(rng);
        let note =
            Note::obfuscated(rng, &sender.public_spend_key(), stake, blinder);
        let (_, crossover) = note
            .try_into()
            .expect("Obfuscated notes should always yield crossovers");

        let outputs = vec![(output_note, stake, note_blinder)];

        let proof = self
            .prover
            .request_wfct_proof(
                crossover.value_commitment().into(),
                stake,
                blinder,
            )
            .map_err(Error::from_prover_err)?;

        let message = expiration.to_le_bytes();
        let signature = sk.sign(&pk, &message);

        let contract_id = rusk_abi::stake_contract();
        let call_data =
            (TX_WITHDRAW, pk, signature, output_note, proof.to_bytes())
                .encode_to_vec();

        let call = (contract_id, call_data);

        let utx = UnprovenTransaction::new(
            rng,
            &self.state,
            &sender,
            inputs,
            outputs,
            fee,
            (crossover, stake, blinder),
            Some(call),
        )
        .map_err(Error::from_state_err)?;

        self.prover
            .compute_proof_and_propagate(&utx)
            .map_err(Error::from_prover_err)?;
        Ok(())
    }

    /// Gets the balance of a key.
    pub fn get_balance(&self, ssk_index: u64) -> Result<u64, Error<S, SC, PC>> {
        let sender = self
            .store
            .retrieve_ssk(ssk_index)
            .map_err(Error::from_store_err)?;
        let vk = sender.view_key();

        let notes = self
            .state
            .fetch_notes(0, &vk)
            .map_err(Error::from_state_err)?;

        let mut balance = 0;
        for note in notes.iter() {
            balance += note.value(Some(&vk))?;
        }

        Ok(balance)
    }

    /// Gets the stake and the expiration of said stake for a key.
    pub fn get_stake(
        &self,
        sk_index: u64,
    ) -> Result<(u64, u64), Error<S, SC, PC>> {
        let sk = self
            .store
            .retrieve_sk(sk_index)
            .map_err(Error::from_store_err)?;

        let pk = PublicKey::from(&sk);

        let s = self.state.fetch_stake(&pk).map_err(Error::from_state_err)?;

        Ok(s)
    }
}

fn contract_to_scalar(id: &ContractId) -> BlsScalar {
    let mut scalar = [0; 32];
    scalar.copy_from_slice(id.as_bytes());

    scalar[31] &= 0x3f;

    BlsScalar::from_bytes(&scalar).unwrap_or_default()
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
