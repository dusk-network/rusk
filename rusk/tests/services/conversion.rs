// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use node_data::ledger::SpentTransaction;
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::node::RuskVmConfig;
use rusk::{Result, Rusk};
use tempfile::tempdir;

use crate::common::logger;
use crate::common::state::{generator_procedure, new_state};
use crate::common::wallet::{
    test_wallet as wallet, TestStateClient, TestStore,
};

const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;

const INITIAL_PHOENIX_BALANCE: u64 = 10_000_000_000;
const INITIAL_MOONLIGHT_BALANCE: u64 = 10_000_000_000;

// Creates the Rusk initial state for the tests below
fn initial_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot = toml::from_str(include_str!("../config/convert.toml"))
        .expect("Cannot deserialize config");
    let vm_config = RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);

    new_state(dir, &snapshot, vm_config)
}

/// Makes a transaction that converts Dusk from Phoenix to Moonlight, and
/// produces a block with a single transaction, checking balances accordingly.
fn wallet_convert_to_moonlight(
    rusk: &Rusk,
    wallet: &wallet::Wallet<TestStore, TestStateClient>,
    block_height: u64,
) {
    const CONVERT_VALUE: u64 = INITIAL_PHOENIX_BALANCE / 2;
    const GAS_LIMIT: u64 = 1_000_000_000;

    let phoenix_balance = wallet
        .get_balance(0)
        .expect("Getting phoenix balance should succeed");
    let moonlight_account = wallet
        .get_account(0)
        .expect("Getting account data should succeed");

    assert_eq!(
        phoenix_balance.value, INITIAL_PHOENIX_BALANCE,
        "The Phoenix notes must be of its initial value"
    );
    assert_eq!(
        moonlight_account.balance, INITIAL_MOONLIGHT_BALANCE,
        "The Moonlight account should have its initial balance"
    );

    let mut rng = StdRng::seed_from_u64(0xdead);

    let tx = wallet
        .phoenix_to_moonlight(&mut rng, 0, 0, CONVERT_VALUE, GAS_LIMIT, 1)
        .expect("Creating conversion transaction should succeed");

    let txs: Vec<SpentTransaction> = generator_procedure(
        rusk,
        &[tx],
        block_height,
        BLOCK_GAS_LIMIT,
        vec![],
        None,
    )
    .expect("The generator procedure should succeed");

    let tx = txs.first().expect("tx to be processed");
    let gas_spent = tx.gas_spent;

    let phoenix_balance = wallet
        .get_balance(0)
        .expect("Getting phoenix balance should succeed");
    let moonlight_account = wallet
        .get_account(0)
        .expect("Getting account data should succeed");

    assert_eq!(
        phoenix_balance.value, INITIAL_PHOENIX_BALANCE - CONVERT_VALUE - gas_spent,
        "The Phoenix notes must be of their initial value minus the converted amount and gas spent"
    );
    assert_eq!(
        moonlight_account.balance, INITIAL_MOONLIGHT_BALANCE + CONVERT_VALUE,
        "The Moonlight account should have its initial balance plus the converted amount"
    );
}

/// Makes a transaction that converts Dusk from Phoenix to Moonlight, and
/// produces a block with a single transaction, checking balances accordingly.
fn wallet_convert_to_phoenix(
    rusk: &Rusk,
    wallet: &wallet::Wallet<TestStore, TestStateClient>,
    block_height: u64,
) {
    const CONVERT_VALUE: u64 = INITIAL_PHOENIX_BALANCE / 2;
    const GAS_LIMIT: u64 = 1_000_000_000;

    let moonlight_account = wallet
        .get_account(0)
        .expect("Getting account data should succeed");
    let phoenix_balance = wallet
        .get_balance(0)
        .expect("Getting phoenix balance should succeed");

    assert_eq!(
        moonlight_account.balance, INITIAL_MOONLIGHT_BALANCE,
        "The Moonlight account should have its initial balance"
    );
    assert_eq!(
        phoenix_balance.value, INITIAL_PHOENIX_BALANCE,
        "The Phoenix notes must be of its initial value"
    );

    let mut rng = StdRng::seed_from_u64(0xdead);

    let tx = wallet
        .moonlight_to_phoenix(&mut rng, 0, 0, CONVERT_VALUE, GAS_LIMIT, 1)
        .expect("Creating conversion transaction should succeed");

    let txs: Vec<SpentTransaction> = generator_procedure(
        rusk,
        &[tx],
        block_height,
        BLOCK_GAS_LIMIT,
        vec![],
        None,
    )
    .expect("The generator procedure should succeed");

    let tx = txs.first().expect("tx to be processed");
    let gas_spent = tx.gas_spent;

    let moonlight_account = wallet
        .get_account(0)
        .expect("Getting account data should succeed");
    let phoenix_balance = wallet
        .get_balance(0)
        .expect("Getting phoenix balance should succeed");

    assert_eq!(
        moonlight_account.balance, INITIAL_MOONLIGHT_BALANCE - CONVERT_VALUE - gas_spent,
        "The Moonlight account must have its initial value minus the converted amount and gas spent"
    );
    assert_eq!(
        phoenix_balance.value, INITIAL_PHOENIX_BALANCE + CONVERT_VALUE,
        "The Phoenix notes must be of their initial value minus plus the converted amount"
    );
}

#[tokio::test(flavor = "multi_thread")]
pub async fn convert_to_moonlight() -> Result<()> {
    const BLOCK_HEIGHT: u64 = 2;

    // Setup the logger
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp)?;

    let cache = Arc::new(RwLock::new(HashMap::new()));

    // Create a wallet
    let wallet = wallet::Wallet::new(
        TestStore,
        TestStateClient {
            rusk: rusk.clone(),
            cache: cache.clone(),
        },
    );

    wallet_convert_to_moonlight(&rusk, &wallet, BLOCK_HEIGHT);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn convert_to_phoenix() -> Result<()> {
    const BLOCK_HEIGHT: u64 = 2;

    // Setup the logger
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp)?;

    let cache = Arc::new(RwLock::new(HashMap::new()));

    // Create a wallet
    let wallet = wallet::Wallet::new(
        TestStore,
        TestStateClient {
            rusk: rusk.clone(),
            cache: cache.clone(),
        },
    );

    wallet_convert_to_phoenix(&rusk, &wallet, BLOCK_HEIGHT);

    Ok(())
}
