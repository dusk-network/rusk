// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::Path;

use execution_core::stake::MINIMUM_STAKE;
use execution_core::{
    dusk,
    stake::{StakeAmount, STAKE_CONTRACT},
};
use rusk_wallet::{currency::Lux, gas::Gas, Wallet};

use rusk::{Result, Rusk};
use tempfile::tempdir;
use tracing::info;

use crate::common::state::{generator_procedure, new_state};
use crate::common::wallet::{test_wallet, WalletFile};
use crate::common::*;

const BLOCK_HEIGHT: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;
const GAS_LIMIT: u64 = 10_000_000_000;
const GAS_PRICE: Lux = 1;

const SENDER_INDEX_0: u8 = 0;
const SENDER_INDEX_1: u8 = 1;
const SENDER_INDEX_2: u8 = 2;

// Creates the Rusk initial state for the tests below
fn stake_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot = toml::from_str(include_str!("../config/stake.toml"))
        .expect("Cannot deserialize config");

    new_state(dir, &snapshot, BLOCK_GAS_LIMIT)
}

// Creates the Rusk initial state for the tests below
fn slash_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot = toml::from_str(include_str!("../config/slash.toml"))
        .expect("Cannot deserialize config");

    new_state(dir, &snapshot, BLOCK_GAS_LIMIT)
}

/// Stakes an amount Dusk and produces a block with this single transaction,
/// checking the stake is set successfully. It then proceeds to withdraw the
/// stake and checking it is correctly withdrawn.
async fn wallet_stake(rusk: &Rusk, wallet: &Wallet<WalletFile>, value: u64) {
    let sender_addr_0 = &wallet.addresses()[SENDER_INDEX_0 as usize];

    wallet
        .stake_info(SENDER_INDEX_0)
        .await
        .expect("stakeinfo to be found")
        .expect("stake to be Some")
        .amount
        .expect("stake amount to be found");

    assert!(
        wallet
            .stake_info(SENDER_INDEX_2)
            .await
            .expect("stakeinfo to be found")
            .expect("stake to be Some")
            .amount
            .is_none(),
        "stake amount not to be found"
    );

    let tx = wallet
        .phoenix_stake(
            sender_addr_0,
            value.into(),
            Gas {
                limit: GAS_LIMIT,
                price: GAS_PRICE,
            },
        )
        .await
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

    let stake = wallet
        .stake_info(SENDER_INDEX_2)
        .await
        .expect("stake to be found")
        .expect("stake to be Some");
    let stake_value = stake.amount.expect("stake should have an amount").value;

    assert_eq!(stake_value, value);

    wallet
        .stake_info(SENDER_INDEX_0)
        .await
        .expect("stakeinfo to be found")
        .expect("stake to be Some")
        .amount
        .expect("stake amount to be found");

    let tx = wallet
        .phoenix_unstake(
            sender_addr_0,
            Gas {
                limit: GAS_LIMIT,
                price: GAS_PRICE,
            },
        )
        .await
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

    let stake = wallet
        .stake_info(SENDER_INDEX_0)
        .await
        .expect("stake should still be state")
        .expect("stake to be Some");
    assert_eq!(stake.amount, None);

    let tx = wallet
        .phoenix_stake_withdraw(
            sender_addr_0,
            1,
            Gas {
                limit: GAS_LIMIT,
                price: GAS_PRICE,
            },
        )
        .await
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

    let stake = wallet
        .stake_info(SENDER_INDEX_1)
        .await
        .expect("stake should still be state")
        .expect("Stake to be some");
    assert_eq!(stake.reward, 0);
}

