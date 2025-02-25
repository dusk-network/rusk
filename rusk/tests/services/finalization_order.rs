// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use rand::prelude::*;
use rand::rngs::StdRng;

use dusk_core::transfer::Transaction;
use rusk::{node::RuskVmConfig, Result, Rusk};
use tempfile::tempdir;
use tokio::sync::broadcast;
use tracing::info;

use crate::common::logger;
use crate::common::state::{
    generator_procedure2, new_state, DEFAULT_MIN_GAS_LIMIT,
};

use crate::common::wallet::{
    test_wallet as wallet, DummyCacheItem, TestStateClient, TestStore, Wallet,
};

const BLOCK_GAS_LIMIT: u64 = 24_000_000;
const GAS_LIMIT: u64 = 12_000_000; // Lowest value for a transfer
const INITIAL_BALANCE: u64 = 10_000_000_000;
const CHAIN_ID: u8 = 0xFA;

// Creates the Rusk initial state for the tests below
fn initial_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot =
        toml::from_str(include_str!("../config/multi_transfer.toml"))
            .expect("Cannot deserialize config");
    let vm_config = RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);

    new_state(dir, &snapshot, vm_config)
}

fn previous_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let vm_config = RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);
    let (sender, _) = broadcast::channel(10);
    let rusk = Rusk::new(
        dir,
        CHAIN_ID,
        vm_config,
        DEFAULT_MIN_GAS_LIMIT,
        u64::MAX,
        sender,
    )
    .expect("Instantiating rusk should succeed");
    Ok(rusk)
}

// creates a separate block for each transaction
fn submit_blocks(rusk: &Rusk, txs: &[Transaction]) -> Vec<[u8; 32]> {
    let mut roots = vec![];

    let base_root = rusk.state_root();
    roots.push(base_root);

    let mut height = 0u64;
    for tx in txs {
        let block_txs = vec![tx.clone()];
        let (_, _root) = generator_procedure2(
            &rusk,
            &block_txs,
            height,
            BLOCK_GAS_LIMIT,
            vec![],
            None,
            None,
        )
        .expect("block to be created");
        let root = rusk.state_root();
        roots.push(root);
        height += 1;
    }

    roots
}

///
/// Prepares 3 transactions transferring amount from
/// indices 0, 1, 2 of the wallet to index 3
/// Also returns a wallet so that further balance verification is possible.
fn prepare_transactions(
    wallet: &Wallet<TestStore, TestStateClient>,
    amount: u64,
) -> Vec<Transaction> {
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

    txs
}

fn prepare_commits(
    rusk: &Rusk,
    cache: Arc<RwLock<HashMap<Vec<u8>, DummyCacheItem>>>,
    amount: u64,
) -> Result<([u8; 32], [u8; 32], [u8; 32])> {
    logger();

    let wallet = new_wallet(&rusk, cache.clone());

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    let txs = prepare_transactions(&wallet, amount);

    let roots = submit_blocks(&rusk, &txs);

    println!(
        "roots={:?}",
        roots.iter().map(|r| hex::encode(r)).collect::<Vec<_>>()
    );

    Ok((
        *roots.get(1).unwrap(),
        *roots.get(2).unwrap(),
        *roots.get(3).unwrap(),
    ))
}

fn new_wallet(
    rusk: &Rusk,
    cache: Arc<RwLock<HashMap<Vec<u8>, DummyCacheItem>>>,
) -> Wallet<TestStore, TestStateClient> {
    wallet::Wallet::new(
        TestStore,
        TestStateClient {
            rusk: rusk.clone(),
            cache,
        },
    )
}

#[tokio::test(flavor = "multi_thread")]
pub async fn finalization_order_correct() -> Result<()> {
    // let tmp = tempdir().expect("Should be able to create temporary directory");
    let tmp = Path::new("/Users/miloszm/.dusk/rusk/state");
    let cache = Arc::new(RwLock::new(HashMap::new()));
    let amount = 1000u64;
    let rusk = initial_state(tmp)?;
    let (root_a, root_b, root_c) =
        prepare_commits(&rusk, cache.clone(), amount)?;
    rusk.finalize_state(root_c, vec![root_a, root_b])
        .expect("finalization should work");
    // let rusk = previous_state(&tmp)?;
    // let wallet = new_wallet(&rusk, cache.clone());
    // assert_eq!(
    //     wallet
    //         .get_balance(3)
    //         .expect("Failed to get the balance")
    //         .value,
    //     3 * amount, // NOTE: 3 * amount is correct
    //     "Wrong resulting balance for the receiver"
    // );
    Ok(())
}

// #[tokio::test(flavor = "multi_thread")]
// pub async fn finalization_order_incorrect() -> Result<()> {
//     let tmp = tempdir().expect("Should be able to create temporary directory");
//     let cache = Arc::new(RwLock::new(HashMap::new()));
//     let amount = 1000u64;
//     let rusk = initial_state(tmp.as_ref())?;
//     let (root_a, root_b, root_c) =
//         prepare_commits(&rusk, cache.clone(), amount)?;
//     rusk.finalize_state(root_a, vec![root_b, root_c])
//         .expect("finalization should work"); // good - problem caught
//     let rusk = previous_state(&tmp)?;
//     let wallet = new_wallet(&rusk, cache.clone());
//     assert_eq!(
//         wallet
//             .get_balance(3)
//             .expect("Failed to get the balance")
//             .value,
//         1 * amount, // NOTE: 1 * amount instead of 3 * amount
//         "Wrong resulting balance for the receiver"
//     );
//     Ok(())
// }
