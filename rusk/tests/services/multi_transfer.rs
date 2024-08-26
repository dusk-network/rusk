// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::{Result, Rusk};
use tempfile::tempdir;
use test_wallet::{self as wallet};
use tracing::info;

use crate::common::logger;
use crate::common::state::{generator_procedure, new_state, ExecuteResult};
use crate::common::wallet::{TestStateClient, TestStore};

const BLOCK_HEIGHT: u64 = 1;
// This is purposefully chosen to be low to trigger the discarding of a
// perfectly good transaction.
const BLOCK_GAS_LIMIT: u64 = 24_000_000;
const GAS_LIMIT: u64 = 12_000_000; // Lowest value for a transfer
const INITIAL_BALANCE: u64 = 10_000_000_000;

// Creates the Rusk initial state for the tests below
fn initial_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot =
        toml::from_str(include_str!("../config/multi_transfer.toml"))
            .expect("Cannot deserialize config");

    new_state(dir, &snapshot, BLOCK_GAS_LIMIT)
}

/// Executes three different transactions in the same block, expecting only two
/// to be included due to exceeding the block gas limit
fn wallet_transfer(
    rusk: &Rusk,
    wallet: &wallet::Wallet<TestStore, TestStateClient>,
    amount: u64,
) {
    // Generate a receiver pk
    let receiver = wallet
        .phoenix_public_key(3)
        .expect("Failed to get public key");

    let mut rng = StdRng::seed_from_u64(0xdead);

    let initial_balance_0 = wallet
        .get_balance(0)
        .expect("Failed to get the balance")
        .value;
    let initial_balance_1 = wallet
        .get_balance(1)
        .expect("Failed to get the balance")
        .value;
    let initial_balance_2 = wallet
        .get_balance(2)
        .expect("Failed to get the balance")
        .value;

    // Check the senders initial balance is correct
    assert_eq!(
        initial_balance_0, INITIAL_BALANCE,
        "Wrong initial balance for the sender"
    );
    assert_eq!(
        initial_balance_1, INITIAL_BALANCE,
        "Wrong initial balance for the sender"
    );
    assert_eq!(
        initial_balance_2, INITIAL_BALANCE,
        "Wrong initial balance for the sender"
    );

    // Check the receiver initial balance is zero
    assert_eq!(
        wallet
            .get_balance(3)
            .expect("Failed to get the balance")
            .value,
        0,
        "Wrong initial balance for the receiver"
    );

    let mut txs = Vec::with_capacity(3);

    for i in 0..3 {
        let tx = wallet
            .phoenix_transfer(&mut rng, i, &receiver, amount, GAS_LIMIT, 1)
            .expect("Failed to transfer");
        txs.push(tx);
    }

    let expected = ExecuteResult {
        discarded: 0,
        executed: 2,
    };

    generator_procedure(
        rusk,
        &txs[..],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![],
        Some(expected),
    )
    .expect("generator procedure to succeed");

    // Check the receiver's balance is changed accordingly
    assert_eq!(
        wallet
            .get_balance(3)
            .expect("Failed to get the balance")
            .value,
        2 * amount,
        "Wrong resulting balance for the receiver"
    );

    let final_balance_0 = wallet
        .get_balance(0)
        .expect("Failed to get the balance")
        .value;
    let gas_limit_0 = txs[0].gas_limit();
    let gas_price_0 = txs[0].gas_price();
    let fee_0 = gas_limit_0 * gas_price_0;

    let final_balance_1 = wallet
        .get_balance(1)
        .expect("Failed to get the balance")
        .value;
    let gas_limit_1 = txs[1].gas_limit();
    let gas_price_1 = txs[1].gas_price();
    let fee_1 = gas_limit_1 * gas_price_1;

    assert!(
        initial_balance_0 - amount - fee_0 <= final_balance_0,
        "Final sender balance {} should be greater or equal than {}",
        final_balance_0,
        initial_balance_0 - amount - fee_0
    );

    assert!(
        initial_balance_0 - amount >= final_balance_0,
        "Final sender balance {} should be lesser or equal than {}",
        final_balance_0,
        initial_balance_0 - amount
    );

    assert!(
        initial_balance_1 - amount - fee_1 <= final_balance_1,
        "Final sender balance {} should be greater or equal than {}",
        final_balance_1,
        initial_balance_1 - amount - fee_1
    );

    assert!(
        initial_balance_1 - amount >= final_balance_1,
        "Final sender balance {} should be lesser or equal than {}",
        final_balance_1,
        initial_balance_1 - amount
    );

    // Check the discarded transaction didn't change the balance
    assert_eq!(
        wallet
            .get_balance(2)
            .expect("Failed to get the balance")
            .value,
        initial_balance_2,
        "Wrong resulting balance for discarded TX sender"
    );
}

#[tokio::test(flavor = "multi_thread")]
pub async fn multi_transfer() -> Result<()> {
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
            cache,
        },
    );

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    wallet_transfer(&rusk, &wallet, 1_000);

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
