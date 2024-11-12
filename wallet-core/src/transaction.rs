// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Implementations of basic wallet functionalities to create transactions.

use alloc::vec::Vec;

use dusk_bytes::Serializable;
use ff::Field;
use rand::{CryptoRng, RngCore};
use zeroize::Zeroize;

use execution_core::{
    signatures::bls::{PublicKey as BlsPublicKey, SecretKey as BlsSecretKey},
    stake::{Stake, Withdraw as StakeWithdraw, STAKE_CONTRACT},
    transfer::{
        data::{
            ContractBytecode, ContractCall, ContractDeploy, TransactionData,
        },
        moonlight::Transaction as MoonlightTransaction,
        phoenix::{
            Note, NoteOpening, Prove, PublicKey as PhoenixPublicKey,
            SecretKey as PhoenixSecretKey, Transaction as PhoenixTransaction,
        },
        withdraw::{Withdraw, WithdrawReceiver, WithdrawReplayToken},
        Transaction, TRANSFER_CONTRACT,
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
/// - the Memo provided with `data` is too large
#[allow(clippy::too_many_arguments)]
pub fn phoenix<R: RngCore + CryptoRng, P: Prove>(
    rng: &mut R,
    sender_sk: &PhoenixSecretKey,
    refund_pk: &PhoenixPublicKey,
    receiver_pk: &PhoenixPublicKey,
    inputs: Vec<(Note, NoteOpening)>,
    root: BlsScalar,
    transfer_value: u64,
    is_transfer: bool,
    deposit: u64,
    gas_limit: u64,
    gas_price: u64,
    chain_id: u8,
    data: Option<impl Into<TransactionData>>,
    prover: &P,
) -> Result<Transaction, Error> {
    // always create a transparent transfer-note (the change-note will always be
    // obfuscated) if the transfer-value is 0 (the transfer-contract doesn't
    // append transparent notes with value 0 to the tree)
    let mut obfuscate_transfer_note = is_transfer;
    if transfer_value == 0 {
        obfuscate_transfer_note = false;
    }

    Ok(PhoenixTransaction::new::<R, P>(
        rng,
        sender_sk,
        refund_pk,
        receiver_pk,
        inputs,
        root,
        transfer_value,
        obfuscate_transfer_note,
        deposit,
        gas_limit,
        gas_price,
        chain_id,
        data,
        prover,
    )?
    .into())
}

/// Creates a totally generic Moonlight [`Transaction`], all fields being
/// variable.
///
/// # Note
/// The `moonlight_nonce` is NOT incremented and should be incremented
/// by the caller of this function, if its not done so, rusk
/// will throw 500 error
///
/// # Errors
/// The creation of a transaction is not possible and will error if:
/// - the Memo provided with `data` is too large
#[allow(clippy::too_many_arguments)]
pub fn moonlight(
    sender_sk: &BlsSecretKey,
    receiver_pk: Option<BlsPublicKey>,
    transfer_value: u64,
    deposit: u64,
    gas_limit: u64,
    gas_price: u64,
    moonlight_nonce: u64,
    chain_id: u8,
    data: Option<impl Into<TransactionData>>,
) -> Result<Transaction, Error> {
    Ok(MoonlightTransaction::new(
        sender_sk,
        receiver_pk,
        transfer_value,
        deposit,
        gas_limit,
        gas_price,
        moonlight_nonce,
        chain_id,
        data,
    )?
    .into())
}

/// Create a [`Transaction`] to stake from phoenix-notes.
///
/// # Note
/// The `stake_nonce` is NOT incremented and should be incremented
/// by the caller of this function, if its not done so, rusk
/// will throw 500 error
///
/// # Errors
/// The creation of a transaction is not possible and will error if:
/// - one of the input-notes doesn't belong to the `phoenix_sender_sk`
/// - the transaction input doesn't cover the transaction costs
/// - the `inputs` vector is either empty or larger than 4 elements
/// - the `inputs` vector contains duplicate `Note`s
/// - the `Prove` trait is implemented incorrectly
#[allow(clippy::too_many_arguments)]
pub fn phoenix_stake<R: RngCore + CryptoRng, P: Prove>(
    rng: &mut R,
    phoenix_sender_sk: &PhoenixSecretKey,
    stake_sk: &BlsSecretKey,
    inputs: Vec<(Note, NoteOpening)>,
    root: BlsScalar,
    gas_limit: u64,
    gas_price: u64,
    chain_id: u8,
    stake_value: u64,
    stake_nonce: u64,
    prover: &P,
) -> Result<Transaction, Error> {
    // in a staking transaction the receiver and refund-address is the sender
    let phoenix_receiver_pk = PhoenixPublicKey::from(phoenix_sender_sk);
    let phoenix_refund_pk = PhoenixPublicKey::from(phoenix_sender_sk);

    let transfer_value = 0;
    let is_transfer = false;
    let deposit = stake_value;

    let stake = Stake::new(stake_sk, stake_value, stake_nonce, chain_id);

    let contract_call = ContractCall::new(STAKE_CONTRACT, "stake", &stake)?;

    phoenix::<R, P>(
        rng,
        phoenix_sender_sk,
        &phoenix_refund_pk,
        &phoenix_receiver_pk,
        inputs,
        root,
        transfer_value,
        is_transfer,
        deposit,
        gas_limit,
        gas_price,
        chain_id,
        Some(contract_call),
        prover,
    )
}

/// Create a [`Transaction`] to stake from a Moonlight account.
///
/// # Note
/// The `moonlight_nonce` and `stake_nonce` are NOT incremented
/// and should be incremented by the caller of this function, if its not done
/// so, rusk will throw 500 error
///
/// # Errors
/// The creation of this transaction doesn't error, but still returns a result
/// for the sake of API consistency.
#[allow(clippy::too_many_arguments)]
pub fn moonlight_stake(
    moonlight_sender_sk: &BlsSecretKey,
    stake_sk: &BlsSecretKey,
    stake_value: u64,
    gas_limit: u64,
    gas_price: u64,
    moonlight_nonce: u64,
    stake_nonce: u64,
    chain_id: u8,
) -> Result<Transaction, Error> {
    let transfer_value = 0;
    let deposit = stake_value;

    let stake = Stake::new(stake_sk, stake_value, stake_nonce, chain_id);

    let contract_call = ContractCall::new(STAKE_CONTRACT, "stake", &stake)?;

    moonlight(
        moonlight_sender_sk,
        None,
        transfer_value,
        deposit,
        gas_limit,
        gas_price,
        moonlight_nonce,
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
pub fn phoenix_stake_reward<R: RngCore + CryptoRng, P: Prove>(
    rng: &mut R,
    phoenix_sender_sk: &PhoenixSecretKey,
    stake_sk: &BlsSecretKey,
    inputs: Vec<(Note, NoteOpening, BlsScalar)>,
    root: BlsScalar,
    reward_amount: u64,
    gas_limit: u64,
    gas_price: u64,
    chain_id: u8,
    prover: &P,
) -> Result<Transaction, Error> {
    // in a staking transaction the receiver and refund-address is the sender
    let phoenix_receiver_pk = PhoenixPublicKey::from(phoenix_sender_sk);
    let phoenix_refund_pk = PhoenixPublicKey::from(phoenix_sender_sk);

    let transfer_value = 0;
    let is_transfer = false;
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
        &phoenix_refund_pk,
        &phoenix_receiver_pk,
        inputs,
        root,
        transfer_value,
        is_transfer,
        deposit,
        gas_limit,
        gas_price,
        chain_id,
        Some(contract_call),
        prover,
    )
}

/// Create a [`Transaction`] to withdraw stake rewards into Moonlight account.
///
/// # Note
/// The `moonlight_nonce` is NOT incremented and should be incremented by the
/// caller of this function, if its not done so, rusk will throw 500 error
///
/// # Errors
/// The creation of this transaction doesn't error, but still returns a result
/// for the sake of API consistency.
#[allow(clippy::too_many_arguments)]
pub fn moonlight_stake_reward<R: RngCore + CryptoRng>(
    rng: &mut R,
    moonlight_sender_sk: &BlsSecretKey,
    stake_sk: &BlsSecretKey,
    reward_amount: u64,
    gas_limit: u64,
    gas_price: u64,
    moonlight_nonce: u64,
    chain_id: u8,
) -> Result<Transaction, Error> {
    let transfer_value = 0;
    let deposit = 0;

    let gas_payment_token = WithdrawReplayToken::Moonlight(moonlight_nonce);

    let contract_call = stake_reward_to_moonlight(
        rng,
        moonlight_sender_sk,
        stake_sk,
        gas_payment_token,
        reward_amount,
    )?;

    moonlight(
        moonlight_sender_sk,
        None,
        transfer_value,
        deposit,
        gas_limit,
        gas_price,
        moonlight_nonce,
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
pub fn phoenix_unstake<R: RngCore + CryptoRng, P: Prove>(
    rng: &mut R,
    phoenix_sender_sk: &PhoenixSecretKey,
    stake_sk: &BlsSecretKey,
    inputs: Vec<(Note, NoteOpening, BlsScalar)>,
    root: BlsScalar,
    unstake_value: u64,
    gas_limit: u64,
    gas_price: u64,
    chain_id: u8,
    prover: &P,
) -> Result<Transaction, Error> {
    // in a staking transaction the receiver and refund-address is the sender
    let phoenix_receiver_pk = PhoenixPublicKey::from(phoenix_sender_sk);
    let phoenix_refund_pk = PhoenixPublicKey::from(phoenix_sender_sk);

    let transfer_value = 0;
    let is_transfer = false;
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
        &phoenix_refund_pk,
        &phoenix_receiver_pk,
        inputs,
        root,
        transfer_value,
        is_transfer,
        deposit,
        gas_limit,
        gas_price,
        chain_id,
        Some(contract_call),
        prover,
    )
}

/// Create a [`Transaction`] to unstake into a Moonlight account.
///
/// # Note
/// The `moonlight_nonce` is NOT incremented and should be incremented by the
/// caller of this function, if its not done so, rusk will throw 500 error
///
/// # Errors
/// The creation of a transaction is not possible and will error if:
/// - the Memo provided with `data` is too large
#[allow(clippy::too_many_arguments)]
pub fn moonlight_unstake<R: RngCore + CryptoRng>(
    rng: &mut R,
    moonlight_sender_sk: &BlsSecretKey,
    stake_sk: &BlsSecretKey,
    unstake_value: u64,
    gas_limit: u64,
    gas_price: u64,
    moonlight_nonce: u64,
    chain_id: u8,
) -> Result<Transaction, Error> {
    let transfer_value = 0;
    let deposit = 0;

    let gas_payment_token = WithdrawReplayToken::Moonlight(moonlight_nonce);

    let contract_call = unstake_to_moonlight(
        rng,
        moonlight_sender_sk,
        stake_sk,
        gas_payment_token,
        unstake_value,
    )?;

    moonlight(
        moonlight_sender_sk,
        None,
        transfer_value,
        deposit,
        gas_limit,
        gas_price,
        moonlight_nonce,
        chain_id,
        Some(contract_call),
    )
}

/// Create an unproven [`Transaction`] to convert Phoenix Dusk into Moonlight
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
/// - the `Prove` trait is implemented incorrectly
#[allow(clippy::too_many_arguments)]
pub fn phoenix_to_moonlight<R: RngCore + CryptoRng, P: Prove>(
    rng: &mut R,
    phoenix_sender_sk: &PhoenixSecretKey,
    moonlight_receiver_sk: &BlsSecretKey,
    inputs: Vec<(Note, NoteOpening, BlsScalar)>,
    root: BlsScalar,
    convert_value: u64,
    gas_limit: u64,
    gas_price: u64,
    chain_id: u8,
    prover: &P,
) -> Result<Transaction, Error> {
    // because a conversion is done through a contract call, the receiver and
    // refund-address of the transaction is the sender
    let phoenix_receiver_pk = PhoenixPublicKey::from(phoenix_sender_sk);
    let phoenix_refund_pk = PhoenixPublicKey::from(phoenix_sender_sk);

    let transfer_value = 0;
    let is_transfer = false;
    // a conversion is a simultaneous deposit to *and* withdrawal from the
    // transfer contract
    let deposit = convert_value;

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

    let contract_call = convert_to_moonlight(
        rng,
        moonlight_receiver_sk,
        gas_payment_token,
        convert_value,
    )?;

    phoenix::<R, P>(
        rng,
        phoenix_sender_sk,
        &phoenix_refund_pk,
        &phoenix_receiver_pk,
        inputs,
        root,
        transfer_value,
        is_transfer,
        deposit,
        gas_limit,
        gas_price,
        chain_id,
        Some(contract_call),
        prover,
    )
}

/// Create a [`Transaction`] to convert Moonlight Dusk into Phoenix Dusk.
///
/// # Note
/// 1. The ownership of both sender and receiver keys is required, and
/// enforced by the protocol.
/// 2. `moonlight_nonce` is NOT incremented and should be incremented by the
///    caller
/// of this function, if its not done so, rusk will throw 500 error
///
/// # Errors
/// The creation of this transaction doesn't error, but still returns a result
/// for the sake of API consistency.
#[allow(clippy::too_many_arguments)]
pub fn moonlight_to_phoenix<R: RngCore + CryptoRng>(
    rng: &mut R,
    moonlight_sender_sk: &BlsSecretKey,
    phoenix_receiver_sk: &PhoenixSecretKey,
    convert_value: u64,
    gas_limit: u64,
    gas_price: u64,
    moonlight_nonce: u64,
    chain_id: u8,
) -> Result<Transaction, Error> {
    // a conversion is a simultaneous deposit to *and* withdrawal from the
    // transfer contract
    let deposit = convert_value;
    let transfer_value = 0;
    let gas_payment_token = WithdrawReplayToken::Moonlight(moonlight_nonce);

    let contract_call = convert_to_phoenix(
        rng,
        phoenix_receiver_sk,
        gas_payment_token,
        convert_value,
    )?;

    moonlight(
        moonlight_sender_sk,
        None,
        transfer_value,
        deposit,
        gas_limit,
        gas_price,
        moonlight_nonce,
        chain_id,
        Some(contract_call),
    )
}

/// Create a new unproven [`Transaction`] to deploy a contract to the network.
///
/// # Errors
/// The creation of a transaction is not possible and will error if:
/// - one of the input-notes doesn't belong to the `sender_sk`
/// - the transaction input doesn't cover the transaction costs
/// - the `inputs` vector is either empty or larger than 4 elements
/// - the `inputs` vector contains duplicate `Note`s
/// - the `Prove` trait is implemented incorrectly
#[allow(clippy::too_many_arguments)]
pub fn phoenix_deployment<R: RngCore + CryptoRng, P: Prove>(
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
    prover: &P,
) -> Result<Transaction, Error> {
    // in a contract deployment transaction the receiver and refund-address is
    // the sender
    let phoenix_receiver_pk = PhoenixPublicKey::from(phoenix_sender_sk);
    let phoenix_refund_pk = PhoenixPublicKey::from(phoenix_sender_sk);

    let transfer_value = 0;
    let is_transfer = false;
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
        &phoenix_refund_pk,
        &phoenix_receiver_pk,
        inputs,
        root,
        transfer_value,
        is_transfer,
        deposit,
        gas_limit,
        gas_price,
        chain_id,
        Some(deploy),
        prover,
    )
}

/// Create a new [`Transaction`] to deploy a contract to the network.
///
/// # Note
/// The `moonlight_nonce` is NOT incremented and should be incremented by the
/// caller of this function, if its not done so, rusk will throw 500 error
///
/// # Errors
/// The creation of this transaction doesn't error, but still returns a result
/// for the sake of API consistency.
#[allow(clippy::too_many_arguments)]
pub fn moonlight_deployment(
    moonlight_sender_sk: &BlsSecretKey,
    bytecode: impl Into<Vec<u8>>,
    owner: &BlsPublicKey,
    init_args: Vec<u8>,
    gas_limit: u64,
    gas_price: u64,
    moonlight_nonce: u64,
    deploy_nonce: u64,
    chain_id: u8,
) -> Result<Transaction, Error> {
    let transfer_value = 0;
    let deposit = 0;

    let bytes = bytecode.into();
    let deploy = ContractDeploy {
        bytecode: ContractBytecode {
            hash: blake3::hash(&bytes).into(),
            bytes,
        },
        owner: owner.to_bytes().to_vec(),
        init_args: Some(init_args),
        nonce: deploy_nonce,
    };

    moonlight(
        moonlight_sender_sk,
        None,
        transfer_value,
        deposit,
        gas_limit,
        gas_price,
        moonlight_nonce,
        chain_id,
        Some(deploy),
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

pub(crate) fn stake_reward_to_moonlight<R: RngCore + CryptoRng>(
    rng: &mut R,
    moonlight_receiver_sk: &BlsSecretKey,
    stake_sk: &BlsSecretKey,
    gas_payment_token: WithdrawReplayToken,
    reward_amount: u64,
) -> Result<ContractCall, Error> {
    let withdraw = withdraw_to_moonlight(
        rng,
        moonlight_receiver_sk,
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

pub(crate) fn unstake_to_moonlight<R: RngCore + CryptoRng>(
    rng: &mut R,
    moonlight_receiver_sk: &BlsSecretKey,
    stake_sk: &BlsSecretKey,
    gas_payment_token: WithdrawReplayToken,
    unstake_value: u64,
) -> Result<ContractCall, Error> {
    let withdraw = withdraw_to_moonlight(
        rng,
        moonlight_receiver_sk,
        STAKE_CONTRACT,
        gas_payment_token,
        unstake_value,
    );

    let unstake = StakeWithdraw::new(stake_sk, withdraw);

    ContractCall::new(STAKE_CONTRACT, "unstake", &unstake)
}

fn convert_to_moonlight<R: RngCore + CryptoRng>(
    rng: &mut R,
    moonlight_receiver_sk: &BlsSecretKey,
    gas_payment_token: WithdrawReplayToken,
    convert_value: u64,
) -> Result<ContractCall, Error> {
    ContractCall::new(
        TRANSFER_CONTRACT,
        "convert",
        &withdraw_to_moonlight(
            rng,
            moonlight_receiver_sk,
            TRANSFER_CONTRACT,
            gas_payment_token,
            convert_value,
        ),
    )
}

fn convert_to_phoenix<R: RngCore + CryptoRng>(
    rng: &mut R,
    phoenix_receiver_sk: &PhoenixSecretKey,
    gas_payment_token: WithdrawReplayToken,
    convert_value: u64,
) -> Result<ContractCall, Error> {
    ContractCall::new(
        TRANSFER_CONTRACT,
        "convert",
        &withdraw_to_phoenix(
            rng,
            phoenix_receiver_sk,
            TRANSFER_CONTRACT,
            gas_payment_token,
            convert_value,
        ),
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

/// Create a [`Withdraw`] struct to be used to withdraw funds from a contract
/// into a Moonlight account.
///
/// The gas payment can be done by either Phoenix or Moonlight by setting the
/// `gas_payment_token` accordingly.
fn withdraw_to_moonlight<R: RngCore + CryptoRng>(
    rng: &mut R,
    receiver_sk: &BlsSecretKey,
    contract: impl Into<ContractId>,
    gas_payment_token: WithdrawReplayToken,
    value: u64,
) -> Withdraw {
    Withdraw::new(
        rng,
        receiver_sk,
        contract.into(),
        value,
        WithdrawReceiver::Moonlight(BlsPublicKey::from(receiver_sk)),
        gas_payment_token,
    )
}
