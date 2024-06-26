// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{
    BalanceInfo, ProverClient, StakeInfo, StateClient, Store, MAX_CALL_SIZE,
};

use core::convert::Infallible;

use alloc::string::{FromUtf8Error, String};
use alloc::vec::Vec;

use dusk_bytes::Error as BytesError;
use execution_core::{
    stake::{Stake, Unstake, Withdraw},
    transfer::{ContractCall, Fee, Payload, Transaction},
    BlsPublicKey as StakePublicKey, BlsScalar, JubJubScalar, Note,
    PhoenixError, PublicKey, SchnorrSecretKey, SecretKey, TxSkeleton, ViewKey,
    OUTPUT_NOTES,
};
use ff::Field;
use rand_core::{CryptoRng, Error as RngError, RngCore};
use rkyv::ser::serializers::{
    AllocScratchError, CompositeSerializerError, SharedSerializeMapError,
};
use rkyv::validation::validators::CheckDeserializeError;
use rusk_prover::{UnprovenTransaction, UnprovenTransactionInput};

const MAX_INPUT_NOTES: usize = 4;

const TX_STAKE: &str = "stake";
const TX_UNSTAKE: &str = "unstake";
const TX_WITHDRAW: &str = "withdraw";

type SerializerError = CompositeSerializerError<
    Infallible,
    AllocScratchError,
    SharedSerializeMapError,
>;

/// The error type returned by this crate.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Error<S: Store, SC: StateClient, PC: ProverClient> {
    /// Underlying store error.
    Store(S::Error),
    /// Error originating from the state client.
    State(SC::Error),
    /// Error originating from the prover client.
    Prover(PC::Error),
    /// Rkyv serialization.
    Rkyv,
    /// Random number generator error.
    Rng(RngError),
    /// Serialization and deserialization of Dusk types.
    Bytes(BytesError),
    /// Bytes were meant to be utf8 but aren't.
    Utf8(FromUtf8Error),
    /// Originating from the transaction model.
    Phoenix(PhoenixError),
    /// Not enough balance to perform transaction.
    NotEnoughBalance,
    /// Note combination for the given value is impossible given the maximum
    /// amount if inputs in a transaction.
    NoteCombinationProblem,
    /// The key is already staked. This happens when there already is an amount
    /// staked for a key and the user tries to make a stake transaction.
    AlreadyStaked {
        /// The key that already has a stake.
        key: StakePublicKey,
        /// Information about the key's stake.
        stake: StakeInfo,
    },
    /// The key is not staked. This happens when a key doesn't have an amount
    /// staked and the user tries to make an unstake transaction.
    NotStaked {
        /// The key that is not staked.
        key: StakePublicKey,
        /// Information about the key's stake.
        stake: StakeInfo,
    },
    /// The key has no reward. This happens when a key has no reward in the
    /// stake contract and the user tries to make a withdraw transaction.
    NoReward {
        /// The key that has no reward.
        key: StakePublicKey,
        /// Information about the key's stake.
        stake: StakeInfo,
    },
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

impl<S: Store, SC: StateClient, PC: ProverClient> From<SerializerError>
    for Error<S, SC, PC>
{
    fn from(_: SerializerError) -> Self {
        Self::Rkyv
    }
}

