// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![feature(lazy_cell)]
pub mod common;
use crate::common::*;

use std::path::Path;

use dusk_bls12_381::BlsScalar;
use dusk_pki::SecretSpendKey;
use parking_lot::MutexGuard;
use phoenix_core::Note;
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::{Result, Rusk, RuskInner};
use rusk_abi::TRANSFER_CONTRACT;
use tempfile::tempdir;
use tracing::info;

use crate::common::state::new_state;

const BLOCK_HEIGHT: u64 = 1;
const INITIAL_BALANCE: u64 = 10_000_000_000;

// Creates the Rusk initial state for the tests below
fn initial_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot = toml::from_str(include_str!("./config/rusk-state.toml"))
        .expect("Cannot deserialize config");

    new_state(dir, &snapshot)
}

fn push_note<'a, F, T>(rusk: &'a Rusk, after_push: F) -> T
where
    F: FnOnce(MutexGuard<'a, RuskInner>) -> T,
{
    info!("Generating a note");
    let mut rng = StdRng::seed_from_u64(0xdead);

    let ssk = SecretSpendKey::random(&mut rng);
    let psk = ssk.public_spend_key();

    let note = Note::transparent(&mut rng, &psk, INITIAL_BALANCE);

    rusk.with_inner(|mut inner| {
        let current_commit = inner.current_commit;
        let mut session =
            rusk_abi::new_session(&inner.vm, current_commit, BLOCK_HEIGHT)
                .expect("current commit should exist");

        session.set_point_limit(u64::MAX);

        let _: Note = session
            .call(TRANSFER_CONTRACT, "push_note", &note)
            .expect("Pushing note should succeed");
        let _: BlsScalar = session
            .call(TRANSFER_CONTRACT, "update_root", &())
            .expect("Updating root should succeed");

        let commit_id = session.commit().expect("Committing should succeed");
        inner.current_commit = commit_id;

        after_push(inner)
    })
}

#[test]
pub fn rusk_state_accepted() -> Result<()> {
    // Setup the logger
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp)?;

    push_note(&rusk, |_inner| {});

    let leaves = rusk.leaves_in_range(0..1)?;

    assert_eq!(
        leaves.len(),
        2,
        "There should be two notes in the state now"
    );

    rusk.revert()?;
    let leaves = rusk.leaves_in_range(0..1)?;

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

    push_note(&rusk, |mut inner| {
        inner.base_commit = inner.current_commit;
    });

    let leaves = rusk.leaves_in_range(0..1)?;

    assert_eq!(
        leaves.len(),
        2,
        "There should be two notes in the state now"
    );

    rusk.revert()?;
    let leaves = rusk.leaves_in_range(0..1)?;

    assert_eq!(
        leaves.len(),
        2,
        "The new note should still be there after reversion"
    );

    Ok(())
}
