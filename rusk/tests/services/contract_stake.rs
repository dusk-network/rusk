// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::Path;
use std::sync::{Arc, RwLock};

use dusk_core::stake::{self, Stake, DEFAULT_MINIMUM_STAKE, EPOCH};

use dusk_bytes::Serializable;
use dusk_core::abi::ContractId;
use dusk_core::transfer::data::ContractCall;
use dusk_core::transfer::{self, Transaction};
use dusk_vm::gen_contract_id;
use node_data::ledger::SpentTransaction;
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::{Result, Rusk};
use std::collections::HashMap;
use tempfile::tempdir;
use test_wallet::{self as wallet};
use tracing::info;

use crate::common::state::{generator_procedure2, new_state};
use crate::common::wallet::{TestStateClient, TestStore};
use crate::common::*;

const BLOCK_HEIGHT: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;
const GAS_LIMIT: u64 = 10_000_000_000;
const GAS_PRICE: u64 = 1;

// Creates the Rusk initial state for the tests below
fn stake_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot =
        toml::from_str(include_str!("../config/stake_from_contract.toml"))
            .expect("Cannot deserialize config");

    new_state(dir, &snapshot, u64::MAX)
}

#[tokio::test(flavor = "multi_thread")]
pub async fn stake_from_contract() -> Result<()> {
    // Setup the logger
    logger();

    let mut rng = StdRng::seed_from_u64(0xdead);

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = stake_state(&tmp)?;

    let cache = Arc::new(RwLock::new(HashMap::new()));

    // Create a wallet
    let wallet = wallet::Wallet::new(
        TestStore,
        TestStateClient {
            rusk: rusk.clone(),
            cache,
        },
    );

    let contract_id = deploy_proxy_contract(&rusk, &wallet);

    info!("Contract ID: {:?}", contract_id);

    let pk = wallet.account_public_key(0).unwrap();
    let sk = wallet.account_secret_key(0).unwrap();

    let stake = Stake::new_from_contract(
        &sk,
        contract_id,
        DEFAULT_MINIMUM_STAKE,
        rusk.chain_id().unwrap(),
    );
    let call = ContractCall::new(contract_id, "stake", &stake)
        .expect("call to be successful");
    let stake_from_contract = wallet
        .moonlight_execute(
            0,
            0,
            DEFAULT_MINIMUM_STAKE,
            GAS_LIMIT,
            GAS_PRICE,
            Some(call.clone()),
        )
        .expect("stake to be successful");
    let tx = execute_transaction(stake_from_contract, &rusk, 1, None, None);
    info!("Stake from contract - gas spent {:?}", tx.gas_spent);

    let stake = wallet.get_stake(0).expect("stake to be found");
    assert_eq!(
        DEFAULT_MINIMUM_STAKE,
        stake.amount.expect("stake amount to be found").value
    );

    let stake_from_contract = wallet
        .moonlight_execute(
            0,
            0,
            DEFAULT_MINIMUM_STAKE,
            GAS_LIMIT,
            GAS_PRICE,
            Some(call.clone()),
        )
        .expect("stake to be successful");
    let tx = execute_transaction(stake_from_contract, &rusk, 2, None, None);
    info!("Stake from contract - gas spent {:?}", tx.gas_spent);

    let stake = wallet.get_stake(0).expect("stake to be found");
    assert_eq!(
        DEFAULT_MINIMUM_STAKE * 2,
        stake.amount.expect("stake amount to be found").value
    );

    let stake_from_contract = wallet
        .moonlight_execute(
            0,
            0,
            DEFAULT_MINIMUM_STAKE,
            GAS_LIMIT,
            GAS_PRICE,
            Some(call),
        )
        .expect("stake to be successful");
    let tx = execute_transaction(
        stake_from_contract,
        &rusk,
        EPOCH * 2,
        None,
        Some(pk),
    );
    info!("Stake from contract - gas spent {:?}", tx.gas_spent);

    let stake = wallet.get_stake(0).expect("stake to be found");
    assert_eq!(
        DEFAULT_MINIMUM_STAKE * 2 + (DEFAULT_MINIMUM_STAKE / 10 * 9) as u64,
        stake.amount.expect("stake amount to be found").value
    );

    assert_eq!(
        (DEFAULT_MINIMUM_STAKE / 10) as u64,
        stake.amount.expect("stake amount to be found").locked
    );

    let stake_from_account = wallet
        .moonlight_stake(0, 0, 1000, GAS_LIMIT, 1)
        .expect("stake to be successful");
    execute_transaction(
        stake_from_account,
        &rusk,
        1,
        "Panic: Keys mismatch",
        None,
    );

    let unstake = wallet
        .moonlight_unstake(&mut rng, 0, 0, GAS_LIMIT, 1)
        .expect("stake to be successful");

    execute_transaction(
        unstake,
        &rusk,
        1,
        "Panic: expect StakeFundOwner::Account",
        None,
    );

    let unstake = stake::Withdraw::new(
        &sk,
        &sk,
        transfer::withdraw::Withdraw::new(
            &mut rng,
            &sk,
            contract_id,
            3 * DEFAULT_MINIMUM_STAKE,
            transfer::withdraw::WithdrawReceiver::Moonlight(pk),
            transfer::withdraw::WithdrawReplayToken::Moonlight(7),
        ),
    );

    let prev_balance = wallet.get_account(0).unwrap().balance;
    assert_eq!(
        wallet
            .get_stake(0)
            .expect("stake to exists")
            .amount
            .unwrap()
            .total_funds(),
        3 * DEFAULT_MINIMUM_STAKE
    );
    let call = ContractCall::new(contract_id, "unstake", &unstake)
        .expect("call to be successful");
    let unstake_from_contract = wallet
        .moonlight_execute(0, 0, 0, GAS_LIMIT, GAS_PRICE, Some(call.clone()))
        .expect("unstakes to be successful");
    let tx = execute_transaction(unstake_from_contract, &rusk, 1, None, None);
    info!("UnStake from contract - gas spent {:?}", tx.gas_spent);

    assert_eq!(wallet.get_stake(0).expect("stake to exists").amount, None);
    let new_balance = wallet.get_account(0).unwrap().balance;
    let fee_paid = tx.gas_spent * GAS_PRICE;
    assert_eq!(
        new_balance,
        prev_balance + 3 * DEFAULT_MINIMUM_STAKE - fee_paid
    );

    let current_reward = wallet.get_stake(0).expect("Stake to exists").reward;

    let withdraw = stake::Withdraw::new(
        &sk,
        &sk,
        transfer::withdraw::Withdraw::new(
            &mut rng,
            &sk,
            contract_id,
            current_reward - 1,
            transfer::withdraw::WithdrawReceiver::Moonlight(
                wallet.account_public_key(0).unwrap(),
            ),
            transfer::withdraw::WithdrawReplayToken::Moonlight(8),
        ),
    );

    let call = ContractCall::new(contract_id, "withdraw", &withdraw)
        .expect("call to be successful");
    let withdraw_from_contract = wallet
        .moonlight_execute(0, 0, 0, GAS_LIMIT, GAS_PRICE, Some(call.clone()))
        .expect("unstakes to be successful");
    let tx = execute_transaction(withdraw_from_contract, &rusk, 1, None, None);
    info!("Withdraw from contract - gas spent {:?}", tx.gas_spent);

    assert_eq!(wallet.get_stake(0).expect("stake to exists").reward, 1);

    Ok(())
}

