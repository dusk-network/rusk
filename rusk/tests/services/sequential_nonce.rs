// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use dusk_bytes::Serializable;
use dusk_core::transfer::data::TransactionData;
use rand::prelude::*;
use rusk::{Result, Rusk};
use tempfile::tempdir;
use tracing::info;

use crate::common::logger;
use crate::common::state::{
    generator_procedure, new_state_from_config_with_block_gas_limit,
    ExecuteResult,
};
use crate::common::wallet::{
    test_wallet as wallet, TestStateClient, TestStore,
};

const BLOCK_HEIGHT: u64 = 1;
// This is purposefully chosen to be low to trigger the discarding of a
// perfectly good transaction.
const BLOCK_GAS_LIMIT: u64 = 24_000_000;

const GAS_LIMIT: u64 = 12_000_000; // Lowest value for a transfer
const INITIAL_BALANCE: u64 = 10_000_000_000;

/// Executes three different transactions in the same block, expecting only two
/// to be included due to exceeding the block gas limit
fn wallet_transfer(
    rusk: &Rusk,
    wallet: &wallet::Wallet<TestStore, TestStateClient>,
    amount: u64,
) {
    for i in 0..3 {
        let account = wallet
            .account_public_key(i)
            .expect("Failed to get the account");
        info!(
            "Account {i}: {}",
            bs58::encode(account.to_bytes()).into_string()
        );
    }
    // Generate a receiver pk
    let receiver = wallet
        .account_public_key(3)
        .expect("Failed to get public key");

    let initial_balance_0 = wallet
        .get_account(0)
        .expect("Failed to get the balance")
        .balance;
    let initial_balance_1 = wallet
        .get_account(1)
        .expect("Failed to get the balance")
        .balance;
    let initial_balance_2 = wallet
        .get_account(2)
        .expect("Failed to get the balance")
        .balance;

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

    const TOTAL_TX: u64 = 50;
    let mut idxs: Vec<u64> = (1..=TOTAL_TX).collect();
    let mut rng = thread_rng(); // Get a random number generator
    idxs.shuffle(&mut rng);

    let mut txs = Vec::with_capacity(TOTAL_TX as usize);
    for i in idxs {
        let tx = wallet
            .moonlight_transaction(
                0,
                Some(receiver),
                amount,
                0,
                GAS_LIMIT,
                1,
                None::<TransactionData>,
                Some(i),
            )
            .expect("Failed to transfer");

        txs.push(tx);
    }

    let expected = ExecuteResult {
        discarded: 0,
        executed: TOTAL_TX as usize,
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
            .get_account(3)
            .expect("Failed to get the balance")
            .balance,
        TOTAL_TX * amount,
        "Wrong resulting balance for the receiver"
    );

    let final_balance_0 = wallet
        .get_account(0)
        .expect("Failed to get the balance")
        .balance;
    let gas_limit_0 = txs[0].gas_limit();
    let gas_price_0 = txs[0].gas_price();
    let fee_0 = gas_limit_0 * gas_price_0;

    assert!(
        initial_balance_0 - (amount + fee_0) * TOTAL_TX <= final_balance_0,
        "Final sender balance {} should be greater or equal than {}",
        final_balance_0,
        initial_balance_0 - (amount + fee_0) * TOTAL_TX
    );

    assert!(
        initial_balance_0 - amount * TOTAL_TX >= final_balance_0,
        "Final sender balance {} should be lesser or equal than {}",
        final_balance_0,
        initial_balance_0 - amount * TOTAL_TX
    );
}

#[tokio::test(flavor = "multi_thread")]
pub async fn multi_transfer() -> Result<()> {
    // Setup the logger
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = new_state_from_config_with_block_gas_limit(
        &tmp,
        include_str!("../config/sequential_nonce.toml"),
        BLOCK_GAS_LIMIT,
    )
    .await?;

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

    info!("Original Root: {}", hex::encode(original_root));

    wallet_transfer(&rusk, &wallet, 1_000);

    // Check the state's root is changed from the original one
    let new_root = rusk.state_root();
    info!("New root after the 1st transfer: {}", hex::encode(new_root));
    assert_ne!(original_root, new_root, "Root should have changed");

    Ok(())
}
