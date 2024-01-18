// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::Path;
use std::sync::{Arc, LazyLock, RwLock};

use dusk_bls12_381_sign::PublicKey;
use dusk_pki::SecretSpendKey;
use dusk_wallet_core::{self as wallet, Store};
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::{Result, Rusk, MINIMUM_STAKE};
use rusk_abi::dusk::dusk;
use rusk_abi::STAKE_CONTRACT;
use std::collections::HashMap;
use tempfile::tempdir;
use tracing::info;

use crate::common::state::{generator_procedure, new_state};
use crate::common::wallet::{TestProverClient, TestStateClient, TestStore};
use crate::common::*;

const BLOCK_HEIGHT: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;
const GAS_LIMIT: u64 = 10_000_000_000;

// Creates the Rusk initial state for the tests below
fn stake_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot = toml::from_str(include_str!("../config/stake.toml"))
        .expect("Cannot deserialize config");

    new_state(dir, &snapshot)
}

// Creates the Rusk initial state for the tests below
fn slash_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot = toml::from_str(include_str!("../config/slash.toml"))
        .expect("Cannot deserialize config");

    new_state(dir, &snapshot)
}

static SSK: LazyLock<SecretSpendKey> = LazyLock::new(|| {
    info!("Generating SecretSpendKey");
    TestStore.retrieve_ssk(0).expect("Should not fail in test")
});

// static SK: LazyLock<SecretKey> = LazyLock::new(|| {
//     info!("Generating BLS SecretKey");
//     TestStore.retrieve_sk(0).expect("Should not fail in test")
// });

/// Stakes an amount Dusk and produces a block with this single transaction,
/// checking the stake is set successfully. It then proceeds to withdraw the
/// stake and checking it is correctly withdrawn.
fn wallet_stake(
    rusk: &Rusk,
    wallet: &wallet::Wallet<TestStore, TestStateClient, TestProverClient>,
    value: u64,
) {
    // Sender psk
    let psk = SSK.public_spend_key();

    let mut rng = StdRng::seed_from_u64(0xdead);

    wallet
        .get_stake(0)
        .expect("stakeinfo to be found")
        .amount
        .expect("stake amount to be found");

    assert!(
        wallet
            .get_stake(2)
            .expect("stakeinfo to be found")
            .amount
            .is_none(),
        "stake amount to be found"
    );

    let tx = wallet
        .stake(&mut rng, 0, 2, &psk, value, GAS_LIMIT, 1)
        .expect("Failed to create a stake transaction");
    let executed_txs = generator_procedure(
        rusk,
        &[tx],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![],
        None,
    )
    .expect("generator procedure to succeed");
    if let Some(e) = &executed_txs
        .first()
        .expect("Transaction must be executed")
        .err
    {
        panic!("Stake transaction failed due to {e}")
    }

    let stake = wallet.get_stake(2).expect("stake to be found");
    let stake_value = stake.amount.expect("stake should have an amount").0;

    assert_eq!(stake_value, value);

    wallet
        .get_stake(0)
        .expect("stakeinfo to be found")
        .amount
        .expect("stake amount to be found");

    let tx = wallet
        .unstake(&mut rng, 0, 0, &psk, GAS_LIMIT, 1)
        .expect("Failed to unstake");
    let spent_txs = generator_procedure(
        rusk,
        &[tx],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![],
        None,
    )
    .expect("generator procedure to succeed");
    let spent_tx = spent_txs.first().expect("Unstake tx to be included");
    assert_eq!(spent_tx.err, None, "unstake to be successfull");

    let stake = wallet.get_stake(0).expect("stake should still be state");
    assert_eq!(stake.amount, None);

    let tx = wallet
        .withdraw(&mut rng, 0, 1, &psk, GAS_LIMIT, 1)
        .expect("failed to withdraw reward");
    generator_procedure(
        rusk,
        &[tx],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![],
        None,
    )
    .expect("generator procedure to succeed");

    let stake = wallet.get_stake(1).expect("stake should still be state");
    assert_eq!(stake.reward, 0);
}

#[tokio::test(flavor = "multi_thread")]
pub async fn stake() -> Result<()> {
    // Setup the logger
    logger();

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
        TestProverClient::default(),
    );

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    // Perform some staking actions.
    wallet_stake(&rusk, &wallet, MINIMUM_STAKE);

    // Check the state's root is changed from the original one
    let new_root = rusk.state_root();
    info!(
        "New root after the 1st transfer: {:?}",
        hex::encode(new_root)
    );
    assert_ne!(original_root, new_root, "Root should have changed");

    // let recv = kadcast_recv.try_recv();
    // let (_, _, h) = recv.expect("Transaction has not been locally
    // propagated"); assert_eq!(h, 0, "Transaction locally propagated with
    // wrong height");

    Ok(())
}

