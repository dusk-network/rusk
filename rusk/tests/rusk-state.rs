// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![feature(lazy_cell)]
pub mod common;

use crate::common::*;

use std::path::Path;
use std::sync::{mpsc, Arc};

use execution_core::{
    transfer::{
        phoenix::{
            Note, NoteLeaf, PublicKey as PhoenixPublicKey,
            SecretKey as PhoenixSecretKey,
        },
        TRANSFER_CONTRACT,
    },
    JubJubScalar, LUX,
};
use ff::Field;
use parking_lot::RwLockWriteGuard;
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::node::{Rusk, RuskTip};
use rusk::Result;
use rusk_abi::VM;
use tempfile::tempdir;
use tracing::info;

use crate::common::state::new_state;

const BLOCK_HEIGHT: u64 = 1;
const CHAIN_ID: u8 = 0xFA;
const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;
const INITIAL_BALANCE: u64 = 10_000_000_000;

// Creates the Rusk initial state for the tests below
fn initial_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot = toml::from_str(include_str!("./config/rusk-state.toml"))
        .expect("Cannot deserialize config");

    new_state(dir, &snapshot, BLOCK_GAS_LIMIT)
}

fn leaves_from_height(rusk: &Rusk, height: u64) -> Result<Vec<NoteLeaf>> {
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

    let sender_sk = PhoenixSecretKey::random(&mut rng);
    let sender_pk = PhoenixPublicKey::from(&sender_sk);
    let receiver_pk =
        PhoenixPublicKey::from(&PhoenixSecretKey::random(&mut rng));

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
            rusk_abi::new_session(vm, current_commit, CHAIN_ID, BLOCK_HEIGHT)
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

// This code is used to generate the transaction bytes for the phoenix
// benchmarks. To generate:
//   - uncomment the `#[tokio::test(..)]' line
//   - run the test 'generate_phoenix_txs'
//   - move the resulting "phoenix-txs" file under "benches/phoenix-txs"
#[allow(dead_code)]
// #[tokio::test(flavor = "multi_thread")]
async fn generate_phoenix_txs() -> Result<(), Box<dyn std::error::Error>> {
    use common::wallet::{TestStateClient, TestStore};
    use std::io::Write;

    common::logger();

    let tmp = tempdir()?;
    let snapshot = toml::from_str(include_str!("./config/bench.toml"))
        .expect("Cannot deserialize config");

    let rusk = new_state(&tmp, &snapshot, 100_000_000_000)?;

    let cache =
        Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()));

    let wallet =
        test_wallet::Wallet::new(TestStore, TestStateClient { rusk, cache });

    const N_ADDRESSES: usize = 100;

    const TRANSFER_VALUE: u64 = 1_000_000;
    const GAS_LIMIT: u64 = 100_000_000;

    let wallet = Arc::new(wallet);

    let mut txs_file = std::fs::File::create("phoenix-txs")?;

    for sender_index in 0..N_ADDRESSES as u8 {
        let wallet = wallet.clone();
        let mut rng = StdRng::seed_from_u64(0xdead);

        let receiver_index = (sender_index + 1) % N_ADDRESSES as u8;
        let receiver = wallet.phoenix_public_key(receiver_index).unwrap();

        let task = tokio::task::spawn_blocking(move || {
            wallet
                .phoenix_transfer(
                    &mut rng,
                    sender_index,
                    &receiver,
                    TRANSFER_VALUE,
                    GAS_LIMIT,
                    LUX,
                )
                .expect("Making a transfer TX should succeed")
        });

        let tx = task.await.expect("Joining should succeed");
        txs_file.write(hex::encode(tx.to_var_bytes()).as_bytes())?;
        txs_file.write(b"\n")?;
    }

    Ok(())
}

// This code is used to generate the transaction bytes for the moonlight
// benchmarks. To generate:
//   - uncomment the `#[tokio::test(..)]' line
//   - run the test 'generate_moonlight_txs'
//   - move the resulting "moonlight-txs" file under "benches/moonlight-txs"
#[allow(dead_code)]
// #[tokio::test(flavor = "multi_thread")]
async fn generate_moonlight_txs() -> Result<(), Box<dyn std::error::Error>> {
    use common::wallet::{TestStateClient, TestStore};
    use std::io::Write;

    common::logger();

    let tmp = tempdir()?;
    let snapshot = toml::from_str(include_str!("./config/bench.toml"))
        .expect("Cannot deserialize config");

    let rusk = new_state(&tmp, &snapshot, 100_000_000_000)?;

    let cache =
        Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()));

    let wallet =
        test_wallet::Wallet::new(TestStore, TestStateClient { rusk, cache });

    const N_ADDRESSES: usize = 100;

    const TRANSFER_VALUE: u64 = 1_000_000;
    const GAS_LIMIT: u64 = 100_000_000;

    let wallet = Arc::new(wallet);

    let mut txs_file = std::fs::File::create("moonlight-txs")?;

    for sender_index in 0..N_ADDRESSES as u8 {
        let wallet = wallet.clone();

        let receiver_index = (sender_index + 1) % N_ADDRESSES as u8;
        let receiver = wallet.account_public_key(receiver_index).unwrap();

        let task = tokio::task::spawn_blocking(move || {
            wallet
                .moonlight_transfer(
                    sender_index,
                    receiver,
                    TRANSFER_VALUE,
                    GAS_LIMIT,
                    LUX,
                )
                .expect("Making a transfer TX should succeed")
        });

        let tx = task.await.expect("Joining should succeed");
        txs_file.write(hex::encode(tx.to_var_bytes()).as_bytes())?;
        txs_file.write(b"\n")?;
    }

    Ok(())
}