impl<C, D, S: Store, SC: StateClient, PC: ProverClient>
    From<CheckDeserializeError<C, D>> for Error<S, SC, PC>
{
    fn from(_: CheckDeserializeError<C, D>) -> Self {
        Self::Rkyv
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

impl<S: Store, SC: StateClient, PC: ProverClient> From<FromUtf8Error>
    for Error<S, SC, PC>
{
    fn from(err: FromUtf8Error) -> Self {
        Self::Utf8(err)
    }
}

impl<S: Store, SC: StateClient, PC: ProverClient> From<PhoenixError>
    for Error<S, SC, PC>
{
    fn from(pe: PhoenixError) -> Self {
        Self::Phoenix(pe)
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

    /// Return the inner Store reference
    pub const fn store(&self) -> &S {
        &self.store
    }

    /// Return the inner State reference
    pub const fn state(&self) -> &SC {
        &self.state
    }

    /// Return the inner Prover reference
    pub const fn prover(&self) -> &PC {
        &self.prover
    }
}

impl<S, SC, PC> Wallet<S, SC, PC>
where
    S: Store,
    SC: StateClient,
    PC: ProverClient,
{
    /// Retrieve the public spend key with the given index.
    pub fn public_key(
        &self,
        index: u64,
    ) -> Result<PublicKey, Error<S, SC, PC>> {
        self.store
            .retrieve_sk(index)
            .map(|sk| PublicKey::from(&sk))
            .map_err(Error::from_store_err)
    }

    /// Retrieve the public key with the given index.
    pub fn stake_public_key(
        &self,
        index: u64,
    ) -> Result<StakePublicKey, Error<S, SC, PC>> {
        self.store
            .retrieve_stake_sk(index)
            .map(|stake_sk| From::from(&stake_sk))
            .map_err(Error::from_store_err)
    }

    /// Fetches the notes and nullifiers in the state and returns the notes that
    /// are still available for spending.
    fn unspent_notes(
        &self,
        sk: &SecretKey,
    ) -> Result<Vec<Note>, Error<S, SC, PC>> {
        let vk = ViewKey::from(sk);

        let notes =
            self.state.fetch_notes(&vk).map_err(Error::from_state_err)?;

        let nullifiers: Vec<_> =
            notes.iter().map(|(n, _)| n.gen_nullifier(sk)).collect();

        let existing_nullifiers = self
            .state
            .fetch_existing_nullifiers(&nullifiers)
            .map_err(Error::from_state_err)?;

        let unspent_notes = notes
            .into_iter()
            .zip(nullifiers.into_iter())
            .filter(|(_, nullifier)| !existing_nullifiers.contains(nullifier))
            .map(|((note, _), _)| note)
            .collect();

        Ok(unspent_notes)
    }

    /// Here we fetch the notes and perform a "minimum number of notes
    /// required" algorithm to select which ones to use for this TX. This is
    /// done by picking notes largest to smallest until they combined have
    /// enough accumulated value.
    ///
    /// We also return the outputs with a possible change note (if applicable).
    #[allow(clippy::type_complexity)]
    fn inputs_and_change_output<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_sk: &SecretKey,
        sender_pk: &PublicKey,
        receiver_pk: &PublicKey,
        transfer_value: u64,
        max_fee: u64,
        deposit: u64,
    ) -> Result<
        (
            Vec<(Note, u64, JubJubScalar)>,
            [(Note, u64, JubJubScalar, [JubJubScalar; 2]); OUTPUT_NOTES],
        ),
        Error<S, SC, PC>,
    > {
        let notes = self.unspent_notes(sender_sk)?;
        let mut notes_and_values = Vec::with_capacity(notes.len());

        let sender_vk = ViewKey::from(sender_sk);

        let mut accumulated_value = 0;
        for note in notes.into_iter() {
            let val = note.value(Some(&sender_vk))?;
            let value_blinder = note.value_blinder(Some(&sender_vk))?;

            accumulated_value += val;
            notes_and_values.push((note, val, value_blinder));
        }

        if accumulated_value < transfer_value + max_fee {
            return Err(Error::NotEnoughBalance);
        }

        let inputs =
            pick_notes(transfer_value + max_fee + deposit, notes_and_values);

        if inputs.is_empty() {
            return Err(Error::NoteCombinationProblem);
        }

        let (transfer_note, transfer_value_blinder, transfer_sender_blinder) =
            generate_obfuscated_note(
                rng,
                sender_pk,
                receiver_pk,
                transfer_value,
            );

        let change = inputs.iter().map(|v| v.1).sum::<u64>()
            - transfer_value
            - max_fee
            - deposit;
        let change_sender_blinder = [
            JubJubScalar::random(&mut *rng),
            JubJubScalar::random(&mut *rng),
        ];
        let change_note = Note::transparent(
            rng,
            sender_pk,
            sender_pk,
            change,
            change_sender_blinder,
        );

        let outputs = [
            (
                transfer_note,
                transfer_value,
                transfer_value_blinder,
                transfer_sender_blinder,
            ),
            (
                change_note,
                change,
                JubJubScalar::zero(),
                change_sender_blinder,
            ),
        ];

        Ok((inputs, outputs))
    }

    /// Execute a generic contract call
    #[allow(clippy::too_many_arguments)]
    pub fn execute<Rng>(
        &self,
        rng: &mut Rng,
        contract_call: ContractCall,
        sender_index: u64,
        gas_limit: u64,
        gas_price: u64,
        deposit: u64,
    ) -> Result<Transaction, Error<S, SC, PC>>
    where
        Rng: RngCore + CryptoRng,
    {
        let sender_sk = self
            .store
            .retrieve_sk(sender_index)
            .map_err(Error::from_store_err)?;
        let sender_pk = PublicKey::from(&sender_sk);

        let (inputs, outputs) = self.inputs_and_change_output(
            rng,
            &sender_sk,
            &sender_pk,
            &sender_pk,
            0,
            gas_limit * gas_price,
            deposit,
        )?;

        let fee = Fee::new(rng, &sender_pk, gas_limit, gas_price);

        let utx = new_unproven_tx(
            rng,
            &self.state,
            &sender_sk,
            inputs,
            outputs,
            fee,
            0,
            Some(contract_call),
        )
        .map_err(Error::from_state_err)?;

        self.prover
            .compute_proof_and_propagate(&utx)
            .map_err(Error::from_prover_err)
    }

    /// Transfer Dusk from one key to another.
    #[allow(clippy::too_many_arguments)]
    pub fn transfer<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u64,
        receiver_pk: &PublicKey,
        value: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC, PC>> {
        let sender_sk = self
            .store
            .retrieve_sk(sender_index)
            .map_err(Error::from_store_err)?;
        let sender_pk = PublicKey::from(&sender_sk);

        let deposit = 0;
        let (inputs, outputs) = self.inputs_and_change_output(
            rng,
            &sender_sk,
            &sender_pk,
            receiver_pk,
            value,
            gas_limit * gas_price,
            deposit,
        )?;

        let fee = Fee::new(rng, &sender_pk, gas_limit, gas_price);

        let utx = new_unproven_tx(
            rng,
            &self.state,
            &sender_sk,
            inputs,
            outputs,
            fee,
            deposit,
            None,
        )
        .map_err(Error::from_state_err)?;

        self.prover
            .compute_proof_and_propagate(&utx)
            .map_err(Error::from_prover_err)
    }

    /// Stakes an amount of Dusk.
    #[allow(clippy::too_many_arguments)]
    pub fn stake<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u64,
        staker_index: u64,
        value: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC, PC>> {
        let sender_sk = self
            .store
            .retrieve_sk(sender_index)
            .map_err(Error::from_store_err)?;
        let sender_pk = PublicKey::from(&sender_sk);

        let stake_sk = self
            .store
            .retrieve_stake_sk(staker_index)
            .map_err(Error::from_store_err)?;
        let stake_pk = StakePublicKey::from(&stake_sk);
        let deposit = value;

        let (inputs, outputs) = self.inputs_and_change_output(
            rng,
            &sender_sk,
            &sender_pk,
            &sender_pk,
            0,
            gas_limit * gas_price,
            deposit,
        )?;

        let stake = self
            .state
            .fetch_stake(&stake_pk)
            .map_err(Error::from_state_err)?;
        if stake.amount.is_some() {
            return Err(Error::AlreadyStaked {
                key: stake_pk,
                stake,
            });
        }

        let fee = Fee::new(rng, &sender_pk, gas_limit, gas_price);

        let msg = Stake::signature_message(stake.counter, value);
        let stake_sig = stake_sk.sign(&stake_pk, &msg);

        let stake = Stake {
            public_key: stake_pk,
            signature: stake_sig,
            value,
        };

        let contract_call = ContractCall::new(
            rusk_abi::STAKE_CONTRACT.to_bytes(),
            TX_STAKE,
            &stake,
        )
        .expect("call data should serialize");

        let utx = new_unproven_tx(
            rng,
            &self.state,
            &sender_sk,
            inputs,
            outputs,
            fee,
            value,
            Some(contract_call),
        )
        .map_err(Error::from_state_err)?;

        self.prover
            .compute_proof_and_propagate(&utx)
            .map_err(Error::from_prover_err)
    }

    /// Unstake a key from the stake contract.
    pub fn unstake<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u64,
        staker_index: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC, PC>> {
        let sender_sk = self
            .store
            .retrieve_sk(sender_index)
            .map_err(Error::from_store_err)?;
        let sender_pk = PublicKey::from(&sender_sk);

        let stake_sk = self
            .store
            .retrieve_stake_sk(staker_index)
            .map_err(Error::from_store_err)?;
        let stake_pk = StakePublicKey::from(&stake_sk);

        let (inputs, outputs) = self.inputs_and_change_output(
            rng,
            &sender_sk,
            &sender_pk,
            &sender_pk,
            0,
            gas_limit * gas_price,
            0,
        )?;

        let stake = self
            .state
            .fetch_stake(&stake_pk)
            .map_err(Error::from_state_err)?;
        let (value, _) = stake.amount.ok_or(Error::NotStaked {
            key: stake_pk,
            stake,
        })?;

        let fee = Fee::new(rng, &sender_pk, gas_limit, gas_price);
        let deposit = 0;

        let unstake_stealth_address = PublicKey::from(&sender_sk)
            .gen_stealth_address(&JubJubScalar::random(&mut *rng));

        let signature_message = Unstake::signature_message(
            stake.counter,
            value,
            unstake_stealth_address,
        );

        let stake_sig = stake_sk.sign(&stake_pk, &signature_message);

        let unstake = Unstake {
            public_key: stake_pk,
            signature: stake_sig,
            address: unstake_stealth_address,
        };

        let call_data = rkyv::to_bytes::<_, MAX_CALL_SIZE>(&unstake)?.to_vec();
        let call = ContractCall {
            contract: rusk_abi::STAKE_CONTRACT.to_bytes(),
            fn_name: String::from(TX_UNSTAKE),
            fn_args: call_data,
        };

        let utx = new_unproven_tx(
            rng,
            &self.state,
            &sender_sk,
            inputs,
            outputs,
            fee,
            deposit,
            Some(call),
        )
        .map_err(Error::from_state_err)?;

        self.prover
            .compute_proof_and_propagate(&utx)
            .map_err(Error::from_prover_err)
    }

    /// Withdraw the reward a key has reward if accumulated by staking and
    /// taking part in operating the network.
    pub fn withdraw<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u64,
        staker_index: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC, PC>> {
        let sender_sk = self
            .store
            .retrieve_sk(sender_index)
            .map_err(Error::from_store_err)?;
        let sender_pk = PublicKey::from(&sender_sk);

        let stake_sk = self
            .store
            .retrieve_stake_sk(staker_index)
            .map_err(Error::from_store_err)?;
        let stake_pk = StakePublicKey::from(&stake_sk);

        let (inputs, outputs) = self.inputs_and_change_output(
            rng,
            &sender_sk,
            &sender_pk,
            &sender_pk,
            0,
            gas_limit * gas_price,
            0,
        )?;

        let stake = self
            .state
            .fetch_stake(&stake_pk)
            .map_err(Error::from_state_err)?;
        if stake.reward == 0 {
            return Err(Error::NoReward {
                key: stake_pk,
                stake,
            });
        }

        let address =
            sender_pk.gen_stealth_address(&JubJubScalar::random(&mut *rng));
        let nonce = BlsScalar::random(&mut *rng);

        let msg = Withdraw::signature_message(stake.counter, address, nonce);
        let stake_sig = stake_sk.sign(&stake_pk, &msg);

        let withdraw = Withdraw {
            public_key: stake_pk,
            signature: stake_sig,
            address,
            nonce,
        };

        let fee = Fee::new(rng, &sender_pk, gas_limit, gas_price);
        let deposit = 0;

        let call_data = rkyv::to_bytes::<_, MAX_CALL_SIZE>(&withdraw)?.to_vec();

        let call = ContractCall {
            contract: rusk_abi::STAKE_CONTRACT.to_bytes(),
            fn_name: String::from(TX_WITHDRAW),
            fn_args: call_data,
        };

        let utx = new_unproven_tx(
            rng,
            &self.state,
            &sender_sk,
            inputs,
            outputs,
            fee,
            deposit,
            Some(call),
        )
        .map_err(Error::from_state_err)?;

        self.prover
            .compute_proof_and_propagate(&utx)
            .map_err(Error::from_prover_err)
    }

    /// Gets the balance of a key.
    pub fn get_balance(
        &self,
        sk_index: u64,
    ) -> Result<BalanceInfo, Error<S, SC, PC>> {
        let sender_sk = self
            .store
            .retrieve_sk(sk_index)
            .map_err(Error::from_store_err)?;
        let vk = ViewKey::from(&sender_sk);

        let notes = self.unspent_notes(&sender_sk)?;
        let mut values = Vec::with_capacity(notes.len());

        for note in notes.into_iter() {
            values.push(note.value(Some(&vk))?);
        }
        values.sort_by(|a, b| b.cmp(a));

        let spendable = values.iter().take(MAX_INPUT_NOTES).sum();
        let value =
            spendable + values.iter().skip(MAX_INPUT_NOTES).sum::<u64>();

        Ok(BalanceInfo { value, spendable })
    }

    /// Gets the stake and the expiration of said stake for a key.
    pub fn get_stake(
        &self,
        sk_index: u64,
    ) -> Result<StakeInfo, Error<S, SC, PC>> {
        let stake_sk = self
            .store
            .retrieve_stake_sk(sk_index)
            .map_err(Error::from_store_err)?;

        let stake_pk = StakePublicKey::from(&stake_sk);

        let s = self
            .state
            .fetch_stake(&stake_pk)
            .map_err(Error::from_state_err)?;

        Ok(s)
    }
}

/// Creates an unproven transaction that conforms to the transfer contract.
#[allow(clippy::too_many_arguments)]
fn new_unproven_tx<Rng: RngCore + CryptoRng, SC: StateClient>(
    rng: &mut Rng,
    state: &SC,
    sender_sk: &SecretKey,
    inputs: Vec<(Note, u64, JubJubScalar)>,
    outputs: [(Note, u64, JubJubScalar, [JubJubScalar; 2]); OUTPUT_NOTES],
    fee: Fee,
    deposit: u64,
    call: Option<ContractCall>,
) -> Result<UnprovenTransaction, SC::Error> {
    let nullifiers: Vec<BlsScalar> = inputs
        .iter()
        .map(|(note, _, _)| note.gen_nullifier(sender_sk))
        .collect();

    let mut openings = Vec::with_capacity(inputs.len());
    for (note, _, _) in &inputs {
        let opening = state.fetch_opening(note)?;
        openings.push(opening);
    }

    let root = state.fetch_root()?;

    let tx_skeleton = TxSkeleton {
        root,
        nullifiers,
        outputs: [outputs[0].0.clone(), outputs[1].0.clone()],
        max_fee: fee.max_fee(),
        deposit,
    };
    let has_deposit = deposit > 0;

    let payload = Payload::new(tx_skeleton, fee, has_deposit, call);
    let payload_hash = payload.hash();

    let inputs: Vec<UnprovenTransactionInput> = inputs
        .into_iter()
        .zip(openings.into_iter())
        .map(|((note, value, value_blinder), opening)| {
            UnprovenTransactionInput::new(
                rng,
                sender_sk,
                note,
                value,
                value_blinder,
                opening,
                payload_hash,
            )
        })
        .collect();

    let schnorr_sk_a = SchnorrSecretKey::from(sender_sk.a());
    let sig_a = schnorr_sk_a.sign(rng, payload_hash);
    let schnorr_sk_b = SchnorrSecretKey::from(sender_sk.b());
    let sig_b = schnorr_sk_b.sign(rng, payload_hash);

    Ok(UnprovenTransaction {
        inputs,
        outputs,
        payload,
        sender_pk: PublicKey::from(sender_sk),
        signatures: (sig_a, sig_b),
    })
}

/// Pick the notes to be used in a transaction from a vector of notes.
///
/// The notes are picked in a way to maximize the number of notes used, while
/// minimizing the value employed. To do this we sort the notes in ascending
/// value order, and go through each combination in a lexicographic order
/// until we find the first combination whose sum is larger or equal to
/// the given value. If such a slice is not found, an empty vector is returned.
///
/// Note: it is presupposed that the input notes contain enough balance to cover
/// the given `value`.
fn pick_notes(
    value: u64,
    notes_and_values: Vec<(Note, u64, JubJubScalar)>,
) -> Vec<(Note, u64, JubJubScalar)> {
    let mut notes_and_values = notes_and_values;
    let len = notes_and_values.len();

    if len <= MAX_INPUT_NOTES {
        return notes_and_values;
    }

    notes_and_values.sort_by(|(_, aval, _), (_, bval, _)| aval.cmp(bval));

    pick_lexicographic(notes_and_values.len(), |indices| {
        indices
            .iter()
            .map(|index| notes_and_values[*index].1)
            .sum::<u64>()
            >= value
    })
    .map(|indices| {
        indices
            .into_iter()
            .map(|index| notes_and_values[index].clone())
            .collect()
    })
    .unwrap_or_default()
}

fn pick_lexicographic<F: Fn(&[usize; MAX_INPUT_NOTES]) -> bool>(
    max_len: usize,
    is_valid: F,
) -> Option<[usize; MAX_INPUT_NOTES]> {
    let mut indices = [0; MAX_INPUT_NOTES];
    indices
        .iter_mut()
        .enumerate()
        .for_each(|(i, index)| *index = i);

    loop {
        if is_valid(&indices) {
            return Some(indices);
        }

        let mut i = MAX_INPUT_NOTES - 1;

        while indices[i] == i + max_len - MAX_INPUT_NOTES {
            if i > 0 {
                i -= 1;
            } else {
                break;
            }
        }

        indices[i] += 1;
        for j in i + 1..MAX_INPUT_NOTES {
            indices[j] = indices[j - 1] + 1;
        }

        if indices[MAX_INPUT_NOTES - 1] == max_len {
            break;
        }
    }

    None
}

/// Generates an obfuscated note for the given public spend key.
fn generate_obfuscated_note<Rng: RngCore + CryptoRng>(
    rng: &mut Rng,
    sender_pk: &PublicKey,
    receiver_pk: &PublicKey,
    value: u64,
) -> (Note, JubJubScalar, [JubJubScalar; 2]) {
    let value_blinder = JubJubScalar::random(&mut *rng);
    let sender_blinder = [
        JubJubScalar::random(&mut *rng),
        JubJubScalar::random(&mut *rng),
    ];

    (
        Note::obfuscated(
            rng,
            sender_pk,
            receiver_pk,
            value,
            value_blinder,
            sender_blinder,
        ),
        value_blinder,
        sender_blinder,
    )
}