fn deploy_proxy_contract(
    rusk: &Rusk,
    wallet: &wallet::Wallet<TestStore, TestStateClient>,
) -> ContractId {
    let deploy_nonce = 0u64;
    let owner = wallet.account_public_key(0).unwrap();
    let charlie_byte_code = include_bytes!(
        "../../../target/wasm32-unknown-unknown/release/charlie.wasm"
    );
    let contract_id =
        gen_contract_id(&charlie_byte_code, deploy_nonce, owner.to_bytes());
    let tx = wallet
        .moonlight_deployment(
            0,
            charlie_byte_code,
            &owner,
            vec![],
            GAS_LIMIT,
            20000,
            0,
        )
        .expect("Failed to create a deploy transaction");

    execute_transaction(tx, rusk, BLOCK_HEIGHT, None, None);
    contract_id
}

fn execute_transaction<'a, E: Into<Option<&'a str>>>(
    tx: Transaction,
    rusk: &Rusk,
    block_height: u64,
    expected_error: E,
    generator: Option<dusk_core::signatures::bls::PublicKey>,
) -> SpentTransaction {
    let (executed_txs, _) = generator_procedure2(
        &rusk,
        &[tx],
        block_height,
        BLOCK_GAS_LIMIT,
        vec![],
        None,
        generator,
    )
    .expect("generator procedure to succeed");
    let tx = executed_txs
        .into_iter()
        .next()
        .expect("Transaction must be executed");

    let tx_error = tx.err.as_ref().map(|e| e.as_str());
    let error = expected_error.into();
    assert_eq!(tx_error, error, "Output error does not match");
    tx
}
