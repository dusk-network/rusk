// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{ProverClient, StateClient, Store};

use core::convert::Infallible;

use alloc::string::FromUtf8Error;
use alloc::vec::Vec;

use dusk_bytes::Error as BytesError;
use poseidon_merkle::Opening;
use rand_core::{CryptoRng, Error as RngError, RngCore};
use rkyv::ser::serializers::{
    AllocScratchError, CompositeSerializerError, SharedSerializeMapError,
};
use rkyv::validation::validators::CheckDeserializeError;
use zeroize::Zeroize;

use execution_core::{
    signatures::bls::{PublicKey as BlsPublicKey, SecretKey as BlsSecretKey},
    stake::StakeData,
    transfer::{
        contract_exec::ContractExec,
        moonlight::{AccountData, Transaction as MoonlightTransaction},
        phoenix::{
            Error as PhoenixError, Note, PublicKey as PhoenixPublicKey,
            SecretKey as PhoenixSecretKey, ViewKey as PhoenixViewKey,
            NOTES_TREE_DEPTH,
        },
        Transaction,
    },
    BlsScalar,
};
use wallet_core::{
    keys::{derive_bls_sk, derive_phoenix_sk},
    phoenix_balance,
    transaction::{
        phoenix_stake, phoenix_transaction, phoenix_unstake,
        phoenix_withdraw_stake_reward,
    },
    BalanceInfo,
};

