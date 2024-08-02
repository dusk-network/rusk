// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{BalanceInfo, ProverClient, StateClient, Store};

use core::convert::Infallible;

use alloc::string::{FromUtf8Error, String};
use alloc::vec::Vec;

use dusk_bytes::Error as BytesError;
use execution_core::{
    stake::{Stake, StakeData, Withdraw as StakeWithdraw},
    transfer::{
        AccountData, ContractCall, ContractDeploy, ContractExec, Fee,
        MoonlightPayload, PhoenixPayload, Transaction, Withdraw,
        WithdrawReceiver, WithdrawReplayToken,
    },
    BlsPublicKey, BlsScalar, JubJubScalar, Note, PhoenixError, PublicKey,
    SchnorrSecretKey, SecretKey, TxSkeleton, ViewKey, OUTPUT_NOTES,
};
use ff::Field;
use rand_core::{CryptoRng, Error as RngError, RngCore};
use rkyv::ser::serializers::{
    AllocScratchError, CompositeSerializerError, SharedSerializeMapError,
};
use rkyv::validation::validators::CheckDeserializeError;
use rusk_prover::{UnprovenTransaction, UnprovenTransactionInput};

const MAX_INPUT_NOTES: usize = 4;
const SCRATCH_SIZE: usize = 2048;

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
    /// stake contract and the user tries to make a withdraw transaction.
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
    /// Retrieve the public key with the given index.
    pub fn public_key(
        &self,
        index: u64,
    ) -> Result<PublicKey, Error<S, SC, PC>> {
        self.store
            .fetch_secret_key(index)
            .map(|sk| PublicKey::from(&sk))
            .map_err(Error::from_store_err)
    }

    /// Retrieve the account public key with the given index.
    pub fn account_public_key(
        &self,
        index: u64,
    ) -> Result<BlsPublicKey, Error<S, SC, PC>> {
        self.store
            .fetch_account_secret_key(index)
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

    fn phoenix_transaction<Rng, MaybeExec>(
        &self,
        rng: &mut Rng,
        sender_sk: &SecretKey,
        receiver_pk: &PublicKey,
        value: u64,
        gas_limit: u64,
        gas_price: u64,
        deposit: u64,
        exec: MaybeExec,
    ) -> Result<Transaction, Error<S, SC, PC>>
    where
        Rng: RngCore + CryptoRng,
        MaybeExec: MaybePhoenixExec<Rng>,
    {
        let sender_pk = PublicKey::from(sender_sk);

        let (inputs, outputs) = self.inputs_and_change_output(
            rng,
            &sender_sk,
            &sender_pk,
            &receiver_pk,
            value,
            gas_limit * gas_price,
            deposit,
        )?;

        let fee = Fee::new(rng, &sender_pk, gas_limit, gas_price);
        let contract_call = exec.maybe_phoenix_exec(
            rng,
            inputs.iter().map(|(n, _, _)| n.clone()).collect(),
        );

        let utx = new_unproven_tx(
            rng,
            &self.state,
            &sender_sk,
            inputs,
            outputs,
            fee,
            deposit,
            contract_call,
        )
        .map_err(Error::from_state_err)?;

        self.prover
            .compute_proof_and_propagate(&utx)
            .map_err(Error::from_prover_err)
    }

    /// Execute a generic contract call or deployment, using Phoenix notes to
    /// pay for gas.
    #[allow(clippy::too_many_arguments)]
    pub fn phoenix_execute<Rng>(
        &self,
        rng: &mut Rng,
        exec: impl Into<ContractExec>,
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
            .fetch_secret_key(sender_index)
            .map_err(Error::from_store_err)?;
        let receiver_pk = PublicKey::from(&sender_sk);

        self.phoenix_transaction(
            rng,
            &sender_sk,
            &receiver_pk,
            0,
            gas_limit,
            gas_price,
            deposit,
            exec.into(),
        )
    }

    /// Transfer Dusk in the form of Phoenix notes from one key to another.
    #[allow(clippy::too_many_arguments)]
    pub fn phoenix_transfer<Rng: RngCore + CryptoRng>(
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
            .fetch_secret_key(sender_index)
            .map_err(Error::from_store_err)?;

        self.phoenix_transaction(
            rng,
            &sender_sk,
            receiver_pk,
            value,
            gas_limit,
            gas_price,
            0,
            None,
        )
    }

    /// Stakes an amount of Dusk using Phoenix notes.
    #[allow(clippy::too_many_arguments)]
    pub fn phoenix_stake<Rng: RngCore + CryptoRng>(
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
            .fetch_secret_key(sender_index)
            .map_err(Error::from_store_err)?;
        let receiver_pk = PublicKey::from(&sender_sk);

        let stake_sk = self
            .store
            .fetch_account_secret_key(staker_index)
            .map_err(Error::from_store_err)?;
        let stake_pk = BlsPublicKey::from(&stake_sk);

        let stake = self
            .state
            .fetch_stake(&stake_pk)
            .map_err(Error::from_state_err)?;

        let stake = Stake::new(&stake_sk, value, stake.nonce + 1);

        let stake_bytes = rkyv::to_bytes::<_, SCRATCH_SIZE>(&stake)
            .expect("Should serialize Stake correctly")
            .to_vec();

        let contract_call = ContractCall {
            contract: rusk_abi::STAKE_CONTRACT.to_bytes(),
            fn_name: String::from(TX_STAKE),
            fn_args: stake_bytes,
        };

        self.phoenix_transaction(
            rng,
            &sender_sk,
            &receiver_pk,
            0,
            gas_limit,
            gas_price,
            value,
            Some(ContractExec::Call(contract_call)),
        )
    }

    /// Unstakes a key from the stake contract, using Phoenix notes.
    pub fn phoenix_unstake<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u64,
        staker_index: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC, PC>> {
        let sender_sk = self
            .store
            .fetch_secret_key(sender_index)
            .map_err(Error::from_store_err)?;
        let receiver_pk = PublicKey::from(&sender_sk);

        let stake_sk = self
            .store
            .fetch_account_secret_key(staker_index)
            .map_err(Error::from_store_err)?;
        let stake_pk = BlsPublicKey::from(&stake_sk);

        let stake = self
            .state
            .fetch_stake(&stake_pk)
            .map_err(Error::from_state_err)?;

        let amount = stake.amount.ok_or(Error::NotStaked {
            key: stake_pk,
            stake,
        })?;

        self.phoenix_transaction(
            rng,
            &sender_sk,
            &receiver_pk,
            0,
            gas_limit,
            gas_price,
            0,
            |rng: &mut Rng, input_notes: Vec<Note>| {
                let address = receiver_pk
                    .gen_stealth_address(&JubJubScalar::random(&mut *rng));
                let note_sk = sender_sk.gen_note_sk(&address);

                let nullifiers = input_notes
                    .into_iter()
                    .map(|note| note.gen_nullifier(&sender_sk))
                    .collect();

                let withdraw = Withdraw::new(
                    rng,
                    &note_sk,
                    rusk_abi::STAKE_CONTRACT.to_bytes(),
                    amount.value,
                    WithdrawReceiver::Phoenix(address),
                    WithdrawReplayToken::Phoenix(nullifiers),
                );

                let withdraw = StakeWithdraw::new(&stake_sk, withdraw);

                let withdraw_bytes =
                    rkyv::to_bytes::<_, SCRATCH_SIZE>(&withdraw)
                        .expect("Serializing Withdraw should succeed")
                        .to_vec();

                ContractCall {
                    contract: rusk_abi::STAKE_CONTRACT.to_bytes(),
                    fn_name: String::from(TX_UNSTAKE),
                    fn_args: withdraw_bytes,
                }
            },
        )
    }

    /// Withdraw the accumulated staking reward for a key, into Phoenix notes.
    /// Rewards are accumulated by participating in the consensus.
    pub fn phoenix_withdraw<Rng: RngCore + CryptoRng>(
        &self,
        rng: &mut Rng,
        sender_index: u64,
        staker_index: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC, PC>> {
        let sender_sk = self
            .store
            .fetch_secret_key(sender_index)
            .map_err(Error::from_store_err)?;
        let receiver_pk = PublicKey::from(&sender_sk);

        let stake_sk = self
            .store
            .fetch_account_secret_key(staker_index)
            .map_err(Error::from_store_err)?;
        let stake_pk = BlsPublicKey::from(&stake_sk);

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

        self.phoenix_transaction(
            rng,
            &sender_sk,
            &receiver_pk,
            0,
            gas_limit,
            gas_price,
            0,
            |rng: &mut Rng, input_notes: Vec<Note>| {
                let address = receiver_pk
                    .gen_stealth_address(&JubJubScalar::random(&mut *rng));
                let note_sk = sender_sk.gen_note_sk(&address);

                let nullifiers = input_notes
                    .into_iter()
                    .map(|note| note.gen_nullifier(&sender_sk))
                    .collect();

                let withdraw = Withdraw::new(
                    rng,
                    &note_sk,
                    rusk_abi::STAKE_CONTRACT.to_bytes(),
                    stake.reward,
                    WithdrawReceiver::Phoenix(address),
                    WithdrawReplayToken::Phoenix(nullifiers),
                );

                let unstake = StakeWithdraw::new(&stake_sk, withdraw);

                let unstake_bytes = rkyv::to_bytes::<_, SCRATCH_SIZE>(&unstake)
                    .expect("Serializing Withdraw should succeed")
                    .to_vec();

                ContractCall {
                    contract: rusk_abi::STAKE_CONTRACT.to_bytes(),
                    fn_name: String::from(TX_WITHDRAW),
                    fn_args: unstake_bytes,
                }
            },
        )
    }

    /// Transfer Dusk from one key to another using moonlight.
    pub fn moonlight_transfer(
        &self,
        from: u64,
        to: BlsPublicKey,
        value: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Transaction, Error<S, SC, PC>> {
        self.moonlight_transaction(
            from,
            Some(to),
            value,
            0,
            gas_limit,
            gas_price,
            None::<ContractExec>,
        )
    }

    /// Creates a generic moonlight transaction.
    #[allow(clippy::too_many_arguments)]
    pub fn moonlight_transaction(
        &self,
        from: u64,
        to: Option<BlsPublicKey>,
        value: u64,
        deposit: u64,
        gas_limit: u64,
        gas_price: u64,
        exec: Option<impl Into<ContractExec>>,
    ) -> Result<Transaction, Error<S, SC, PC>> {
        let from_sk = self
            .store
            .fetch_account_secret_key(from)
            .map_err(Error::from_store_err)?;

        let from = BlsPublicKey::from(&from_sk);

        let account = self
            .state
            .fetch_account(&from)
            .map_err(Error::from_state_err)?;

        // technically this check is not necessary, but it's nice to not spam
        // the network with transactions that are unspendable.
        let max_value = value + deposit + gas_limit * gas_price;
        if max_value > account.balance {
            return Err(Error::NotEnoughBalance);
        }
        let nonce = account.nonce + 1;

        let payload = MoonlightPayload {
            from,
            to,
            value,
            deposit,
            gas_limit,
            gas_price,
            nonce,
            exec: exec.map(Into::into),
        };

        let digest = payload.to_hash_input_bytes();
        let signature = from_sk.sign(&digest);

        Ok(Transaction::moonlight(payload, signature))
    }

    /// Gets the balance of a key.
    pub fn get_balance(
        &self,
        sk_index: u64,
    ) -> Result<BalanceInfo, Error<S, SC, PC>> {
        let sender_sk = self
            .store
            .fetch_secret_key(sk_index)
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
    ) -> Result<StakeData, Error<S, SC, PC>> {
        let account_sk = self
            .store
            .fetch_account_secret_key(sk_index)
            .map_err(Error::from_store_err)?;

        let account_pk = BlsPublicKey::from(&account_sk);

        let stake = self
            .state
            .fetch_stake(&account_pk)
            .map_err(Error::from_state_err)?;

        Ok(stake)
    }

    /// Gets the account data for a key.
    pub fn get_account(
        &self,
        sk_index: u64,
    ) -> Result<AccountData, Error<S, SC, PC>> {
        let account_sk = self
            .store
            .fetch_account_secret_key(sk_index)
            .map_err(Error::from_store_err)?;

        let account_pk = BlsPublicKey::from(&account_sk);

        let account = self
            .state
            .fetch_account(&account_pk)
            .map_err(Error::from_state_err)?;

        Ok(account)
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
    exec: Option<impl Into<ContractExec>>,
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

    let payload = PhoenixPayload {
        tx_skeleton,
        fee,
        exec: exec.map(Into::into),
    };
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

/// Optionally produces contract calls/executions for Phoenix transactions.
trait MaybePhoenixExec<R> {
    fn maybe_phoenix_exec(
        self,
        rng: &mut R,
        inputs: Vec<Note>,
    ) -> Option<ContractExec>;
}

impl<R> MaybePhoenixExec<R> for Option<ContractExec> {
    fn maybe_phoenix_exec(
        self,
        _rng: &mut R,
        _inputs: Vec<Note>,
    ) -> Option<ContractExec> {
        self
    }
}

impl<R> MaybePhoenixExec<R> for ContractExec {
    fn maybe_phoenix_exec(
        self,
        _rng: &mut R,
        _inputs: Vec<Note>,
    ) -> Option<ContractExec> {
        Some(self)
    }
}

impl<R> MaybePhoenixExec<R> for ContractCall {
    fn maybe_phoenix_exec(
        self,
        _rng: &mut R,
        _inputs: Vec<Note>,
    ) -> Option<ContractExec> {
        Some(ContractExec::Call(self))
    }
}

impl<R> MaybePhoenixExec<R> for ContractDeploy {
    fn maybe_phoenix_exec(
        self,
        _rng: &mut R,
        _inputs: Vec<Note>,
    ) -> Option<ContractExec> {
        Some(ContractExec::Deploy(self))
    }
}

impl<R, F, M> MaybePhoenixExec<R> for F
where
    F: FnOnce(&mut R, Vec<Note>) -> M,
    M: MaybePhoenixExec<R>,
{
    fn maybe_phoenix_exec(
        self,
        rng: &mut R,
        inputs: Vec<Note>,
    ) -> Option<ContractExec> {
        // NOTE: it may be (pun intended) possible to get rid of this clone if
        // we use a lifetime into a slice of `Note`s. However, it comes at the
        // cost of clarity. This is more important here, since this is testing
        // infrastructure, and not production code.
        let maybe = self(rng, inputs.clone());
        maybe.maybe_phoenix_exec(rng, inputs)
    }
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
