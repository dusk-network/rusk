// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Integration test for the withdrawal replay nullifier-count hook.
//!
//! The hook (registered in `vm::execute`) validates that a Phoenix
//! `WithdrawReplayToken` carries exactly the same number of nullifiers as the
//! encapsulating transaction — a defense-in-depth measure for audit finding
//! P1.6-3.
//!
//! Tests exercise both happy and unhappy paths:
//! - Happy: a legitimate Phoenix unstake with matching nullifier counts passes.
//! - Unhappy: a tampered unstake with fewer nullifiers in the replay token is
//!   rejected by the hook.

use dusk_core::stake::DEFAULT_MINIMUM_STAKE;
use dusk_core::transfer::TRANSFER_CONTRACT;
use dusk_core::transfer::data::ContractCall;

use anyhow::Result;
use dusk_rusk_test::TestContext;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rusk::node::{FEATURE_HARDFORK_BOREAS, RuskVmConfig};
use tracing::info;

use crate::common::*;

const BLOCK_HEIGHT: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;
const GAS_LIMIT: u64 = 10_000_000_000;
const GAS_PRICE: u64 = 1;

async fn stake_state() -> Result<TestContext> {
    let state_toml = include_str!("../config/stake.toml");
    let mut vm_config =
        RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);
    vm_config.with_feature(FEATURE_HARDFORK_BOREAS, 1);

    TestContext::instantiate(state_toml, vm_config).await
}

/// Performs a Phoenix unstake, which triggers an inter-contract call from the
/// stake contract to the transfer contract's `withdraw` function. The
/// withdrawal replay hook must allow this legitimate call through.
#[tokio::test(flavor = "multi_thread")]
pub async fn phoenix_unstake_passes_withdrawal_replay_hook() -> Result<()> {
    logger();

    let tc = stake_state().await?;
    let wallet = tc.wallet();
    let mut rng = StdRng::seed_from_u64(0xbeef);

    // Verify the stake exists before unstaking.
    let stake = wallet.get_stake(0).expect("stake info should exist");
    let staked = stake.amount.expect("stake should have an amount");
    assert_eq!(staked.value, DEFAULT_MINIMUM_STAKE);
    info!("Stake confirmed: {} Dusk", staked.value);

    // Unstake — this triggers stake→transfer `withdraw` ICC.
    let tx = wallet
        .phoenix_unstake(&mut rng, 0, 0, GAS_LIMIT, GAS_PRICE)
        .expect("Failed to create unstake transaction");

    info!("Phoenix unstake tx nullifiers: {}", tx.nullifiers().len());

    // Execute. If the hook incorrectly rejects valid withdrawals, this panics.
    let spent = tc.execute_transaction(tx, BLOCK_HEIGHT, None);
    info!("Unstake gas spent: {}", spent.gas_spent);

    // Confirm the stake was removed.
    let stake = wallet.get_stake(0).expect("stake info should exist");
    assert!(stake.amount.is_none(), "stake should have been removed");

    Ok(())
}

/// Constructs a Phoenix unstake with a replay token carrying fewer nullifiers
/// than the encapsulating transaction. The withdrawal replay hook must reject
/// this — the transaction should be discarded (unspendable) by the block
/// generator.
#[tokio::test(flavor = "multi_thread")]
pub async fn mismatched_nullifier_count_rejected_by_hook() -> Result<()> {
    use crate::common::state::{ExecuteResult, generator_procedure};

    logger();

    let tc = stake_state().await?;
    let wallet = tc.wallet();
    let mut rng = StdRng::seed_from_u64(0xdead);

    // Build a transaction whose replay token has one fewer nullifier
    // than the transaction's actual nullifier count.
    let tx = wallet
        .phoenix_unstake_bad_replay_token(&mut rng, 0, 0, GAS_LIMIT, GAS_PRICE)
        .expect("Failed to create tampered unstake transaction");

    let tx_nullifiers = tx.nullifiers().len();
    info!(
        "Tampered tx nullifiers: {tx_nullifiers}, \
         replay token nullifiers: {}",
        tx_nullifiers - 1
    );

    // The hook returns false on mismatch. The spending part of the
    // transaction succeeds, but the inter-contract call to `withdraw`
    // fails with "call hook rejected". The transaction is still included
    // in the block (executed, not discarded), but with an error — the
    // full gas limit is consumed and the withdrawal has no effect.
    let spent_txs = generator_procedure(
        tc.rusk(),
        &[tx],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![],
        Some(ExecuteResult {
            executed: 1,
            discarded: 0,
        }),
    )
    .expect("generator procedure should succeed");

    assert_eq!(spent_txs.len(), 1, "tx should be included in the block");
    let err = spent_txs[0].err.as_ref().expect("tx should have an error");
    assert!(
        err.contains("call hook rejected"),
        "error should mention call hook rejection, got: {err}",
    );

    // Stake must remain intact since the withdrawal failed.
    let stake = wallet.get_stake(0).expect("stake info should exist");
    assert!(
        stake.amount.is_some(),
        "stake should remain after discarded unstake"
    );

    Ok(())
}

/// Constructs a Phoenix transaction that calls `TRANSFER_CONTRACT::withdraw`
/// with garbage (non-deserializable) `fn_args`. The fail-closed hook must
/// reject this — the transaction is included but errors out.
#[tokio::test(flavor = "multi_thread")]
pub async fn garbage_withdraw_args_rejected_by_hook() -> Result<()> {
    use crate::common::state::{ExecuteResult, generator_procedure};

    logger();

    let tc = stake_state().await?;
    let wallet = tc.wallet();
    let mut rng = StdRng::seed_from_u64(0xcafe);

    // Build a Phoenix transaction that directly calls
    // transfer::withdraw with garbage bytes.
    let garbage_call = ContractCall::new(TRANSFER_CONTRACT, "withdraw")
        .with_raw_args(vec![0xDE, 0xAD, 0xBE, 0xEF]);

    let tx = wallet
        .phoenix_execute(&mut rng, 0, GAS_LIMIT, GAS_PRICE, 0, garbage_call)
        .expect("Failed to create garbage-args transaction");

    info!("Garbage withdraw tx nullifiers: {}", tx.nullifiers().len());

    let spent_txs = generator_procedure(
        tc.rusk(),
        &[tx],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![],
        Some(ExecuteResult {
            executed: 1,
            discarded: 0,
        }),
    )
    .expect("generator procedure should succeed");

    assert_eq!(spent_txs.len(), 1, "tx should be included in the block");
    let err = spent_txs[0].err.as_ref().expect("tx should have an error");
    assert!(
        err.contains("call hook rejected"),
        "error should mention call hook rejection, got: {err}",
    );

    Ok(())
}