const MAX_INPUT_NOTES: usize = 4;

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
    /// Originating from the phoenix transaction model.
    Phoenix(PhoenixError),
    /// Not enough balance to perform phoenix transaction.
    NotEnoughBalance,
    /// Note combination for the given value is impossible given the maximum
    /// amount if inputs in a phoenix transaction.
    NoteCombinationProblem,
    /// The key is already staked. This happens when there already is an amount
    /// staked for a key and the user tries to make a stake transaction.
    AlreadyStaked {
        /// The key that already has a stake.
        key: BlsPublicKey,
        /// Information about the key's stake.
        stake: StakeData,
    },
    /// The key is not staked. This happens when a key doesn't have an amount
    /// staked and the user tries to make an unstake transaction.
    NotStaked {
        /// The key that is not staked.
        key: BlsPublicKey,
        /// Information about the key's stake.
        stake: StakeData,
    },
    /// The key has no reward. This happens when a key has no reward in the
    /// stake contract and the user tries to make a stake withdraw transaction.
    NoReward {
        /// The key that has no reward.
        key: BlsPublicKey,
        /// Information about the key's stake.
        stake: StakeData,
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
    /// Retrieve the secret key with the given index.
    pub fn phoenix_secret_key(
        &self,
        index: u8,
    ) -> Result<PhoenixSecretKey, Error<S, SC, PC>> {
        self.store
            .phoenix_secret_key(index)
            .map_err(Error::from_store_err)
    }

    /// Retrieve the public key with the given index.
    pub fn phoenix_public_key(
        &self,
        index: u8,
    ) -> Result<PhoenixPublicKey, Error<S, SC, PC>> {
        self.store
            .phoenix_public_key(index)
            .map_err(Error::from_store_err)
    }

    /// Retrieve the account secret key with the given index.
    pub fn account_secret_key(
        &self,
        index: u8,
    ) -> Result<BlsSecretKey, Error<S, SC, PC>> {
        self.store
            .account_secret_key(index)
            .map_err(Error::from_store_err)
    }

    /// Retrieve the account public key with the given index.
    pub fn account_public_key(
        &self,
        index: u8,
    ) -> Result<BlsPublicKey, Error<S, SC, PC>> {
        self.store
            .account_public_key(index)
            .map_err(Error::from_store_err)
    }

    /// Fetches the notes and nullifiers in the state and returns the notes that
    /// are still available for spending.
    fn unspent_notes_and_nullifiers(
        &self,
        sk: &PhoenixSecretKey,
    ) -> Result<Vec<(Note, BlsScalar)>, Error<S, SC, PC>> {
        let vk = PhoenixViewKey::from(sk);

        let notes: Vec<Note> = self
            .state
            .fetch_notes(&vk)
            .map_err(Error::from_state_err)?
            .into_iter()
            .map(|(note, _bh)| note)
            .collect();

        let nullifiers: Vec<_> =
            notes.iter().map(|n| n.gen_nullifier(sk)).collect();

        let existing_nullifiers = self
            .state
            .fetch_existing_nullifiers(&nullifiers)
            .map_err(Error::from_state_err)?;

        let unspent_notes_and_nullifiers = notes
            .into_iter()
            .zip(nullifiers.into_iter())
            .filter(|(_note, nullifier)| {
                !existing_nullifiers.contains(nullifier)
            })
            .collect();

        Ok(unspent_notes_and_nullifiers)
    }

    /// Here we fetch the notes and their nullifiers to cover the
    /// transaction-costs.
    #[allow(clippy::type_complexity)]
    fn input_notes_nullifiers(
        &self,
        sender_sk: &PhoenixSecretKey,
        transaction_cost: u64,
    ) -> Result<Vec<(Note, BlsScalar)>, Error<S, SC, PC>> {
        let sender_vk = PhoenixViewKey::from(sender_sk);

        // decrypt the value of all unspent note
        let unspent_notes_nullifiers =
            self.unspent_notes_and_nullifiers(sender_sk)?;
        let mut notes_values_nullifiers =
            Vec::with_capacity(unspent_notes_nullifiers.len());

        let mut accumulated_value = 0;
        for (note, nullifier) in unspent_notes_nullifiers {
            let val = note.value(Some(&sender_vk))?;
            accumulated_value += val;
            notes_values_nullifiers.push((note, val, nullifier));
        }

        if accumulated_value < transaction_cost {
            return Err(Error::NotEnoughBalance);
        }

        // pick the four smallest notes that cover the costs
        let inputs = pick_notes(transaction_cost, notes_values_nullifiers);

        if inputs.is_empty() {
            return Err(Error::NoteCombinationProblem);
        }

        Ok(inputs)
    }

    /// Here we fetch the notes, their openings and nullifiers to cover the
    /// transfer-costs.
    #[allow(clippy::type_complexity)]
    fn input_notes_openings_nullifiers(
        &self,
        sender_sk: &PhoenixSecretKey,
        transaction_cost: u64,
    ) -> Result<
        Vec<(Note, Opening<(), NOTES_TREE_DEPTH>, BlsScalar)>,
        Error<S, SC, PC>,
    > {
        let notes_and_nullifiers =
            self.input_notes_nullifiers(sender_sk, transaction_cost)?;

        let mut notes_openings_nullifiers =
            Vec::with_capacity(notes_and_nullifiers.len());
        for (note, nullifier) in notes_and_nullifiers.into_iter() {
            let opening = self
                .state
                .fetch_opening(&note)
                .map_err(Error::from_state_err)?;
            notes_openings_nullifiers.push((note, opening, nullifier));
        }

        Ok(notes_openings_nullifiers)
    }

    /// Here we fetch the notes and their openings to cover the
    /// transfer-costs.
    #[allow(clippy::type_complexity)]
    fn input_notes_openings(
        &self,
        sender_sk: &PhoenixSecretKey,
        transaction_cost: u64,
    ) -> Result<Vec<(Note, Opening<(), NOTES_TREE_DEPTH>)>, Error<S, SC, PC>>
    {
        let notes_and_nullifiers =
            self.input_notes_nullifiers(sender_sk, transaction_cost)?;

        let mut notes_openings = Vec::with_capacity(notes_and_nullifiers.len());
        for (note, _nullifier) in notes_and_nullifiers.into_iter() {
            let opening = self
                .state
                .fetch_opening(&note)
                .map_err(Error::from_state_err)?;
            notes_openings.push((note, opening));
        }

        Ok(notes_openings)
    }

    /// Execute a generic contract call or deployment, using Phoenix notes to
    /// pay for gas.
    #[allow(clippy::too_many_arguments)]
    pub fn phoenix_execute<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u8,
        gas_limit: u64,
        gas_price: u64,
        deposit: u64,
        exec: impl Into<ContractExec>,
    ) -> Result<Transaction, Error<S, SC, PC>> {
        let mut sender_sk = self.phoenix_secret_key(sender_index)?;
        let receiver_pk = self.phoenix_public_key(sender_index)?;
        let change_pk = receiver_pk;

        let input_notes_openings = self.input_notes_openings(
            &sender_sk,
            gas_limit * gas_price + deposit,
        )?;

        let root = self.state.fetch_root().map_err(Error::from_state_err)?;

        let transfer_value = 0;
        let obfuscated_transaction = false;

        let utx = phoenix_transaction(
            rng,
            &sender_sk,
            &change_pk,
            &receiver_pk,
            input_notes_openings,
            root,
            transfer_value,
            obfuscated_transaction,
            deposit,
            gas_limit,
            gas_price,
            Some(exec),
        );

        sender_sk.zeroize();

        PC::compute_proof_and_propagate(&utx)
            .map_err(|e| Error::from_prover_err(e))
    }

    /// Transfer Dusk in the form of Phoenix notes from one key to another.
    #[allow(clippy::too_many_arguments)]
    pub fn phoenix_transfer<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u8,
        receiver_pk: &PhoenixPublicKey,
        transfer_value: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC, PC>> {
        let mut sender_sk = self.phoenix_secret_key(sender_index)?;
        let change_pk = self.phoenix_public_key(sender_index)?;

        let input_notes_openings = self.input_notes_openings(
            &sender_sk,
            transfer_value + gas_limit * gas_price,
        )?;

        let root = self.state.fetch_root().map_err(Error::from_state_err)?;

        let obfuscated_transaction = true;
        let deposit = 0;

        let exec: Option<ContractExec> = None;

        let utx = phoenix_transaction(
            rng,
            &sender_sk,
            &change_pk,
            &receiver_pk,
            input_notes_openings,
            root,
            transfer_value,
            obfuscated_transaction,
            deposit,
            gas_limit,
            gas_price,
            exec,
        );

        sender_sk.zeroize();

        PC::compute_proof_and_propagate(&utx)
            .map_err(|e| Error::from_prover_err(e))
    }

    /// Stakes an amount of Dusk using Phoenix notes.
    #[allow(clippy::too_many_arguments)]
    pub fn phoenix_stake<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u8,
        staker_index: u8,
        stake_value: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC, PC>> {
        let mut phoenix_sender_sk = self.phoenix_secret_key(sender_index)?;
        let mut stake_sk = self.account_secret_key(staker_index)?;

        let stake_pk = BlsPublicKey::from(&stake_sk);

        let inputs = self.input_notes_openings(
            &phoenix_sender_sk,
            gas_limit * gas_price + stake_value,
        )?;

        let root = self.state.fetch_root().map_err(Error::from_state_err)?;

        let current_nonce = self
            .state
            .fetch_stake(&stake_pk)
            .map_err(Error::from_state_err)?
            .nonce;

        let utx = phoenix_stake(
            rng,
            &phoenix_sender_sk,
            &stake_sk,
            inputs,
            root,
            gas_limit,
            gas_price,
            stake_value,
            current_nonce,
        );

        stake_sk.zeroize();
        phoenix_sender_sk.zeroize();

        PC::compute_proof_and_propagate(&utx)
            .map_err(|e| Error::from_prover_err(e))
    }

    /// Unstakes a key from the stake contract, using Phoenix notes.
    pub fn phoenix_unstake<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u8,
        staker_index: u8,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC, PC>> {
        let mut phoenix_sender_sk = self.phoenix_secret_key(sender_index)?;
        let mut stake_sk = self.account_secret_key(staker_index)?;

        let stake_pk = BlsPublicKey::from(&stake_sk);

        let inputs = self.input_notes_openings_nullifiers(
            &phoenix_sender_sk,
            gas_limit * gas_price,
        )?;

        let root = self.state.fetch_root().map_err(Error::from_state_err)?;

        let stake = self
            .state
            .fetch_stake(&stake_pk)
            .map_err(Error::from_state_err)?;

        let staked_amount = stake
            .amount
            .ok_or(Error::NotStaked {
                key: stake_pk,
                stake,
            })?
            .value;

        let utx = phoenix_unstake(
            rng,
            &phoenix_sender_sk,
            &stake_sk,
            inputs,
            root,
            staked_amount,
            gas_limit,
            gas_price,
        );

        stake_sk.zeroize();
        phoenix_sender_sk.zeroize();

        PC::compute_proof_and_propagate(&utx)
            .map_err(|e| Error::from_prover_err(e))
    }

    /// Withdraw the accumulated staking reward for a key, into Phoenix notes.
    /// Rewards are accumulated by participating in the consensus.
    pub fn phoenix_stake_withdraw<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u8,
        staker_index: u8,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC, PC>> {
        let mut phoenix_sender_sk = self.phoenix_secret_key(sender_index)?;
        let mut stake_sk = self.account_secret_key(staker_index)?;

        let stake_pk = BlsPublicKey::from(&stake_sk);

        let inputs = self.input_notes_openings_nullifiers(
            &phoenix_sender_sk,
            gas_limit * gas_price,
        )?;

        let root = self.state.fetch_root().map_err(Error::from_state_err)?;

        let stake_reward = self
            .state
            .fetch_stake(&stake_pk)
            .map_err(Error::from_state_err)?
            .reward;

        let utx = phoenix_withdraw_stake_reward(
            rng,
            &phoenix_sender_sk,
            &stake_sk,
            inputs,
            root,
            stake_reward,
            gas_limit,
            gas_price,
        );

        stake_sk.zeroize();
        phoenix_sender_sk.zeroize();

        PC::compute_proof_and_propagate(&utx)
            .map_err(|e| Error::from_prover_err(e))
    }

    /// Transfer Dusk from one account to another using moonlight.
    pub fn moonlight_transfer(
        &self,
        from_index: u8,
        to_account: BlsPublicKey,
        value: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<MoonlightTransaction, Error<S, SC, PC>> {
        let deposit = 0;
        let exec: Option<ContractExec> = None;

        self.moonlight_transaction(
            from_index,
            Some(to_account),
            value,
            deposit,
            gas_limit,
            gas_price,
            exec,
        )
    }

    /// Creates a generic moonlight transaction.
    #[allow(clippy::too_many_arguments)]
    pub fn moonlight_transaction(
        &self,
        from_index: u8,
        to_account: Option<BlsPublicKey>,
        value: u64,
        deposit: u64,
        gas_limit: u64,
        gas_price: u64,
        exec: Option<impl Into<ContractExec>>,
    ) -> Result<MoonlightTransaction, Error<S, SC, PC>> {
        let mut seed = self.store.get_seed().map_err(Error::from_store_err)?;
        let mut from_sk = derive_bls_sk(&seed, from_index);
        let from_account = BlsPublicKey::from(&from_sk);

        let account = self
            .state
            .fetch_account(&from_account)
            .map_err(Error::from_state_err)?;

        // technically this check is not necessary, but it's nice to not spam
        // the network with transactions that are unspendable.
        let max_value = value + deposit + gas_limit * gas_price;
        if max_value > account.balance {
            return Err(Error::NotEnoughBalance);
        }
        let nonce = account.nonce + 1;

        let tx = MoonlightTransaction::new(
            &from_sk, to_account, value, deposit, gas_limit, gas_price, nonce,
            exec,
        );

        seed.zeroize();
        from_sk.zeroize();

        Ok(tx.into())
    }

    /// Gets the balance of a key.
    pub fn get_balance(
        &self,
        sk_index: u8,
    ) -> Result<BalanceInfo, Error<S, SC, PC>> {
        let mut seed = self.store.get_seed().map_err(Error::from_store_err)?;
        let mut phoenix_sk = derive_phoenix_sk(&seed, sk_index);
        let phoenix_vk = PhoenixViewKey::from(&phoenix_sk);

        let unspent_notes: Vec<Note> = self
            .unspent_notes_and_nullifiers(&phoenix_sk)?
            .into_iter()
            .map(|(note, _nullifier)| note)
            .collect();
        let balance = phoenix_balance(&phoenix_vk, unspent_notes);

        seed.zeroize();
        phoenix_sk.zeroize();

        Ok(balance)
    }

    /// Gets the stake and the expiration of said stake for a key.
    pub fn get_stake(
        &self,
        sk_index: u8,
    ) -> Result<StakeData, Error<S, SC, PC>> {
        let mut seed = self.store.get_seed().map_err(Error::from_store_err)?;
        let mut account_sk = derive_bls_sk(&seed, sk_index);

        let account_pk = BlsPublicKey::from(&account_sk);

        let stake = self
            .state
            .fetch_stake(&account_pk)
            .map_err(Error::from_state_err)?;

        seed.zeroize();
        account_sk.zeroize();

        Ok(stake)
    }

    /// Gets the account data for a key.
    pub fn get_account(
        &self,
        sk_index: u8,
    ) -> Result<AccountData, Error<S, SC, PC>> {
        let mut seed = self.store.get_seed().map_err(Error::from_store_err)?;
        let mut account_sk = derive_bls_sk(&seed, sk_index);

        let account_pk = BlsPublicKey::from(&account_sk);

        let account = self
            .state
            .fetch_account(&account_pk)
            .map_err(Error::from_state_err)?;

        seed.zeroize();
        account_sk.zeroize();

        Ok(account)
    }
}

/// Pick the notes to be used in a phoenix transaction from a vector of notes.
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
    notes_values_nullifiers: Vec<(Note, u64, BlsScalar)>,
) -> Vec<(Note, BlsScalar)> {
    let mut notes_values_nullifiers = notes_values_nullifiers;
    let len = notes_values_nullifiers.len();

    if len <= MAX_INPUT_NOTES {
        return notes_values_nullifiers
            .into_iter()
            .map(|(note, _value, nullifier)| (note, nullifier))
            .collect();
    }

    notes_values_nullifiers
        .sort_by(|(_, aval, _), (_, bval, _)| aval.cmp(bval));

    pick_lexicographic(notes_values_nullifiers.len(), |indices| {
        indices
            .iter()
            .map(|index| &notes_values_nullifiers[*index].1)
            .sum::<u64>()
            >= value
    })
    .map(|indices| {
        indices
            .into_iter()
            .map(|index| {
                let (note, _value, nullifier) =
                    notes_values_nullifiers[index].clone();
                (note, nullifier)
            })
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
