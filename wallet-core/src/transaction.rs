// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Implementations of basic wallet functionalities to create transactions.

use alloc::vec::Vec;
use core::fmt::Debug;

use rand::{CryptoRng, RngCore};

use ff::Field;
use poseidon_merkle::Opening;
use zeroize::Zeroize;

use execution_core::{
    signatures::bls::SecretKey as BlsSecretKey,
    stake::{Stake, Withdraw as StakeWithdraw, STAKE_CONTRACT},
    transfer::{
        contract_exec::{ContractCall, ContractExec},
        phoenix::{
            Note, Prove, PublicKey as PhoenixPublicKey,
            SecretKey as PhoenixSecretKey, Transaction as PhoenixTransaction,
            NOTES_TREE_DEPTH,
        },
        withdraw::{Withdraw, WithdrawReceiver, WithdrawReplayToken},
        Transaction,
    },
    BlsScalar, ContractId, JubJubScalar,
};

/// Create a [`Transaction`] that is paid in phoenix-notes.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::module_name_repetitions)]
pub fn proven_phoenix_transaction<R: RngCore + CryptoRng, P: Prove>(
    rng: &mut R,
    sender_sk: &PhoenixSecretKey,
    change_pk: &PhoenixPublicKey,
    receiver_pk: &PhoenixPublicKey,
    inputs: Vec<(Note, Opening<(), NOTES_TREE_DEPTH>)>,
    root: BlsScalar,
    transfer_value: u64,
    obfuscated_transaction: bool,
    deposit: u64,
    gas_limit: u64,
    gas_price: u64,
    exec: Option<impl Into<ContractExec>>,
) -> Transaction
where
    <P as Prove>::Error: Debug,
{
    PhoenixTransaction::new::<R, P>(
        rng,
        sender_sk,
        change_pk,
        receiver_pk,
        inputs,
        root,
        transfer_value,
        obfuscated_transaction,
        deposit,
        gas_limit,
        gas_price,
        exec,
    )
    .into()
}

/// An unproven-transaction is nearly identical to a [`PhoenixTransaction`] with
/// the only difference being that it carries a serialized [`TxCircuitVec`]
/// instead of the proof bytes.
/// This way it is possible to delegate the proof generation of the
/// [`TxCircuitVec`] after the unproven transaction was created while at the
/// same time ensuring non-malleability of the transaction, as the transaction's
/// payload-hash is part of the public inputs of the circuit.
/// Once the proof is generated from the [`TxCircuitVec`] bytes, it can
/// replace the serialized circuit in the transaction by calling
/// [`Transaction::replace_proof`].
#[allow(clippy::too_many_arguments)]
#[allow(clippy::module_name_repetitions)]
pub fn phoenix_transaction<R: RngCore + CryptoRng>(
    rng: &mut R,
    sender_sk: &PhoenixSecretKey,
    change_pk: &PhoenixPublicKey,
    receiver_pk: &PhoenixPublicKey,
    inputs: Vec<(Note, Opening<(), NOTES_TREE_DEPTH>)>,
    root: BlsScalar,
    transfer_value: u64,
    obfuscated_transaction: bool,
    deposit: u64,
    gas_limit: u64,
    gas_price: u64,
    exec: Option<impl Into<ContractExec>>,
) -> PhoenixTransaction {
    PhoenixTransaction::new::<R, UnprovenProver>(
        rng,
        sender_sk,
        change_pk,
        receiver_pk,
        inputs,
        root,
        transfer_value,
        obfuscated_transaction,
        deposit,
        gas_limit,
        gas_price,
        exec,
    )
}

/// Implementation of the Prove trait that adds the serialized circuit instead
/// of a proof. This way the proof creation can be delegated to a 3rd party.
struct UnprovenProver();

impl Prove for UnprovenProver {
    // this implementation of the trait will never error.
    type Error = ();

    fn prove(circuit: &[u8]) -> Result<Vec<u8>, Self::Error> {
        Ok(circuit.to_vec())
    }
}

/// Create a [`Transaction`] to stake from phoenix-notes.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::missing_panics_doc)]
pub fn phoenix_stake<R: RngCore + CryptoRng>(
    rng: &mut R,
    phoenix_sender_sk: &PhoenixSecretKey,
    stake_sk: &BlsSecretKey,
    inputs: Vec<(Note, Opening<(), NOTES_TREE_DEPTH>)>,
    root: BlsScalar,
    gas_limit: u64,
    gas_price: u64,
    stake_value: u64,
    current_nonce: u64,
) -> PhoenixTransaction {
    let receiver_pk = PhoenixPublicKey::from(phoenix_sender_sk);
    let change_pk = receiver_pk;

    let transfer_value = 0;
    let obfuscated_transaction = false;
    let deposit = stake_value;

    let stake = Stake::new(stake_sk, stake_value, current_nonce + 1);

    let contract_call = ContractCall::new(STAKE_CONTRACT, "stake", &stake)
        .expect("rkyv serialization of the stake struct should work.");

    phoenix_transaction(
        rng,
        phoenix_sender_sk,
        &change_pk,
        &receiver_pk,
        inputs,
        root,
        transfer_value,
        obfuscated_transaction,
        deposit,
        gas_limit,
        gas_price,
        Some(contract_call),
    )
}

