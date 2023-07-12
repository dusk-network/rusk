// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, LazyLock, RwLock};

use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_pki::SecretSpendKey;
use dusk_wallet_core::{
    self as wallet, Store, Transaction as PhoenixTransaction,
};
use node_data::ledger::SpentTransaction;
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::{Result, Rusk};
use tempfile::tempdir;
use tracing::info;

use crate::common::logger;
use crate::common::state::{generator_procedure, new_state};
use crate::common::wallet::{TestProverClient, TestStateClient, TestStore};

const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;
const INITIAL_BALANCE: u64 = 10_000_000_000;
const MAX_NOTES: u64 = 10;

// Creates the Rusk initial state for the tests below
fn initial_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot = toml::from_str(include_str!("../config/transfer.toml"))
        .expect("Cannot deserialize config");

    new_state(dir, &snapshot)
}

static SSK: LazyLock<SecretSpendKey> = LazyLock::new(|| {
    info!("Generating SecretSpendKey");
    TestStore.retrieve_ssk(0).expect("Should not fail in test")
});

/// Transacts between two accounts on the in the same wallet and produces a
/// block with a single transaction, checking balances are transferred
/// successfully.
fn wallet_transfer(
    rusk: &Rusk,
    wallet: &wallet::Wallet<TestStore, TestStateClient, TestProverClient>,
    amount: u64,
    block_height: u64,
    precomputed_tx: Option<PhoenixTransaction>,
) -> PhoenixTransaction {
    // Sender psk
    let psk = SSK.public_spend_key();

    // Generate a receiver psk
    let receiver = wallet
        .public_spend_key(1)
        .expect("Failed to get public spend key");

    let mut rng = StdRng::seed_from_u64(0xdead);
    let nonce = BlsScalar::random(&mut rng);

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
    let tx = precomputed_tx.unwrap_or_else(|| {
        wallet
            .transfer(
                &mut rng,
                0,
                &psk,
                &receiver,
                amount,
                1_000_000_000,
                2,
                nonce,
            )
            .expect("Failed to transfer")
    });
    info!("Tx: {}", hex::encode(tx.to_var_bytes()));

    let tx_hash_input_bytes = tx.to_hash_input_bytes();
    let tx_hash = rusk_abi::hash(tx_hash_input_bytes);

    info!("Tx ID: {}", hex::encode(tx_hash.to_bytes()));
    let txs: Vec<SpentTransaction> =
        generator_procedure(rusk, &[tx], block_height, BLOCK_GAS_LIMIT, None)
            .expect("generator procedure to succeed");
    let tx = txs.first().expect("tx to be processed");
    let gas_spent = tx.gas_spent;
    info!("Gas spent: {gas_spent}");

    generator_procedure(rusk, &vec![], block_height + 1, BLOCK_GAS_LIMIT, None)
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
    let fee = tx.inner.inner.fee();
    let fee = gas_spent * fee.gas_price;

    assert_eq!(
        sender_initial_balance - amount - fee,
        sender_final_balance,
        "Final sender balance mismatch"
    );
    tx.inner.inner.clone()
}

#[tokio::test(flavor = "multi_thread")]
pub async fn wallet() -> Result<()> {
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
        TestProverClient::default(),
    );

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    let tx = wallet_transfer(&rusk, &wallet, 1_000, 2, None);

    // Check the state's root is changed from the original one
    let new_root = rusk.state_root();
    info!(
        "New root after the 1st transfer: {:?}",
        hex::encode(new_root)
    );
    assert_ne!(original_root, new_root, "Root should have changed");

    // Revert the state
    rusk.revert().expect("Reverting should succeed");
    cache.write().unwrap().clear();

    // Check the state's root is back to the original one
    info!("Root after reset: {:?}", hex::encode(rusk.state_root()));
    assert_eq!(original_root, rusk.state_root(), "Root be the same again");

    wallet_transfer(&rusk, &wallet, 1_000, 2, Some(tx));

    // Check the state's root is back to the original one
    info!(
        "New root after the 2nd transfer: {:?}",
        hex::encode(rusk.state_root())
    );
    assert_eq!(
        new_root,
        rusk.state_root(),
        "Root is the same compare to the first transfer"
    );

    // let recv = kadcast_recv.try_recv();
    // let (tx, _, h) = recv.expect("Transaction has not been locally
    // propagated"); info!("Tx Wire Message {}", hex::encode(tx));
    // assert_eq!(h, 0, "Transaction locally propagated with wrong height");

    Ok(())
}
