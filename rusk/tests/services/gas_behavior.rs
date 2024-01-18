// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use dusk_wallet_core::{self as wallet};
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::{Result, Rusk};
use rusk_abi::TRANSFER_CONTRACT;
use tempfile::tempdir;
use tracing::info;

use crate::common::logger;
use crate::common::state::{generator_procedure, new_state};
use crate::common::wallet::{TestProverClient, TestStateClient, TestStore};

const BLOCK_HEIGHT: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 1_000_000_000_000;
const INITIAL_BALANCE: u64 = 10_000_000_000;

const GAS_LIMIT_0: u64 = 7_000_000;
const GAS_LIMIT_1: u64 = 200_000_000;

// Creates the Rusk initial state for the tests below
fn initial_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot = toml::from_str(include_str!("../config/gas-behavior.toml"))
        .expect("Cannot deserialize config");

    new_state(dir, &snapshot)
}

const SENDER_INDEX_0: u64 = 0;
const SENDER_INDEX_1: u64 = 1;

fn make_transactions(
    rusk: &Rusk,
    wallet: &wallet::Wallet<TestStore, TestStateClient, TestProverClient>,
) {
    // We will refund the transaction to ourselves.
    let refund_0 = wallet
        .public_spend_key(SENDER_INDEX_0)
        .expect("Getting a public spend key should succeed");

    let initial_balance_0 = wallet
        .get_balance(SENDER_INDEX_0)
        .expect("Getting initial balance should succeed")
        .value;

    // We will refund the transaction to ourselves.
    let refund_1 = wallet
        .public_spend_key(SENDER_INDEX_1)
        .expect("Getting a public spend key should succeed");

    let initial_balance_1 = wallet
        .get_balance(SENDER_INDEX_1)
        .expect("Getting initial balance should succeed")
        .value;

    assert_eq!(
        initial_balance_0, INITIAL_BALANCE,
        "The sender should have the given initial balance"
    );
    assert_eq!(
        initial_balance_1, INITIAL_BALANCE,
        "The sender should have the given initial balance"
    );

    let mut rng = StdRng::seed_from_u64(0xdead);

    // The first transaction will be a `wallet.execute` to the transfer
    // contract, querying for the root of the tree. This will be given too
    // little gas to execute correctly and error, consuming all gas provided.
    let tx_0 = wallet
        .execute(
            &mut rng,
            TRANSFER_CONTRACT.to_bytes().into(),
            String::from("root"),
            (),
            SENDER_INDEX_0,
            &refund_0,
            GAS_LIMIT_0,
            1,
        )
        .expect("Making the transaction should succeed");

    // The second transaction will also be a `wallet.execute` to the transfer
    // contract, querying for the root of the tree. This will be tested for
    // gas cost.
    let tx_1 = wallet
        .execute(
            &mut rng,
            TRANSFER_CONTRACT.to_bytes().into(),
            String::from("root"),
            (),
            SENDER_INDEX_1,
            &refund_1,
            GAS_LIMIT_1,
            1,
        )
        .expect("Making the transaction should succeed");

    let spent_transactions = generator_procedure(
        rusk,
        &[tx_0, tx_1],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![],
        None,
    )
    .expect("generator procedure should succeed");

    let mut spent_transactions = spent_transactions.into_iter();
    let tx_0 = spent_transactions
        .next()
        .expect("There should be two spent transactions");
    let tx_1 = spent_transactions
        .next()
        .expect("There should be two spent transactions");

    assert!(tx_0.err.is_some(), "The first transaction should error");
    assert!(tx_1.err.is_none(), "The second transaction should succeed");

    assert_eq!(
        tx_0.gas_spent, GAS_LIMIT_0,
        "Erroring transaction should consume all gas"
    );
    assert!(
        tx_1.gas_spent < GAS_LIMIT_1,
        "Successful transaction should consume less than provided"
    );
}

#[tokio::test(flavor = "multi_thread")]
pub async fn erroring_tx_charged_full() -> Result<()> {
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
        TestProverClient::default(),
    );

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    make_transactions(&rusk, &wallet);

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
