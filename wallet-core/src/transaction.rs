// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Implementations of basic wallet functionalities to create transactions.

use alloc::vec::Vec;

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
    BlsScalar, ContractId, Error, JubJubScalar,
};

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
///
/// # Errors
/// The creation of a transaction is not possible and will error if:
/// - one of the input-notes doesn't belong to the `phoenix_sender_sk`
/// - the transaction input doesn't cover the transaction costs
/// - the `inputs` vector is either empty or larger than 4 elements
/// - the `inputs` vector contains duplicate `Note`s
/// - the `Prove` trait is implemented incorrectly
#[allow(clippy::too_many_arguments)]
pub fn phoenix<R: RngCore + CryptoRng, P: Prove>(
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
    chain_id: u8,
    exec: Option<impl Into<ContractExec>>,
) -> Result<Transaction, Error> {
    Ok(PhoenixTransaction::new::<R, P>(
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
        chain_id,
        exec,
    )?
    .into())
}

/// Create a [`Transaction`] to stake from phoenix-notes.
///
/// # Errors
/// The creation of a transaction is not possible and will error if:
/// - one of the input-notes doesn't belong to the `phoenix_sender_sk`
/// - the transaction input doesn't cover the transaction costs
/// - the `inputs` vector is either empty or larger than 4 elements
/// - the `inputs` vector contains duplicate `Note`s
/// - the `Prove` trait is implemented incorrectly
#[allow(clippy::too_many_arguments)]
#[allow(clippy::missing_panics_doc)]
pub fn phoenix_stake<R: RngCore + CryptoRng, P: Prove>(
    rng: &mut R,
    phoenix_sender_sk: &PhoenixSecretKey,
    stake_sk: &BlsSecretKey,
    inputs: Vec<(Note, Opening<(), NOTES_TREE_DEPTH>)>,
    root: BlsScalar,
    gas_limit: u64,
    gas_price: u64,
    chain_id: u8,
    stake_value: u64,
    current_nonce: u64,
) -> Result<Transaction, Error> {
    let receiver_pk = PhoenixPublicKey::from(phoenix_sender_sk);
    let change_pk = receiver_pk;

    let transfer_value = 0;
    let obfuscated_transaction = false;
    let deposit = stake_value;

    let stake = Stake::new(stake_sk, stake_value, current_nonce + 1, chain_id);

    let contract_call = ContractCall::new(STAKE_CONTRACT, "stake", &stake)?;

    phoenix::<R, P>(
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
        chain_id,
        Some(contract_call),
    )
}

/// Create an unproven [`Transaction`] to withdraw stake rewards into a
/// phoenix-note.
///
/// # Errors
/// The creation of a transaction is not possible and will error if:
/// - one of the input-notes doesn't belong to the `phoenix_sender_sk`
/// - the transaction input doesn't cover the transaction costs
/// - the `inputs` vector is either empty or larger than 4 elements
/// - the `inputs` vector contains duplicate `Note`s
/// - the `Prove` trait is implemented incorrectly
#[allow(clippy::too_many_arguments)]
#[allow(clippy::missing_panics_doc)]
pub fn phoenix_stake_reward<R: RngCore + CryptoRng, P: Prove>(
    rng: &mut R,
    phoenix_sender_sk: &PhoenixSecretKey,
    stake_sk: &BlsSecretKey,
    inputs: Vec<(Note, Opening<(), NOTES_TREE_DEPTH>, BlsScalar)>,
    root: BlsScalar,
    reward_amount: u64,
    gas_limit: u64,
    gas_price: u64,
    chain_id: u8,
) -> Result<Transaction, Error> {
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

    let contract_call = stake_reward_to_phoenix(
        rng,
        phoenix_sender_sk,
        stake_sk,
        gas_payment_token,
        reward_amount,
    )?;

    phoenix::<R, P>(
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
        chain_id,
        Some(contract_call),
    )
}

/// Create an unproven [`Transaction`] to unstake into a phoenix-note.
///
/// # Errors
/// The creation of a transaction is not possible and will error if:
/// - one of the input-notes doesn't belong to the `sender_sk`
/// - the transaction input doesn't cover the transaction costs
/// - the `inputs` vector is either empty or larger than 4 elements
/// - the `inputs` vector contains duplicate `Note`s
/// - the `Prove` trait is implemented incorrectly
#[allow(clippy::too_many_arguments)]
#[allow(clippy::missing_panics_doc)]
pub fn phoenix_unstake<R: RngCore + CryptoRng, P: Prove>(
    rng: &mut R,
    phoenix_sender_sk: &PhoenixSecretKey,
    stake_sk: &BlsSecretKey,
    inputs: Vec<(Note, Opening<(), NOTES_TREE_DEPTH>, BlsScalar)>,
    root: BlsScalar,
    unstake_value: u64,
    gas_limit: u64,
    gas_price: u64,
    chain_id: u8,
) -> Result<Transaction, Error> {
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

    let contract_call = unstake_to_phoenix(
        rng,
        phoenix_sender_sk,
        stake_sk,
        gas_payment_token,
        unstake_value,
    )?;

    phoenix::<R, P>(
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
        chain_id,
        Some(contract_call),
    )
}

fn stake_reward_to_phoenix<R: RngCore + CryptoRng>(
    rng: &mut R,
    phoenix_sender_sk: &PhoenixSecretKey,
    stake_sk: &BlsSecretKey,
    gas_payment_token: WithdrawReplayToken,
    reward_amount: u64,
) -> Result<ContractCall, Error> {
    let withdraw = withdraw_to_phoenix(
        rng,
        phoenix_sender_sk,
        STAKE_CONTRACT,
        gas_payment_token,
        reward_amount,
    );

    let reward_withdraw = StakeWithdraw::new(stake_sk, withdraw);

    ContractCall::new(STAKE_CONTRACT, "withdraw", &reward_withdraw)
}

fn unstake_to_phoenix<R: RngCore + CryptoRng>(
    rng: &mut R,
    phoenix_sender_sk: &PhoenixSecretKey,
    stake_sk: &BlsSecretKey,
    gas_payment_token: WithdrawReplayToken,
    unstake_value: u64,
) -> Result<ContractCall, Error> {
    let withdraw = withdraw_to_phoenix(
        rng,
        phoenix_sender_sk,
        STAKE_CONTRACT,
        gas_payment_token,
        unstake_value,
    );

    let unstake = StakeWithdraw::new(stake_sk, withdraw);

    ContractCall::new(STAKE_CONTRACT, "unstake", &unstake)
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
