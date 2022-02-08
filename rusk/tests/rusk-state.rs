// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod common;
use crate::common::*;
use dusk_pki::SecretSpendKey;
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::{Result, Rusk, RuskState};

use microkelvin::{BackendCtor, DiskBackend};

use tracing::info;

use phoenix_core::Note;

const BLOCK_HEIGHT: u64 = 1;
const INITIAL_BALANCE: u64 = 10_000_000_000;

// Function used to creates a temporary diskbackend for Rusk
fn testbackend() -> BackendCtor<DiskBackend> {
    BackendCtor::new(DiskBackend::ephemeral)
}

// Creates the Rusk initial state for the tests below
fn initial_state() -> Result<Rusk> {
    let state_id = rusk_recovery_tools::state::deploy(&testbackend())?;

    let rusk = Rusk::builder(testbackend).id(state_id).build()?;

    let mut state = rusk.state()?;
    let transfer = state.transfer_contract()?;

    assert!(
        transfer.get_note(0)?.is_some(),
        "Expect to have one note at the genesis state",
    );

    assert!(
        transfer.get_note(1)?.is_none(),
        "Expect to have ONLY one note at the genesis state",
    );

    state.accept();
    state.finalize();

    Ok(rusk)
}

fn push_note(rusk_state: &mut RuskState) -> Result<()> {
    info!("Generating a note");
    let mut rng = StdRng::seed_from_u64(0xdead);

    let ssk = SecretSpendKey::random(&mut rng);
    let psk = ssk.public_spend_key();

    let note = Note::transparent(&mut rng, &psk, INITIAL_BALANCE);

    let mut transfer = rusk_state.transfer_contract()?;

    transfer.push_note(BLOCK_HEIGHT, note)?;

    transfer.update_root()?;

    info!("Updating the new transfer contract state");
    unsafe {
        rusk_state
            .set_contract_state(&rusk_abi::transfer_contract(), &transfer)?;
    };

    Ok(())
}

pub fn rusk_state_accepted() -> Result<()> {
    // Setup the logger
    logger();

    let rusk = initial_state()?;

    let mut state = rusk.state()?;
    push_note(&mut state)?;

    let transfer = state.transfer_contract()?;

    assert!(transfer.get_note(1)?.is_some(), "Note added");

    state.accept();
    state.revert();

    let transfer = state.transfer_contract()?;
    assert!(transfer.get_note(1)?.is_none(), "Note removed");
    Ok(())
}

pub fn rusk_state_finalized() -> Result<()> {
    // Setup the logger
    logger();

    let rusk = initial_state()?;

    let mut state = rusk.state()?;
    push_note(&mut state)?;

    let transfer = state.transfer_contract()?;

    assert!(transfer.get_note(1)?.is_some(), "Note added");

    state.finalize();
    state.revert();

    assert!(transfer.get_note(1)?.is_some(), "Note still present");
    Ok(())
}

pub fn rusk_state_ephemeral() -> Result<()> {
    // Setup the logger
    logger();

    let rusk = initial_state()?;

    // The state is dropped at the end of the block, all changes that are not
    // accepted / finalized are lost
    {
        let mut state = rusk.state()?;

        push_note(&mut state)?;
        state.finalize();

        push_note(&mut state)?;
        state.accept();

        push_note(&mut state)?;

        let transfer = state.transfer_contract()?;

        assert!(transfer.get_note(1)?.is_some(), "Note added");
        assert!(transfer.get_note(2)?.is_some(), "Note added");
        assert!(transfer.get_note(3)?.is_some(), "Note added");
    }

    let mut state = rusk.state()?;
    let transfer = state.transfer_contract()?;

    assert!(transfer.get_note(1)?.is_some(), "Note still present");
    assert!(transfer.get_note(2)?.is_some(), "Note still present");
    assert!(transfer.get_note(3)?.is_none(), "Note removed");

    state.revert();
    let transfer = state.transfer_contract()?;

    assert!(transfer.get_note(1)?.is_some(), "Note still present");
    assert!(transfer.get_note(2)?.is_none(), "Note removed");
    assert!(transfer.get_note(3)?.is_none(), "Note removed");

    Ok(())
}
