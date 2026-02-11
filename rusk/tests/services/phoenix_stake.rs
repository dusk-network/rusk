// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::stake::DEFAULT_MINIMUM_STAKE;
use dusk_core::{
    dusk,
    stake::{StakeAmount, STAKE_CONTRACT},
};

use anyhow::Result;
use dusk_rusk_test::TestContext;
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::node::RuskVmConfig;
use tracing::info;

use crate::common::*;

const BLOCK_HEIGHT: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;
const GAS_LIMIT: u64 = 10_000_000_000;
const GAS_PRICE: u64 = 1;

// Creates the Rusk initial state for the tests below
async fn stake_state() -> Result<TestContext> {
    let state_toml = include_str!("../config/stake.toml");
    let vm_config = RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);

    TestContext::instantiate(state_toml, vm_config).await
}

/// Stakes an amount Dusk and produces a block with this single transaction,
/// checking the stake is set successfully. It then proceeds to withdraw the
/// stake and checking it is correctly withdrawn.
fn wallet_stake(tc: &TestContext, value: u64) {
    let wallet = tc.wallet();

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
        "stake amount not to be found"
    );

    let tx = wallet
        .phoenix_stake(&mut rng, 0, 2, value, GAS_LIMIT, GAS_PRICE)
        .expect("Failed to create a stake transaction");
    let _ = tc.execute_transaction(tx, BLOCK_HEIGHT, None);

    let stake = wallet.get_stake(2).expect("stake to be found");
    let stake_value = stake.amount.expect("stake should have an amount").value;

    assert_eq!(stake_value, value);

    wallet
        .get_stake(0)
        .expect("stakeinfo to be found")
        .amount
        .expect("stake amount to be found");

    let tx = wallet
        .phoenix_unstake(&mut rng, 0, 0, GAS_LIMIT, GAS_PRICE)
        .expect("Failed to unstake");
    let _ = tc.execute_transaction(tx, BLOCK_HEIGHT, None);

    let stake = wallet.get_stake(0).expect("stake should still be state");
    assert_eq!(stake.amount, None);

    let tx = wallet
        .phoenix_stake_withdraw(&mut rng, 0, 1, GAS_LIMIT, GAS_PRICE)
        .expect("failed to withdraw reward");
    let _ = tc.execute_transaction(tx, BLOCK_HEIGHT, None);

    let stake = wallet.get_stake(1).expect("stake should still be state");
    assert_eq!(stake.reward, 0);
}

#[tokio::test(flavor = "multi_thread")]
pub async fn stake() -> Result<()> {
    // Setup the logger
    logger();

    let tc = stake_state().await?;

    let original_root = tc.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    // Perform some staking actions.
    wallet_stake(&tc, DEFAULT_MINIMUM_STAKE);

    // Check the state's root is changed from the original one
    let new_root = tc.state_root();
    info!(
        "New root after the 1st transfer: {:?}",
        hex::encode(new_root)
    );
    assert_ne!(original_root, new_root, "Root should have changed");

    Ok(())
}

/// Attempt to submit a management transaction intending it to fail. Verify that
/// the reward amount remains unchanged and confirm that the transaction indeed
/// fails
fn wallet_reward(tc: &TestContext) {
    let wallet = tc.wallet();

    let mut rng = StdRng::seed_from_u64(0xdead);

    let stake = wallet.get_stake(2).expect("stake to be found");
    assert_eq!(stake.reward, 0, "stake reward must be empty");

    let tx = wallet
        .phoenix_stake_withdraw(&mut rng, 0, 2, GAS_LIMIT, GAS_PRICE)
        .expect("Creating reward transaction should succeed");
    let _ = tc.execute_transaction(
        tx,
        BLOCK_HEIGHT,
        "Panic: A stake should exist in the map to get rewards!",
    );

    let stake = wallet.get_stake(2).expect("stake to be found");
    assert_eq!(stake.reward, 0, "stake reward must be empty");
}

#[tokio::test(flavor = "multi_thread")]
pub async fn reward() -> Result<()> {
    // Setup the logger
    logger();

    let tc = stake_state().await?;

    let original_root = tc.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    // Perform some staking actions.
    wallet_reward(&tc);

    // Check the state's root is changed from the original one
    let new_root = tc.state_root();
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

    let state_toml = include_str!("../config/slash.toml");
    let vm_config = RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);

    let tc = TestContext::instantiate(state_toml, vm_config).await?;

    let rusk = tc.rusk();
    let wallet = tc.wallet();

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    let contract_balance = rusk
        .contract_balance(&STAKE_CONTRACT)
        .expect("balance to exists");
    let to_slash = wallet.account_public_key(0).unwrap();
    let stake = wallet.get_stake(0).unwrap();
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

    tc.slash(BLOCK_HEIGHT, vec![to_slash]).expect("to work");
    tc.slash(BLOCK_HEIGHT, vec![to_slash]).expect("to work");

    let last_changes = rusk.last_provisioners_change(None).unwrap();
    let (_, prev) = last_changes.first().expect("Something changed");
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

    let after_slash = wallet.get_stake(0).unwrap();
    assert_eq!(after_slash.reward, dusk(3.0));
    assert_eq!(
        after_slash.amount,
        Some(StakeAmount {
            value: prev_stake - slashed_amount,
            eligibility: 4320,
            locked: dusk(2.0)
        })
    );
    let new_balance = rusk.contract_balance(&STAKE_CONTRACT).unwrap();
    assert_eq!(new_balance, contract_balance);

    tc.slash(BLOCK_HEIGHT + 1, vec![to_slash]).expect("to work");

    let last_changes = rusk.last_provisioners_change(None).unwrap();
    let (_, prev) = last_changes.first().expect("Something changed");
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

    let after_slash = wallet.get_stake(0).unwrap();
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

    let new_balance = rusk.contract_balance(&STAKE_CONTRACT).unwrap();
    assert_eq!(new_balance, contract_balance);

    tc.slash(9000, vec![to_slash]).expect("to work");

    let last_changes = rusk.last_provisioners_change(None).unwrap();
    let (_, prev) = last_changes.first().expect("Something changed");
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
    let after_slash = wallet.get_stake(0).unwrap();

    assert_eq!(after_slash.reward, dusk(3.0));
    assert_eq!(
        after_slash.amount,
        Some(StakeAmount {
            value: dusk(10.08),
            eligibility: 17280,
            locked: prev_locked + slashed_amount
        })
    );
    let new_balance = rusk.contract_balance(&STAKE_CONTRACT).unwrap();
    assert_eq!(new_balance, contract_balance);

    tc.slash(9001, vec![wallet.account_public_key(1).unwrap()])
        .expect_err("Slashing a public key that never staked must fail");

    // Ensure we still have previous changes, because generator procedure failed
    let last_changes = rusk.last_provisioners_change(None).unwrap();
    let (_, prev) = last_changes.first().expect("Something changed");
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

    tc.slash(9001, vec![]).expect("To work properly");
    let last_changes = rusk.last_provisioners_change(None).unwrap();
    assert_eq!(0, last_changes.len(), "No changes expected");

    // Check the state's root is changed from the original one
    let new_root = rusk.state_root();
    info!("New root: {}", hex::encode(new_root));
    assert_ne!(original_root, new_root, "Root should have changed");

    Ok(())
}
