// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![feature(lazy_cell)]
pub mod common;

use crate::common::*;
use std::collections::HashMap;

use std::path::Path;
use std::sync::{mpsc, Arc, RwLock};

use execution_core::{
    transfer::TreeLeaf, JubJubScalar, Note, PublicKey, SecretKey,
};
use ff::Field;
use parking_lot::RwLockWriteGuard;
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::chain::{Rusk, RuskTip};
use rusk::Result;
use rusk_abi::dusk::LUX;
use rusk_abi::{TRANSFER_CONTRACT, VM};
use tempfile::tempdir;
use test_wallet::{self as wallet};
use tokio::task;
use tracing::info;

use crate::common::state::new_state;
use crate::common::wallet::{TestProverClient, TestStateClient, TestStore};

const BLOCK_HEIGHT: u64 = 1;
const INITIAL_BALANCE: u64 = 10_000_000_000;

// Creates the Rusk initial state for the tests below
fn initial_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot = toml::from_str(include_str!("./config/rusk-state.toml"))
        .expect("Cannot deserialize config");

    new_state(dir, &snapshot)
}

fn leaves_from_height(rusk: &Rusk, height: u64) -> Result<Vec<TreeLeaf>> {
    let (sender, receiver) = mpsc::channel();
    rusk.leaves_from_height(height, sender)?;
    Ok(receiver
        .into_iter()
        .map(|bytes| rkyv::from_bytes(&bytes).unwrap())
        .collect())
}

fn push_note<'a, F, T>(rusk: &'a Rusk, after_push: F) -> T
where
    F: FnOnce(RwLockWriteGuard<'a, RuskTip>, &'a VM) -> T,
{
    info!("Generating a note");
    let mut rng = StdRng::seed_from_u64(0xdead);

    let sender_sk = SecretKey::random(&mut rng);
    let sender_pk = PublicKey::from(&sender_sk);
    let receiver_pk = PublicKey::from(&SecretKey::random(&mut rng));

    let sender_blinder = [
        JubJubScalar::random(&mut rng),
        JubJubScalar::random(&mut rng),
    ];

    let note = Note::transparent(
        &mut rng,
        &sender_pk,
        &receiver_pk,
        INITIAL_BALANCE,
        sender_blinder,
    );

    rusk.with_tip(|mut tip, vm| {
        let current_commit = tip.current;
        let mut session =
            rusk_abi::new_session(vm, current_commit, BLOCK_HEIGHT)
                .expect("current commit should exist");

        session
            .call::<_, Note>(
                TRANSFER_CONTRACT,
                "push_note",
                &(0u64, note),
                u64::MAX,
            )
            .expect("Pushing note should succeed");
        session
            .call::<_, ()>(TRANSFER_CONTRACT, "update_root", &(), u64::MAX)
            .expect("Updating root should succeed");

        let commit_id = session.commit().expect("Committing should succeed");
        tip.current = commit_id;

        after_push(tip, vm)
    })
}

#[test]
pub fn rusk_state_accepted() -> Result<()> {
    // Setup the logger
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp)?;

    push_note(&rusk, |_tip, _vm| {});

    let leaves = leaves_from_height(&rusk, 0)?;

    assert_eq!(
        leaves.len(),
        2,
        "There should be two notes in the state now"
    );

    rusk.revert_to_base_root()?;
    let leaves = leaves_from_height(&rusk, 0)?;

    assert_eq!(
        leaves.len(),
        1,
        "The new note should no longer be there after reversion"
    );

    Ok(())
}

#[test]
pub fn rusk_state_finalized() -> Result<()> {
    // Setup the logger
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp)?;

    push_note(&rusk, |mut tip, _vm| {
        tip.base = tip.current;
    });

    let leaves = leaves_from_height(&rusk, 0)?;

    assert_eq!(
        leaves.len(),
        2,
        "There should be two notes in the state now"
    );

    rusk.revert_to_base_root()?;
    let leaves = leaves_from_height(&rusk, 0)?;

    assert_eq!(
        leaves.len(),
        2,
        "The new note should still be there after reversion"
    );

    Ok(())
}

// #[tokio::test(flavor = "multi_thread")]
#[allow(dead_code)]
async fn generate_bench_txs() -> Result<(), Box<dyn std::error::Error>> {
    // Setup the logger
    logger();

    let tmp = tempdir()?;
    let snapshot = toml::from_str(include_str!("./config/bench.toml"))
        .expect("Cannot deserialize config");

    let rusk = new_state(&tmp, &snapshot)?;

    let cache = Arc::new(RwLock::new(HashMap::new()));

    let wallet = wallet::Wallet::new(
        TestStore,
        TestStateClient { rusk, cache },
        TestProverClient::default(),
    );

    // Generates some public keys for the wallet
    // for i in 0..100 {
    //     let pk = wallet.stake_public_key(i).unwrap();
    //     let pk_bytes = pk.to_bytes();
    //     let pk_string = bs58::encode(pk_bytes).into_string();
    //     println!("{pk_string}");
    // }

    const N_ADDRESSES: usize = 100;

    const TRANSFER_VALUE: u64 = 1_000_000;
    const GAS_LIMIT: u64 = 100_000_000;

    let mut tasks = Vec::with_capacity(N_ADDRESSES);
    let mut txs = Vec::with_capacity(N_ADDRESSES);

    let wallet = Arc::new(wallet);

    for sender_index in 0..N_ADDRESSES as u64 {
        let wallet = wallet.clone();
        let mut rng = StdRng::seed_from_u64(0xdead);

        let receiver_index = (sender_index + 1) % N_ADDRESSES as u64;
        let receiver = wallet.public_key(receiver_index).unwrap();

        tasks.push(task::spawn_blocking(move || {
            wallet
                .transfer(
                    &mut rng,
                    sender_index,
                    &receiver,
                    TRANSFER_VALUE,
                    GAS_LIMIT,
                    LUX,
                )
                .expect("Making a transfer TX should succeed")
        }));
    }

    for task in tasks {
        txs.push(task.await.expect("Joining should succeed"));
    }

    for tx in txs {
        let tx_bytes = tx.to_var_bytes();
        let tx_hex = hex::encode(tx_bytes);
        println!("{tx_hex}");
    }

    Ok(())
}
