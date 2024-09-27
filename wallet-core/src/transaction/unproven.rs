// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Implementations of basic wallet functionalities to create unproven phoenix
//! transactions.

use alloc::vec::Vec;

use dusk_bytes::Serializable;
use rand::{CryptoRng, RngCore};

use execution_core::{
    signatures::bls::{PublicKey as BlsPublicKey, SecretKey as BlsSecretKey},
    stake::{Stake, STAKE_CONTRACT},
    transfer::{
        data::{
            ContractBytecode, ContractCall, ContractDeploy, TransactionData,
        },
        phoenix::{
            Note, NoteOpening, PublicKey as PhoenixPublicKey,
            SecretKey as PhoenixSecretKey, UnprovenTransaction,
        },
        withdraw::WithdrawReplayToken,
    },
    BlsScalar, Error,
};

/// Create a generic [`UnprovenTransaction`] to be proven at a later
/// point.
///
/// # Errors
/// The creation of a transaction is not possible and will error if:
/// - one of the input-notes doesn't belong to the `phoenix_sender_sk`
/// - the transaction input doesn't cover the transaction costs
/// - the `inputs` vector is either empty or larger than 4 elements
/// - the `inputs` vector contains duplicate `Note`s
/// - the Memo provided with `data` is too large
#[allow(clippy::too_many_arguments)]
pub fn phoenix<R: RngCore + CryptoRng>(
    rng: &mut R,
    sender_sk: &PhoenixSecretKey,
    change_pk: &PhoenixPublicKey,
    receiver_pk: &PhoenixPublicKey,
    inputs: Vec<(Note, NoteOpening)>,
    root: BlsScalar,
    transfer_value: u64,
    obfuscated_transaction: bool,
    deposit: u64,
    gas_limit: u64,
    gas_price: u64,
    chain_id: u8,
    data: Option<impl Into<TransactionData>>,
) -> Result<UnprovenTransaction, Error> {
    UnprovenTransaction::new(
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
        data,
    )
}

