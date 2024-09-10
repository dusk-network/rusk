// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{StateClient, Store};

use core::convert::Infallible;

use alloc::string::FromUtf8Error;
use alloc::vec::Vec;

use dusk_bytes::Error as BytesError;
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
        data::TransactionData,
        moonlight::{AccountData, Transaction as MoonlightTransaction},
        phoenix::{
            Note, NoteLeaf, NoteOpening, PublicKey as PhoenixPublicKey,
            SecretKey as PhoenixSecretKey, ViewKey as PhoenixViewKey,
        },
        Transaction,
    },
    BlsScalar, Error as ExecutionError,
};
use rusk_prover::LocalProver;
use wallet_core::{
    keys::{derive_bls_sk, derive_phoenix_sk},
    phoenix_balance,
    transaction::{
        moonlight_stake, moonlight_stake_reward, moonlight_to_phoenix,
        moonlight_unstake, phoenix as phoenix_transaction, phoenix_stake,
        phoenix_stake_reward, phoenix_to_moonlight, phoenix_unstake,
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
pub enum Error<S: Store, SC: StateClient> {
    /// Underlying store error.
    Store(S::Error),
    /// Error originating from the state client.
    State(SC::Error),
    /// Rkyv serialization.
    Rkyv,
    /// Random number generator error.
    Rng(RngError),
    /// Serialization and deserialization of Dusk types.
    Bytes(BytesError),
    /// Bytes were meant to be utf8 but aren't.
    Utf8(FromUtf8Error),
    /// Originating from the execution-core error.
    Execution(ExecutionError),
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

impl<S: Store, SC: StateClient> Error<S, SC> {
    /// Returns an error from the underlying store error.
    pub fn from_store_err(se: S::Error) -> Self {
        Self::Store(se)
    }
    /// Returns an error from the underlying state client.
    pub fn from_state_err(se: SC::Error) -> Self {
        Self::State(se)
    }
}

impl<S: Store, SC: StateClient> From<SerializerError> for Error<S, SC> {
    fn from(_: SerializerError) -> Self {
        Self::Rkyv
    }
}

impl<C, D, S: Store, SC: StateClient> From<CheckDeserializeError<C, D>>
    for Error<S, SC>
{
    fn from(_: CheckDeserializeError<C, D>) -> Self {
        Self::Rkyv
    }
}

impl<S: Store, SC: StateClient> From<RngError> for Error<S, SC> {
    fn from(re: RngError) -> Self {
        Self::Rng(re)
    }
}

impl<S: Store, SC: StateClient> From<BytesError> for Error<S, SC> {
    fn from(be: BytesError) -> Self {
        Self::Bytes(be)
    }
}

impl<S: Store, SC: StateClient> From<FromUtf8Error> for Error<S, SC> {
    fn from(err: FromUtf8Error) -> Self {
        Self::Utf8(err)
    }
}

impl<S: Store, SC: StateClient> From<ExecutionError> for Error<S, SC> {
    fn from(ee: ExecutionError) -> Self {
        Self::Execution(ee)
    }
}

/// A wallet implementation.
///
/// This is responsible for holding the keys, and performing operations like
/// creating transactions.
pub struct Wallet<S, SC> {
    store: S,
    state: SC,
}

impl<S, SC> Wallet<S, SC> {
    /// Create a new wallet given the underlying store and node client.
    pub const fn new(store: S, state: SC) -> Self {
        Self { store, state }
    }

    /// Return the inner Store reference
    pub const fn store(&self) -> &S {
        &self.store
    }

    /// Return the inner State reference
    pub const fn state(&self) -> &SC {
        &self.state
    }
}

impl<S, SC> Wallet<S, SC>
where
    S: Store,
    SC: StateClient,
{
    /// Retrieve the secret key with the given index.
    pub fn phoenix_secret_key(
        &self,
        index: u8,
    ) -> Result<PhoenixSecretKey, Error<S, SC>> {
        self.store
            .phoenix_secret_key(index)
            .map_err(Error::from_store_err)
    }

    /// Retrieve the public key with the given index.
    pub fn phoenix_public_key(
        &self,
        index: u8,
    ) -> Result<PhoenixPublicKey, Error<S, SC>> {
        self.store
            .phoenix_public_key(index)
            .map_err(Error::from_store_err)
    }

    /// Retrieve the account secret key with the given index.
    pub fn account_secret_key(
        &self,
        index: u8,
    ) -> Result<BlsSecretKey, Error<S, SC>> {
        self.store
            .account_secret_key(index)
            .map_err(Error::from_store_err)
    }

    /// Retrieve the account public key with the given index.
    pub fn account_public_key(
        &self,
        index: u8,
    ) -> Result<BlsPublicKey, Error<S, SC>> {
        self.store
            .account_public_key(index)
            .map_err(Error::from_store_err)
    }

    /// Fetches the notes and nullifiers in the state and returns the notes that
    /// are still available for spending.
    fn unspent_notes_and_nullifiers(
        &self,
        sk: &PhoenixSecretKey,
    ) -> Result<Vec<(NoteLeaf, BlsScalar)>, Error<S, SC>> {
        let vk = PhoenixViewKey::from(sk);

        let note_leaves =
            self.state.fetch_notes(&vk).map_err(Error::from_state_err)?;

        let nullifiers: Vec<_> = note_leaves
            .iter()
            .map(|(note, _bh)| note.gen_nullifier(sk))
            .collect();

        let existing_nullifiers = self
            .state
            .fetch_existing_nullifiers(&nullifiers)
            .map_err(Error::from_state_err)?;

        let unspent_notes_and_nullifiers = note_leaves
            .into_iter()
            .zip(nullifiers.into_iter())
            .filter(|(_note, nullifier)| {
                !existing_nullifiers.contains(nullifier)
            })
            .map(|((note, block_height), nullifier)| {
                (NoteLeaf { note, block_height }, nullifier)
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
    ) -> Result<Vec<(Note, BlsScalar)>, Error<S, SC>> {
        let sender_vk = PhoenixViewKey::from(sender_sk);

        // decrypt the value of all unspent note
        let unspent_notes_nullifiers =
            self.unspent_notes_and_nullifiers(sender_sk)?;
        let mut notes_values_nullifiers =
            Vec::with_capacity(unspent_notes_nullifiers.len());

        let mut accumulated_value = 0;
        for (note_leaf, nullifier) in unspent_notes_nullifiers {
            let val = note_leaf
                .note
                .value(Some(&sender_vk))
                .map_err(|_| ExecutionError::PhoenixOwnership)?;
            accumulated_value += val;
            notes_values_nullifiers.push((note_leaf.note, val, nullifier));
        }

        if accumulated_value < transaction_cost {
            return Err(ExecutionError::InsufficientBalance.into());
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
    ) -> Result<Vec<(Note, NoteOpening, BlsScalar)>, Error<S, SC>> {
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
    ) -> Result<Vec<(Note, NoteOpening)>, Error<S, SC>> {
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
        data: impl Into<TransactionData>,
    ) -> Result<Transaction, Error<S, SC>> {
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

        let chain_id =
            self.state.fetch_chain_id().map_err(Error::from_state_err)?;

        let tx = phoenix_transaction(
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
            chain_id,
            Some(data),
            &LocalProver,
        )?;

        sender_sk.zeroize();

        Ok(tx)
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
    ) -> Result<Transaction, Error<S, SC>> {
        let mut sender_sk = self.phoenix_secret_key(sender_index)?;
        let change_pk = self.phoenix_public_key(sender_index)?;

        let input_notes_openings = self.input_notes_openings(
            &sender_sk,
            transfer_value + gas_limit * gas_price,
        )?;

        let root = self.state.fetch_root().map_err(Error::from_state_err)?;

        let obfuscated_transaction = true;
        let deposit = 0;

        let data: Option<TransactionData> = None;

        let chain_id =
            self.state.fetch_chain_id().map_err(Error::from_state_err)?;

        let tx = phoenix_transaction(
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
            chain_id,
            data,
            &LocalProver,
        )?;

        sender_sk.zeroize();

        Ok(tx)
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
    ) -> Result<Transaction, Error<S, SC>> {
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

        let chain_id =
            self.state.fetch_chain_id().map_err(Error::from_state_err)?;

        let tx = phoenix_stake(
            rng,
            &phoenix_sender_sk,
            &stake_sk,
            inputs,
            root,
            gas_limit,
            gas_price,
            chain_id,
            stake_value,
            current_nonce,
            &LocalProver,
        )?;

        stake_sk.zeroize();
        phoenix_sender_sk.zeroize();

        Ok(tx)
    }

    /// Unstakes a key from the stake contract, using Phoenix notes.
    pub fn phoenix_unstake<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u8,
        staker_index: u8,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC>> {
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

        let chain_id =
            self.state.fetch_chain_id().map_err(Error::from_state_err)?;

        let tx = phoenix_unstake(
            rng,
            &phoenix_sender_sk,
            &stake_sk,
            inputs,
            root,
            staked_amount,
            gas_limit,
            gas_price,
            chain_id,
            &LocalProver,
        )?;

        stake_sk.zeroize();
        phoenix_sender_sk.zeroize();

        Ok(tx)
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
    ) -> Result<Transaction, Error<S, SC>> {
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

        let chain_id =
            self.state.fetch_chain_id().map_err(Error::from_state_err)?;

        let tx = phoenix_stake_reward(
            rng,
            &phoenix_sender_sk,
            &stake_sk,
            inputs,
            root,
            stake_reward,
            gas_limit,
            gas_price,
            chain_id,
            &LocalProver,
        )?;

        stake_sk.zeroize();
        phoenix_sender_sk.zeroize();

        Ok(tx)
    }

    /// Convert some Phoenix Dusk into Moonlight Dusk.
    pub fn phoenix_to_moonlight<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        phoenix_sender_index: u8,
        moonlight_receiver_index: u8,
        convert_value: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC>> {
        let mut phoenix_sender_sk =
            self.phoenix_secret_key(phoenix_sender_index)?;
        let mut moonlight_receiver_sk =
            self.account_secret_key(moonlight_receiver_index)?;

        let inputs = self.input_notes_openings_nullifiers(
            &phoenix_sender_sk,
            gas_limit * gas_price,
        )?;

        let root = self.state.fetch_root().map_err(Error::from_state_err)?;

        let chain_id =
            self.state.fetch_chain_id().map_err(Error::from_state_err)?;

        let tx = phoenix_to_moonlight(
            rng,
            &phoenix_sender_sk,
            &moonlight_receiver_sk,
            inputs,
            root,
            convert_value,
            gas_limit,
            gas_price,
            chain_id,
            &LocalProver,
        )?;

        phoenix_sender_sk.zeroize();
        moonlight_receiver_sk.zeroize();

        Ok(tx)
    }

    /// Transfer Dusk from one account to another using moonlight.
    pub fn moonlight_transfer(
        &self,
        from_index: u8,
        to_account: BlsPublicKey,
        value: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC>> {
        let deposit = 0;
        let data: Option<TransactionData> = None;

        self.moonlight_transaction(
            from_index,
            Some(to_account),
            value,
            deposit,
            gas_limit,
            gas_price,
            data,
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
        data: Option<impl Into<TransactionData>>,
    ) -> Result<Transaction, Error<S, SC>> {
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
            return Err(ExecutionError::InsufficientBalance.into());
        }
        let nonce = account.nonce + 1;

        let chain_id =
            self.state.fetch_chain_id().map_err(Error::from_state_err)?;

        let tx = MoonlightTransaction::new(
            &from_sk, to_account, value, deposit, gas_limit, gas_price, nonce,
            chain_id, data,
        )?;

        seed.zeroize();
        from_sk.zeroize();

        Ok(tx.into())
    }

    /// Stakes an amount of Dusk using a Moonlight account.
    #[allow(clippy::too_many_arguments)]
    pub fn moonlight_stake(
        &self,
        sender_index: u8,
        staker_index: u8,
        stake_value: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC>> {
        let mut sender_sk = self.account_secret_key(sender_index)?;
        let sender_pk = self.account_public_key(sender_index)?;

        let mut staker_sk = self.account_secret_key(staker_index)?;
        let staker_pk = self.account_public_key(staker_index)?;

        let sender_account = self
            .state
            .fetch_account(&sender_pk)
            .map_err(Error::from_state_err)?;
        let staker_data = self
            .state
            .fetch_stake(&staker_pk)
            .map_err(Error::from_state_err)?;

        let chain_id =
            self.state.fetch_chain_id().map_err(Error::from_state_err)?;

        let tx = moonlight_stake(
            &sender_sk,
            &staker_sk,
            stake_value,
            gas_limit,
            gas_price,
            sender_account.nonce,
            staker_data.nonce,
            chain_id,
        )?;

        sender_sk.zeroize();
        staker_sk.zeroize();

        Ok(tx)
    }

    /// Unstakes a key from the stake contract, using a Moonlight account.
    pub fn moonlight_unstake<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u8,
        staker_index: u8,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC>> {
        let mut sender_sk = self.account_secret_key(sender_index)?;
        let sender_pk = self.account_public_key(sender_index)?;

        let mut staker_sk = self.account_secret_key(staker_index)?;
        let staker_pk = self.account_public_key(staker_index)?;

        let sender_account = self
            .state
            .fetch_account(&sender_pk)
            .map_err(Error::from_state_err)?;
        let staker_data = self
            .state
            .fetch_stake(&staker_pk)
            .map_err(Error::from_state_err)?;

        let unstake_value = staker_data
            .amount
            .ok_or(Error::NotStaked {
                key: staker_pk,
                stake: staker_data,
            })?
            .value;
        let chain_id =
            self.state.fetch_chain_id().map_err(Error::from_state_err)?;

        let tx = moonlight_unstake(
            rng,
            &sender_sk,
            &staker_sk,
            unstake_value,
            gas_limit,
            gas_price,
            sender_account.nonce,
            chain_id,
        )?;

        sender_sk.zeroize();
        staker_sk.zeroize();

        Ok(tx)
    }

    /// Withdraw the accumulated staking reward for a key, into a Moonlight
    /// notes. Rewards are accumulated by participating in the consensus.
    pub fn moonlight_stake_withdraw<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u8,
        staker_index: u8,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC>> {
        let mut sender_sk = self.account_secret_key(sender_index)?;
        let sender_pk = self.account_public_key(sender_index)?;

        let mut staker_sk = self.account_secret_key(staker_index)?;
        let staker_pk = self.account_public_key(staker_index)?;

        let sender_account = self
            .state
            .fetch_account(&sender_pk)
            .map_err(Error::from_state_err)?;
        let staker_data = self
            .state
            .fetch_stake(&staker_pk)
            .map_err(Error::from_state_err)?;

        let chain_id =
            self.state.fetch_chain_id().map_err(Error::from_state_err)?;

        let tx = moonlight_stake_reward(
            rng,
            &sender_sk,
            &staker_sk,
            staker_data.reward,
            gas_limit,
            gas_price,
            sender_account.nonce,
            chain_id,
        )?;

        sender_sk.zeroize();
        staker_sk.zeroize();

        Ok(tx)
    }

    /// Convert some Moonlight Dusk into Phoenix Dusk.
    pub fn moonlight_to_phoenix<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        moonlight_sender_index: u8,
        phoenix_receiver_index: u8,
        convert_value: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC>> {
        let mut moonlight_sender_sk =
            self.account_secret_key(moonlight_sender_index)?;
        let moonlight_sender_pk =
            self.account_public_key(moonlight_sender_index)?;
        let mut phoenix_receiver_sk =
            self.phoenix_secret_key(phoenix_receiver_index)?;

        let moonlight_sender_account = self
            .state
            .fetch_account(&moonlight_sender_pk)
            .map_err(Error::from_state_err)?;

        let nonce = moonlight_sender_account.nonce;

        let chain_id =
            self.state.fetch_chain_id().map_err(Error::from_state_err)?;

        let tx = moonlight_to_phoenix(
            rng,
            &moonlight_sender_sk,
            &phoenix_receiver_sk,
            convert_value,
            gas_limit,
            gas_price,
            nonce,
            chain_id,
        )?;

        moonlight_sender_sk.zeroize();
        phoenix_receiver_sk.zeroize();

        Ok(tx)
    }

    /// Gets the balance of a key.
    pub fn get_balance(
        &self,
        sk_index: u8,
    ) -> Result<BalanceInfo, Error<S, SC>> {
        let mut seed = self.store.get_seed().map_err(Error::from_store_err)?;
        let mut phoenix_sk = derive_phoenix_sk(&seed, sk_index);
        let phoenix_vk = PhoenixViewKey::from(&phoenix_sk);

        let unspent_notes: Vec<NoteLeaf> = self
            .unspent_notes_and_nullifiers(&phoenix_sk)?
            .into_iter()
            .map(|(note_leaf, _nul)| note_leaf)
            .collect();
        let balance = phoenix_balance(&phoenix_vk, unspent_notes.iter());

        seed.zeroize();
        phoenix_sk.zeroize();

        Ok(balance)
    }

    /// Gets the stake and the expiration of said stake for a key.
    pub fn get_stake(&self, sk_index: u8) -> Result<StakeData, Error<S, SC>> {
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
    ) -> Result<AccountData, Error<S, SC>> {
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
