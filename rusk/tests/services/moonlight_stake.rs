// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::stake::{StakeData, DEFAULT_MINIMUM_STAKE};
use dusk_rusk_test::{Result, RuskVmConfig, TestContext};
use rand::prelude::*;
use rand::rngs::StdRng;
use tracing::info;

use crate::common::*;

const BLOCK_HEIGHT: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;
const GAS_LIMIT: u64 = 10_000_000_000;
const GAS_PRICE: u64 = 1;

// Creates the Rusk initial state for the tests below
async fn stake_state() -> Result<TestContext> {
    let state = include_str!("../config/stake.toml");
    let vm_config = RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);
    TestContext::instantiate(state, vm_config).await
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
        .moonlight_stake(0, 2, value, GAS_LIMIT, GAS_PRICE)
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
        .moonlight_unstake(&mut rng, 0, 0, GAS_LIMIT, GAS_PRICE)
        .expect("Failed to unstake");
    let _ = tc.execute_transaction(tx, BLOCK_HEIGHT, None);

    let stake = wallet.get_stake(0).expect("stake should still be state");
    assert_eq!(stake, StakeData::default());

    let tx = wallet
        .moonlight_stake_withdraw(&mut rng, 0, 1, GAS_LIMIT, GAS_PRICE)
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
        .moonlight_stake_withdraw(&mut rng, 0, 2, GAS_LIMIT, GAS_PRICE)
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