/// Create an [`UnprovenTransaction`] to stake from phoenix-notes.
///
/// # Note
/// The `current_nonce` is NOT incremented and should be incremented
/// by the caller of this function, if its not done so, rusk
/// will throw 500 error
///
/// # Errors
/// The creation of a transaction is not possible and will error if:
/// - one of the input-notes doesn't belong to the `phoenix_sender_sk`
/// - the transaction input doesn't cover the transaction costs
/// - the `inputs` vector is either empty or larger than 4 elements
/// - the `inputs` vector contains duplicate `Note`s
#[allow(clippy::too_many_arguments)]
pub fn phoenix_stake<R: RngCore + CryptoRng>(
    rng: &mut R,
    phoenix_sender_sk: &PhoenixSecretKey,
    stake_sk: &BlsSecretKey,
    inputs: Vec<(Note, NoteOpening)>,
    root: BlsScalar,
    gas_limit: u64,
    gas_price: u64,
    chain_id: u8,
    stake_value: u64,
    current_nonce: u64,
) -> Result<UnprovenTransaction, Error> {
    let receiver_pk = PhoenixPublicKey::from(phoenix_sender_sk);
    let change_pk = receiver_pk;

    let transfer_value = 0;
    let obfuscated_transaction = false;
    let deposit = stake_value;

    let stake = Stake::new(stake_sk, stake_value, current_nonce, chain_id);

    let contract_call = ContractCall::new(STAKE_CONTRACT, "stake", &stake)?;

    phoenix(
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

/// Create an [`UnprovenTransaction`] to withdraw stake rewards into a
/// phoenix-note.
///
/// # Errors
/// The creation of a transaction is not possible and will error if:
/// - one of the input-notes doesn't belong to the `phoenix_sender_sk`
/// - the transaction input doesn't cover the transaction costs
/// - the `inputs` vector is either empty or larger than 4 elements
/// - the `inputs` vector contains duplicate `Note`s
#[allow(clippy::too_many_arguments)]
pub fn phoenix_stake_reward<R: RngCore + CryptoRng>(
    rng: &mut R,
    phoenix_sender_sk: &PhoenixSecretKey,
    stake_sk: &BlsSecretKey,
    inputs: Vec<(Note, NoteOpening, BlsScalar)>,
    root: BlsScalar,
    reward_amount: u64,
    gas_limit: u64,
    gas_price: u64,
    chain_id: u8,
) -> Result<UnprovenTransaction, Error> {
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

    let contract_call = super::stake_reward_to_phoenix(
        rng,
        phoenix_sender_sk,
        stake_sk,
        gas_payment_token,
        reward_amount,
    )?;

    phoenix(
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

/// Create an [`UnprovenTransaction`] to unstake into a phoenix-note.
///
/// # Errors
/// The creation of a transaction is not possible and will error if:
/// - one of the input-notes doesn't belong to the `sender_sk`
/// - the transaction input doesn't cover the transaction costs
/// - the `inputs` vector is either empty or larger than 4 elements
/// - the `inputs` vector contains duplicate `Note`s
#[allow(clippy::too_many_arguments)]
pub fn phoenix_unstake<R: RngCore + CryptoRng>(
    rng: &mut R,
    phoenix_sender_sk: &PhoenixSecretKey,
    stake_sk: &BlsSecretKey,
    inputs: Vec<(Note, NoteOpening, BlsScalar)>,
    root: BlsScalar,
    unstake_value: u64,
    gas_limit: u64,
    gas_price: u64,
    chain_id: u8,
) -> Result<UnprovenTransaction, Error> {
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

    let contract_call = super::unstake_to_phoenix(
        rng,
        phoenix_sender_sk,
        stake_sk,
        gas_payment_token,
        unstake_value,
    )?;

    phoenix(
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

/// Create an [`UnprovenTransaction`] to convert Phoenix Dusk into Moonlight
/// Dusk.
///
/// # Note
/// The ownership of both sender and receiver keys is required, and
/// enforced by the protocol.
///
/// # Errors
/// The creation of a transaction is not possible and will error if:
/// - one of the input-notes doesn't belong to the `sender_sk`
/// - the transaction input doesn't cover the transaction costs
/// - the `inputs` vector is either empty or larger than 4 elements
/// - the `inputs` vector contains duplicate `Note`s
#[allow(clippy::too_many_arguments)]
pub fn phoenix_to_moonlight<R: RngCore + CryptoRng>(
    rng: &mut R,
    phoenix_sender_sk: &PhoenixSecretKey,
    moonlight_receiver_sk: &BlsSecretKey,
    inputs: Vec<(Note, NoteOpening, BlsScalar)>,
    root: BlsScalar,
    convert_value: u64,
    gas_limit: u64,
    gas_price: u64,
    chain_id: u8,
) -> Result<UnprovenTransaction, Error> {
    let receiver_pk = PhoenixPublicKey::from(phoenix_sender_sk);
    let change_pk = receiver_pk;

    let transfer_value = 0;
    let obfuscated_transaction = true;
    let deposit = convert_value; // a convertion is a simultaneous deposit to *and* withdrawal from the
                                 // transfer contract

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

    let contract_call = super::convert_to_moonlight(
        rng,
        moonlight_receiver_sk,
        gas_payment_token,
        convert_value,
    )?;

    phoenix(
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

/// Create a new [`UnprovenTransaction`] to deploy a contract to the network.
///
/// # Errors
/// The creation of a transaction is not possible and will error if:
/// - one of the input-notes doesn't belong to the `sender_sk`
/// - the transaction input doesn't cover the transaction costs
/// - the `inputs` vector is either empty or larger than 4 elements
/// - the `inputs` vector contains duplicate `Note`s
/// - the `Prove` trait is implemented incorrectly
#[allow(clippy::too_many_arguments)]
pub fn phoenix_deployment<R: RngCore + CryptoRng>(
    rng: &mut R,
    phoenix_sender_sk: &PhoenixSecretKey,
    inputs: Vec<(Note, NoteOpening, BlsScalar)>,
    root: BlsScalar,
    bytecode: impl Into<Vec<u8>>,
    owner: &BlsPublicKey,
    init_args: Vec<u8>,
    nonce: u64,
    gas_limit: u64,
    gas_price: u64,
    chain_id: u8,
) -> Result<UnprovenTransaction, Error> {
    let receiver_pk = PhoenixPublicKey::from(phoenix_sender_sk);
    let change_pk = receiver_pk;

    let transfer_value = 0;
    let obfuscated_transaction = true;
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

    let bytes = bytecode.into();
    let deploy = ContractDeploy {
        bytecode: ContractBytecode {
            hash: blake3::hash(&bytes).into(),
            bytes,
        },
        owner: owner.to_bytes().to_vec(),
        init_args: Some(init_args),
        nonce,
    };

    phoenix(
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
        Some(deploy),
    )
}
