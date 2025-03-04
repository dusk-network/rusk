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

const BLOCK_GAS_LIMIT: u64 = 48_000_000;
const GAS_LIMIT: u64 = 12_000_000; // Lowest value for a transfer
const INITIAL_BALANCE: u64 = 10_000_000_000;
const CHAIN_ID: u8 = 0xFA;
const TXS_PER_BLOCK: u8 = 1;
const RECEIVER_INDEX: u8 = 4 * TXS_PER_BLOCK;

// Creates the Rusk initial state for the tests below
fn initial_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot =
        toml::from_str(include_str!("../config/finalization.toml"))
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

// creates a separate block for each 4 transactions
fn submit_blocks(rusk: &Rusk, txs: &[Transaction]) -> Vec<[u8; 32]> {
    let mut roots = vec![];
    println!("txs len = {}", txs.len());

    let base_root = rusk.state_root();
    roots.push(base_root);

    let mut height = 0u64;
    for i in 0..((RECEIVER_INDEX/TXS_PER_BLOCK) as usize) {
        let block_txs = &txs[i*TXS_PER_BLOCK as usize..(i+1)*TXS_PER_BLOCK as usize];
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

fn submit_block(rusk: &Rusk, height: u64, txs: &[Transaction]) -> [u8; 32] {
    let (_, _root) = generator_procedure2(
        &rusk,
        txs,
        height,
        BLOCK_GAS_LIMIT,
        vec![],
        None,
        None,
    )
    .expect("block to be created");
    rusk.state_root()
}

fn submit_empty_blocks(rusk: &Rusk, blocks: u64) -> Vec<[u8; 32]> {
    let mut roots = vec![];

    let base_root = rusk.state_root();
    roots.push(base_root);

    for height in 0..blocks {
        let (_, root) = generator_procedure2(
            &rusk,
            &[],
            height,
            BLOCK_GAS_LIMIT,
            vec![],
            None,
            None,
        )
        .expect("block to be created");
        roots.push(root);
    }

    roots
}

fn submit_empty_block(rusk: &Rusk, height: u64) -> [u8; 32] {
    let (_, root) = generator_procedure2(
        &rusk,
        &[],
        height,
        BLOCK_GAS_LIMIT,
        vec![],
        None,
        None,
    )
    .expect("block to be created");
    root
}

///
/// Prepares RECEIVER_INDEX transactions transferring amount from
/// indices 0, 1, 2, ..(RECEIVER_INDEX-1) of the wallet to index RECEIVER_INDEX
fn prepare_transactions(
    wallet: &Wallet<TestStore, TestStateClient>,
    amount: u64,
    check_balances: bool,
) -> Vec<Transaction> {
    let receiver = wallet
        .phoenix_public_key(RECEIVER_INDEX)
        .expect("Failed to get public key");

    let mut rng = StdRng::seed_from_u64(0xdead);

    if check_balances {
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
                .get_balance(RECEIVER_INDEX)
                .expect("Failed to get the balance")
                .value,
            0,
            "Wrong initial balance for the receiver"
        );
    }

    let mut txs = Vec::with_capacity(4);

    for i in 0..RECEIVER_INDEX {
        let tx = wallet
            .phoenix_transfer(&mut rng, i, &receiver, amount, GAS_LIMIT, 1)
            .expect("Failed to transfer");
        txs.push(tx);
    }

    txs
}

fn prepare_transactions_for_one_block(
    wallet: &Wallet<TestStore, TestStateClient>,
    amount: u64,
) -> Vec<Transaction> {
    let receiver = wallet
        .phoenix_public_key(RECEIVER_INDEX)
        .expect("Failed to get public key");

    let mut rng = StdRng::seed_from_u64(0xdead);

    let mut txs = Vec::with_capacity(TXS_PER_BLOCK as usize);

    for i in 0..TXS_PER_BLOCK {
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

    let txs = prepare_transactions(&wallet, amount, false);

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

fn do_block(
    rusk: &Rusk,
    cache: Arc<RwLock<HashMap<Vec<u8>, DummyCacheItem>>>,
    amount: u64,
    block: u64
) -> Result<[u8; 32]> {
    logger();

    let wallet = new_wallet(&rusk, cache.clone());

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    let txs = prepare_transactions_for_one_block(&wallet, amount);

    let root = submit_block(&rusk, block, &txs);

    Ok(root)
}

fn prepare_empty_commits(rusk: &Rusk) -> Result<([u8; 32], [u8; 32])> {
    logger();

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    let empty_blocks_roots = submit_empty_blocks(&rusk, 2);

    println!(
        "empty blocks roots={:?}",
        empty_blocks_roots
            .iter()
            .map(|r| hex::encode(r))
            .collect::<Vec<_>>()
    );

    Ok((
        *empty_blocks_roots.get(1).unwrap(),
        *empty_blocks_roots.get(2).unwrap(),
    ))
}

fn do_empty_block(rusk: &Rusk, height: u64) -> Result<[u8; 32]> {
    let empty_block_root = submit_empty_block(&rusk, height);
    Ok(empty_block_root)
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
    let tmp = tempdir().expect("Should be able to create temporary directory");
    let cache = Arc::new(RwLock::new(HashMap::new()));
    let amount = 1000u64;
    let rusk = initial_state(tmp.as_ref())?;
    let (root_a, root_b, root_c) =
        prepare_commits(&rusk, cache.clone(), amount)?;
    let (root_e1, root_e2) = prepare_empty_commits(&rusk)?;
    rusk.finalize_state(root_e2, vec![root_a, root_b, root_c, root_e1])
        .expect("finalization should work");
    let rusk = previous_state(&tmp)?;
    let wallet = new_wallet(&rusk, cache.clone());
    assert_eq!(
        wallet
            .get_balance(3)
            .expect("Failed to get the balance")
            .value,
        3 * amount, // NOTE: 3 * amount is correct
        "Wrong resulting balance for the receiver"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn finalization_after_empty_block() -> Result<()> {
    let tmp = tempdir().expect("Should be able to create temporary directory");
    let cache = Arc::new(RwLock::new(HashMap::new()));
    let amount = 1000u64;
    let rusk = initial_state(tmp.as_ref())?;

    let root_a = do_block(&rusk, cache.clone(), amount, 0)?;
    let root_b = do_block(&rusk, cache.clone(), amount, 1)?;
    rusk.finalize_state(root_b, vec![root_a])?;
    let root_empty = do_empty_block(&rusk, 2)?;
    rusk.finalize_state(root_empty, vec![root_b])?;
    let root_c = do_block(&rusk, cache.clone(), amount, 3)?;
    rusk.finalize_state(root_c, vec![root_empty])?;

    let rusk = previous_state(&tmp)?;
    println!("xroot={}", hex::encode(rusk.state_root()));
    let wallet = new_wallet(&rusk, cache.clone());
    assert_eq!(
        wallet
            .get_balance(RECEIVER_INDEX)
            .expect("Failed to get the balance")
            .value,
        4 * amount,
        "Wrong resulting balance for the receiver"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn finalization_order_incorrect() -> Result<()> {
    let tmp = tempdir().expect("Should be able to create temporary directory");
    let cache = Arc::new(RwLock::new(HashMap::new()));
    let amount = 1000u64;
    let rusk = initial_state(tmp.as_ref())?;
    let (root_a, root_b, root_c) =
        prepare_commits(&rusk, cache.clone(), amount)?;
    rusk.finalize_state(root_a, vec![root_b, root_c])
        .expect("finalization should work"); // good - problem caught
    let rusk = previous_state(&tmp)?;
    let wallet = new_wallet(&rusk, cache.clone());
    assert_eq!(
        wallet
            .get_balance(3)
            .expect("Failed to get the balance")
            .value,
        1 * amount, // NOTE: 1 * amount instead of 3 * amount
        "Wrong resulting balance for the receiver"
    );
    Ok(())
}
