// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Implementations of basic wallet functionalities to create moonlight
//! transactions.

use execution_core::{
    signatures::bls::{PublicKey as BlsPublicKey, SecretKey as BlsSecretKey},
    stake::{Stake, Withdraw as StakeWithdraw, STAKE_CONTRACT},
    transfer::{
        data::{ContractCall, TransactionData},
        moonlight::Transaction as MoonlightTransaction,
        withdraw::{Withdraw, WithdrawReceiver, WithdrawReplayToken},
        Transaction,
    },
    Error,
};

use rand::{CryptoRng, RngCore};

/// Generate a moonlight transaction
///
/// # Errors
/// - the transaction-data is incorrect
#[allow(clippy::too_many_arguments)]
pub fn moonlight(
    from_sk: &BlsSecretKey,
    to_account: Option<BlsPublicKey>,
    value: u64,
    deposit: u64,
    gas_limit: u64,
    gas_price: u64,
    nonce: u64,
    chain_id: u8,
    data: Option<impl Into<TransactionData>>,
) -> Result<Transaction, Error> {
    Ok(MoonlightTransaction::new(
        from_sk,
        to_account,
        value,
        deposit,
        gas_limit,
        gas_price,
        nonce + 1,
        chain_id,
        data,
    )?
    .into())
}

/// Stake through moonlight, the stake_nonce is the nonce of the stake
/// which is obtained via stake info query on the chain
///
/// The `nonce` is the nonce of the moonlight transaction
pub fn moonlight_stake(
    from_sk: &BlsSecretKey,
    stake_value: u64,
    chain_id: u8,
    stake_nonce: u64,
    nonce: u64,
    gas_limit: u64,
    gas_price: u64,
) -> Result<Transaction, Error> {
    let receiver_pk = BlsPublicKey::from(from_sk);

    let transfer_value = 0;
    let deposit = stake_value;

    let stake = Stake::new(from_sk, stake_value, stake_nonce + 1, chain_id);

    let contract_call = ContractCall::new(STAKE_CONTRACT, "stake", &stake)?;

    Ok(MoonlightTransaction::new(
        from_sk,
        Some(receiver_pk),
        transfer_value,
        deposit,
        gas_limit,
        gas_price,
        nonce + 1,
        chain_id,
        Some(contract_call),
    )?
    .into())
}

/// Unstake through moonlight
pub fn moonlight_unstake<R: RngCore + CryptoRng>(
    rng: &mut R,
    from_sk: &BlsSecretKey,
    unstake_value: u64,
    chain_id: u8,
    nonce: u64,
    gas_limit: u64,
    gas_price: u64,
) -> Result<Transaction, Error> {
    let receiver_pk = BlsPublicKey::from(from_sk);

    let transfer_value = 0;
    let deposit = unstake_value;

    let withdraw = Withdraw::new(
        rng,
        from_sk,
        STAKE_CONTRACT,
        unstake_value,
        WithdrawReceiver::Moonlight(receiver_pk),
        WithdrawReplayToken::Moonlight(nonce),
    );

    let unstake = StakeWithdraw::new(from_sk, withdraw);

    let contract_call = ContractCall::new(STAKE_CONTRACT, "unstake", &unstake)?;

    Ok(MoonlightTransaction::new(
        from_sk,
        Some(receiver_pk),
        transfer_value,
        deposit,
        gas_limit,
        gas_price,
        nonce + 1,
        chain_id,
        Some(contract_call),
    )?
    .into())
}
