// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use anyhow::Result;
use dusk_rusk_test::TestContext;
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::node::RuskVmConfig;
use tracing::info;

use crate::common::logger;

const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;
const INITIAL_BALANCE: u64 = 10_000_000_000;
const MAX_NOTES: u64 = 10;

/// Transacts between two accounts on the in the same wallet and produces a
/// block with a single transaction, checking balances are transferred
/// successfully.
fn wallet_transfer(tc: &TestContext, amount: u64, block_height: u64) {
    let wallet = tc.wallet();

    // Generate a receiver pk
    let receiver_pk = wallet
        .phoenix_public_key(1)
        .expect("Failed to get public key");

    let mut rng = StdRng::seed_from_u64(0xdead);

    // Store the sender initial balance
    let sender_initial_balance = wallet
        .get_balance(0)
        .expect("Failed to get the balance")
        .value;

    // Check the sender's initial balance is correct
    assert_eq!(
        sender_initial_balance,
        INITIAL_BALANCE * MAX_NOTES,
        "Wrong initial balance for the sender"
    );

    // Check the receiver initial balance is zero
    assert_eq!(
        wallet
            .get_balance(1)
            .expect("Failed to get the balance")
            .value,
        0,
        "Wrong initial balance for the receiver"
    );

    // Execute a transfer
    let tx = wallet
        .phoenix_transfer(&mut rng, 0, &receiver_pk, amount, 1_000_000_000, 2)
        .expect("Failed to transfer");
    info!("Tx: {}", hex::encode(tx.to_var_bytes()));

    let tx_hash_input_bytes = tx.to_hash_input_bytes();
    let tx_id = dusk_vm::host_queries::hash(tx_hash_input_bytes);

    info!("Tx ID: {}", hex::encode(tx_id.to_bytes()));
    let tx = tc.execute_transaction(tx, block_height, None);
    let gas_spent = tx.gas_spent;
    info!("Gas spent: {gas_spent}");

    tc.empty_block(block_height + 1)
        .expect("empty block generator procedure to succeed");

    // Check the receiver's balance is changed accordingly
    assert_eq!(
        wallet
            .get_balance(1)
            .expect("Failed to get the balance")
            .value,
        amount,
        "Wrong resulting balance for the receiver"
    );

    // Check the sender's balance is changed accordingly
    let sender_final_balance = wallet
        .get_balance(0)
        .expect("Failed to get the balance")
        .value;
    let fee = gas_spent * tx.inner.inner.gas_price();

    assert_eq!(
        sender_initial_balance - amount - fee,
        sender_final_balance,
        "Final sender balance mismatch"
    );
}

#[tokio::test(flavor = "multi_thread")]
pub async fn wallet() -> Result<()> {
    // Setup the logger
    logger();
    let state_toml = include_str!("../config/transfer.toml");
    let vm_config = RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);

    let tc = TestContext::instantiate(state_toml, vm_config).await?;

    let original_root = tc.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    wallet_transfer(&tc, 1_000, 2);

    // Check the state's root is changed from the original one
    let new_root = tc.state_root();
    info!(
        "New root after the 1st transfer: {:?}",
        hex::encode(new_root)
    );
    assert_ne!(original_root, new_root, "Root should have changed");

    // Revert the state
    tc.revert_to_base_root().expect("Reverting should succeed");

    // Check the state's root is back to the original one
    info!("Root after reset: {:?}", hex::encode(tc.state_root()));
    assert_eq!(original_root, tc.state_root(), "Root be the same again");

    wallet_transfer(&tc, 1_000, 2);

    // Check the state's root is back to the original one
    info!(
        "New root after the 2nd transfer: {:?}",
        hex::encode(tc.state_root())
    );
    assert_eq!(
        new_root,
        tc.state_root(),
        "Root is the same compare to the first transfer"
    );

    Ok(())
}
