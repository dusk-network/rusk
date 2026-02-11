// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::transfer::{
    data::{ContractCall, TransactionData},
    TRANSFER_CONTRACT,
};
use dusk_rusk_test::{Result, RuskVmConfig, TestContext};
use rand::prelude::*;
use rand::rngs::StdRng;
use tracing::info;

use crate::common::logger;
use crate::common::state::generator_procedure;

const BLOCK_HEIGHT: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 1_000_000_000_000;

const INITIAL_BALANCE: u64 = 10_000_000_000;

const GAS_LIMIT_0: u64 = 100_000_000;
const GAS_LIMIT_1: u64 = 300_000_000;
const GAS_PRICE: u64 = 1;
const DEPOSIT: u64 = 0;

const SENDER_INDEX_0: u8 = 0;
const SENDER_INDEX_1: u8 = 1;

fn make_transactions(tc: &TestContext) {
    let rusk = tc.rusk();
    let wallet = tc.wallet();
    let initial_balance_0 = wallet
        .get_balance(SENDER_INDEX_0)
        .expect("Getting initial balance should succeed")
        .value;

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
    let contract_call = ContractCall::new(TRANSFER_CONTRACT, "root");
    let tx_0 = wallet
        .phoenix_execute(
            &mut rng,
            SENDER_INDEX_0,
            GAS_LIMIT_0,
            GAS_PRICE,
            DEPOSIT,
            TransactionData::Call(contract_call.clone()),
        )
        .expect("Making the transaction should succeed");

    // The second transaction will also be a `wallet.execute` to the transfer
    // contract, querying for the root of the tree. This will be tested for
    // gas cost.
    let tx_1 = wallet
        .phoenix_execute(
            &mut rng,
            SENDER_INDEX_1,
            GAS_LIMIT_1,
            GAS_PRICE,
            DEPOSIT,
            contract_call,
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

    let state = include_str!("../config/gas-behavior.toml");
    let vm_config = RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);
    let tc = TestContext::instantiate(state, vm_config).await?;

    let original_root = tc.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    make_transactions(&tc);

    // Check the state's root is changed from the original one
    let new_root = tc.state_root();
    info!(
        "New root after the 1st transfer: {:?}",
        hex::encode(new_root)
    );
    assert_ne!(original_root, new_root, "Root should have changed");

    Ok(())
}