#[tokio::test(flavor = "multi_thread")]
pub async fn stake() -> Result<()> {
    // Setup the logger
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = stake_state(&tmp)?;

    // Create a wallet
    let wallet = test_wallet()?;

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    // Perform some staking actions.
    wallet_stake(&rusk, &wallet, MINIMUM_STAKE).await;

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
async fn wallet_reward(rusk: &Rusk, wallet: &Wallet<WalletFile>) {
    let sender_addr_0 = &wallet.addresses()[SENDER_INDEX_0 as usize];

    let tx = wallet
        .phoenix_stake_withdraw(
            sender_addr_0,
            SENDER_INDEX_2,
            Gas {
                limit: GAS_LIMIT,
                price: GAS_PRICE,
            },
        )
        .await
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
    let stake = wallet
        .stake_info(SENDER_INDEX_2)
        .await
        .expect("stake to be found")
        .expect("stake to be some");
    assert_eq!(stake.reward, 0, "stake reward must be empty");
}

#[tokio::test(flavor = "multi_thread")]
pub async fn reward() -> Result<()> {
    // Setup the logger
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = stake_state(&tmp)?;

    // Create a wallet
    let wallet = test_wallet()?;

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    // Perform some staking actions.
    wallet_reward(&rusk, &wallet).await;

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

    // Create a wallet
    let wallet = test_wallet()?;

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    let contract_balance = rusk
        .contract_balance(STAKE_CONTRACT)
        .expect("balance to exists");
    let to_slash = wallet.bls_public_key(SENDER_INDEX_0);
    let stake = wallet.stake_info(SENDER_INDEX_0).await.unwrap().unwrap();
    let initial_stake_value = dusk(20.0);
    assert_eq!(stake.reward, dusk(3.0));
    assert_eq!(
        stake.amount,
        Some(StakeAmount {
            value: initial_stake_value,
            eligibility: 0,
            locked: 0
        })
    );

    generator_procedure(
        &rusk,
        &[],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![to_slash],
        None,
    )
    .expect("to work");
    generator_procedure(
        &rusk,
        &[],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![to_slash],
        None,
    )
    .expect("to work");

    let last_changes = rusk.last_provisioners_change(None).unwrap();
    let (_, prev) = last_changes.first().expect("Something changed").clone();
    let prev = prev.expect("to have something");
    assert_eq!(prev.reward, dusk(3.0));
    assert_eq!(
        prev.amount,
        Some(StakeAmount {
            value: dusk(20.0),
            eligibility: 0,
            locked: 0
        })
    );

    let prev_stake = prev.amount.unwrap().value;
    let slashed_amount = prev_stake / 10;

    let after_slash = wallet.stake_info(SENDER_INDEX_0).await.unwrap().unwrap();
    assert_eq!(after_slash.reward, dusk(3.0));
    assert_eq!(
        after_slash.amount,
        Some(StakeAmount {
            value: prev_stake - slashed_amount,
            eligibility: 4320,
            locked: dusk(2.0)
        })
    );
    let new_balance = rusk.contract_balance(STAKE_CONTRACT).unwrap();
    assert_eq!(new_balance, contract_balance);

    generator_procedure(
        &rusk,
        &[],
        BLOCK_HEIGHT + 1,
        BLOCK_GAS_LIMIT,
        vec![to_slash],
        None,
    )
    .expect("to work");

    let last_changes = rusk.last_provisioners_change(None).unwrap();
    let (_, prev) = last_changes.first().expect("Something changed").clone();
    let prev = prev.expect("to have something");
    assert_eq!(prev.reward, dusk(3.0));
    assert_eq!(
        prev.amount,
        Some(StakeAmount {
            value: dusk(18.0),
            eligibility: 4320,
            locked: dusk(2.0)
        })
    );

    let prev_stake = prev.amount.unwrap().value;
    let prev_locked = prev.amount.unwrap().locked;
    // 20% slash
    let slashed_amount = prev_stake / 10 * 2;

    let after_slash = wallet.stake_info(SENDER_INDEX_0).await.unwrap().unwrap();
    assert_eq!(after_slash.reward, dusk(3.0));
    assert_eq!(
        after_slash.amount,
        Some(StakeAmount {
            value: prev_stake - slashed_amount,
            eligibility: 6480,
            locked: prev_locked + slashed_amount
        })
    );
    assert_eq!(
        after_slash.amount,
        Some(StakeAmount {
            value: dusk(14.4),
            eligibility: 6480,
            locked: dusk(20.0) - dusk(14.4)
        })
    );

    let new_balance = rusk.contract_balance(STAKE_CONTRACT).unwrap();
    assert_eq!(new_balance, contract_balance);

    generator_procedure(
        &rusk,
        &[],
        9000,
        BLOCK_GAS_LIMIT,
        vec![to_slash],
        None,
    )
    .expect("to work");

    let last_changes = rusk.last_provisioners_change(None).unwrap();
    let (_, prev) = last_changes.first().expect("Something changed").clone();
    let prev = prev.expect("to have something");
    assert_eq!(prev.reward, dusk(3.0));
    assert_eq!(
        prev.amount,
        Some(StakeAmount {
            value: dusk(14.4),
            eligibility: 6480,
            locked: dusk(20.0) - dusk(14.4)
        })
    );

    let prev_stake = prev.amount.unwrap().value;
    let prev_locked = prev.amount.unwrap().locked;
    // 30% slash
    let slashed_amount = prev_stake / 10 * 3;
    let after_slash = wallet.stake_info(SENDER_INDEX_0).await.unwrap().unwrap();

    assert_eq!(after_slash.reward, dusk(3.0));
    assert_eq!(
        after_slash.amount,
        Some(StakeAmount {
            value: dusk(10.08),
            eligibility: 17280,
            locked: prev_locked + slashed_amount
        })
    );
    let new_balance = rusk.contract_balance(STAKE_CONTRACT).unwrap();
    assert_eq!(new_balance, contract_balance);

    generator_procedure(
        &rusk,
        &[],
        9001,
        BLOCK_GAS_LIMIT,
        vec![wallet.bls_public_key(SENDER_INDEX_1)],
        None,
    )
    .expect_err("Slashing a public key that never staked must fail");

    //Ensure we still have previous changes, because generator procedure failed
    let last_changes = rusk.last_provisioners_change(None).unwrap();
    let (_, prev) = last_changes.first().expect("Something changed").clone();
    let prev = prev.expect("to have something");
    assert_eq!(prev.reward, dusk(3.0));
    assert_eq!(
        prev.amount,
        Some(StakeAmount {
            value: dusk(14.4),
            eligibility: 6480,
            locked: dusk(20.0) - dusk(14.4)
        })
    );

    generator_procedure(&rusk, &[], 9001, BLOCK_GAS_LIMIT, vec![], None)
        .expect("To work properly");
    let last_changes = rusk.last_provisioners_change(None).unwrap();
    assert_eq!(0, last_changes.len(), "No changes expected");

    // Check the state's root is changed from the original one
    let new_root = rusk.state_root();
    info!("New root: {}", hex::encode(new_root));
    assert_ne!(original_root, new_root, "Root should have changed");

    Ok(())
}