/// Attempt to submit a management transaction intending it to fail. Verify that
/// the reward amount remains unchanged and confirm that the transaction indeed
/// fails
fn wallet_reward(
    rusk: &Rusk,
    wallet: &wallet::Wallet<TestStore, TestStateClient, TestProverClient>,
) {
    // Sender psk
    let psk = SSK.public_spend_key();

    let mut rng = StdRng::seed_from_u64(0xdead);

    let bls_key = wallet.store().retrieve_sk(2).unwrap();
    let bls_key = PublicKey::from(&bls_key);
    let reward_calldata = (bls_key, 6u32);

    let stake = wallet.get_stake(2).expect("stake to be found");
    assert_eq!(stake.reward, 0, "stake reward must be empty");

    let tx = wallet
        .execute(
            &mut rng,
            rusk_abi::STAKE_CONTRACT.to_bytes().into(),
            String::from("reward"),
            reward_calldata,
            0,
            &psk,
            GAS_LIMIT,
            1,
        )
        .expect("Failed to create a reward transaction");
    let executed_txs = generator_procedure(
        rusk,
        &[tx],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![],
        None,
    )
    .expect("generator procedure to succeed");
    let _ = executed_txs
        .first()
        .expect("Transaction must be executed")
        .err
        .as_ref()
        .expect("reward transaction to fail");
    let stake = wallet.get_stake(2).expect("stake to be found");
    assert_eq!(stake.reward, 0, "stake reward must be empty");
}

#[tokio::test(flavor = "multi_thread")]
pub async fn reward() -> Result<()> {
    // Setup the logger
    logger();

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
        TestProverClient::default(),
    );

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    // Perform some staking actions.
    wallet_reward(&rusk, &wallet);

    // Check the state's root is changed from the original one
    let new_root = rusk.state_root();
    info!(
        "New root after the 1st transfer: {:?}",
        hex::encode(new_root)
    );
    assert_ne!(original_root, new_root, "Root should have changed");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn slash() -> Result<()> {
    // Setup the logger
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = slash_state(&tmp)?;

    let cache = Arc::new(RwLock::new(HashMap::new()));

    // Create a wallet
    let wallet = wallet::Wallet::new(
        TestStore,
        TestStateClient {
            rusk: rusk.clone(),
            cache,
        },
        TestProverClient::default(),
    );

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    let module_balance = rusk
        .module_balance(STAKE_CONTRACT)
        .expect("balance to exists");
    let to_slash = wallet.public_key(0).unwrap();
    let stake = wallet.get_stake(0).unwrap();
    assert_eq!(stake.reward, dusk(3.0));
    assert_eq!(stake.amount, Some((dusk(20.0), 0)));

    generator_procedure(
        &rusk,
        &[],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![to_slash],
        None,
    )
    .expect("to work");
    let after_slash = wallet.get_stake(0).unwrap();
    assert_eq!(after_slash.reward, 0);
    assert_eq!(after_slash.amount, Some((dusk(7.0), 0)));
    let new_balance = rusk.module_balance(STAKE_CONTRACT).unwrap();
    assert_eq!(new_balance, module_balance - dusk(13.0));
    let module_balance = new_balance;

    generator_procedure(
        &rusk,
        &[],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![to_slash],
        None,
    )
    .expect("to work");
    let after_slash = wallet.get_stake(0).unwrap();
    assert_eq!(after_slash.reward, 0);
    assert_eq!(after_slash.amount, Some((0, 0)));
    let new_balance = rusk.module_balance(STAKE_CONTRACT).unwrap();
    assert_eq!(new_balance, module_balance - dusk(7.0));
    let module_balance = new_balance;

    generator_procedure(
        &rusk,
        &[],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![to_slash],
        None,
    )
    .expect("to work");
    let after_slash = wallet.get_stake(0).unwrap();
    assert_eq!(after_slash.reward, 0);
    assert_eq!(after_slash.amount, Some((0, 0)));
    let new_balance = rusk.module_balance(STAKE_CONTRACT).unwrap();
    assert_eq!(new_balance, module_balance);

    generator_procedure(
        &rusk,
        &[],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![wallet.public_key(1).unwrap()],
        None,
    )
    .expect_err("Slashing a public key that never staked must fail");

    // Check the state's root is changed from the original one
    let new_root = rusk.state_root();
    info!(
        "New root after the 1st transfer: {:?}",
        hex::encode(new_root)
    );
    assert_ne!(original_root, new_root, "Root should have changed");

    Ok(())
}