/// Create an unproven [`Transaction`] to withdraw stake rewards into a
/// phoenix-note.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::missing_panics_doc)]
pub fn phoenix_withdraw_stake_reward<R: RngCore + CryptoRng>(
    rng: &mut R,
    phoenix_sender_sk: &PhoenixSecretKey,
    stake_sk: &BlsSecretKey,
    inputs: Vec<(Note, Opening<(), NOTES_TREE_DEPTH>, BlsScalar)>,
    root: BlsScalar,
    reward_amount: u64,
    gas_limit: u64,
    gas_price: u64,
) -> PhoenixTransaction {
    let receiver_pk = PhoenixPublicKey::from(phoenix_sender_sk);
    let change_pk = receiver_pk;

    let transfer_value = 0;
    let obfuscated_transaction = false;
    let deposit = 0;

    // split the input notes and openings from the nullifiers
    let mut nullifiers = Vec::with_capacity(inputs.len());
    let inputs = inputs
        .into_iter()
        .map(|(note, opening, nullifier)| {
            nullifiers.push(nullifier);
            (note, opening)
        })
        .collect();

    let gas_payment_token = WithdrawReplayToken::Phoenix(nullifiers);

    let withdraw = withdraw_to_phoenix(
        rng,
        phoenix_sender_sk,
        STAKE_CONTRACT,
        gas_payment_token,
        reward_amount,
    );

    let reward_withdraw = StakeWithdraw::new(stake_sk, withdraw);

    let contract_call =
        ContractCall::new(STAKE_CONTRACT, "withdraw", &reward_withdraw)
            .expect("rkyv should serialize the reward_withdraw correctly");

    phoenix_transaction(
        rng,
        phoenix_sender_sk,
        &change_pk,
        &receiver_pk,
        inputs,
        root,
        transfer_value,
        obfuscated_transaction,
        deposit,
        gas_limit,
        gas_price,
        Some(contract_call),
    )
}

/// Create an unproven [`Transaction`] to unstake into a phoenix-note.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::missing_panics_doc)]
pub fn phoenix_unstake<R: RngCore + CryptoRng>(
    rng: &mut R,
    phoenix_sender_sk: &PhoenixSecretKey,
    stake_sk: &BlsSecretKey,
    inputs: Vec<(Note, Opening<(), NOTES_TREE_DEPTH>, BlsScalar)>,
    root: BlsScalar,
    unstake_value: u64,
    gas_limit: u64,
    gas_price: u64,
) -> PhoenixTransaction {
    let receiver_pk = PhoenixPublicKey::from(phoenix_sender_sk);
    let change_pk = receiver_pk;

    let transfer_value = 0;
    let obfuscated_transaction = false;
    let deposit = 0;

    // split the input notes and openings from the nullifiers
    let mut nullifiers = Vec::with_capacity(inputs.len());
    let inputs = inputs
        .into_iter()
        .map(|(note, opening, nullifier)| {
            nullifiers.push(nullifier);
            (note, opening)
        })
        .collect();

    let gas_payment_token = WithdrawReplayToken::Phoenix(nullifiers);

    let withdraw = withdraw_to_phoenix(
        rng,
        phoenix_sender_sk,
        STAKE_CONTRACT,
        gas_payment_token,
        unstake_value,
    );

    let unstake = StakeWithdraw::new(stake_sk, withdraw);

    let contract_call = ContractCall::new(STAKE_CONTRACT, "unstake", &unstake)
        .expect("unstake should serialize correctly");

    phoenix_transaction(
        rng,
        phoenix_sender_sk,
        &change_pk,
        &receiver_pk,
        inputs,
        root,
        transfer_value,
        obfuscated_transaction,
        deposit,
        gas_limit,
        gas_price,
        Some(contract_call),
    )
}

/// Create a [`Withdraw`] struct to be used to withdraw funds from a contract
/// into a phoenix-note.
///
/// The gas payment can be done by either phoenix or moonlight by setting the
/// `gas_payment_token` accordingly.
fn withdraw_to_phoenix<R: RngCore + CryptoRng>(
    rng: &mut R,
    receiver_sk: &PhoenixSecretKey,
    contract: impl Into<ContractId>,
    gas_payment_token: WithdrawReplayToken,
    value: u64,
) -> Withdraw {
    let withdraw_address = PhoenixPublicKey::from(receiver_sk)
        .gen_stealth_address(&JubJubScalar::random(&mut *rng));
    let mut withdraw_note_sk = receiver_sk.gen_note_sk(&withdraw_address);

    let withdraw = Withdraw::new(
        rng,
        &withdraw_note_sk,
        contract.into(),
        value,
        WithdrawReceiver::Phoenix(withdraw_address),
        gas_payment_token,
    );

    withdraw_note_sk.zeroize();

    withdraw
}
