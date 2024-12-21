// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::mpsc;

use dusk_core::abi::ContractId;
use dusk_core::signatures::bls::PublicKey as AccountPublicKey;
use dusk_core::transfer::moonlight::AccountData;
use dusk_core::transfer::phoenix::{Note, NoteLeaf, ViewKey as PhoenixViewKey};
use dusk_core::transfer::TRANSFER_CONTRACT;
use dusk_core::BlsScalar;
use dusk_vm::{Error as VMError, Session};

const GAS_LIMIT: u64 = 0x10_000_000;

pub fn contract_balance(
    session: &mut Session,
    contract: ContractId,
) -> Result<u64, VMError> {
    session
        .call(TRANSFER_CONTRACT, "contract_balance", &contract, GAS_LIMIT)
        .map(|r| r.data)
}

pub fn chain_id(session: &mut Session) -> Result<u8, VMError> {
    session
        .call(TRANSFER_CONTRACT, "chain_id", &(), GAS_LIMIT)
        .map(|r| r.data)
}

// moonlight helper functions

pub fn account(
    session: &mut Session,
    pk: &AccountPublicKey,
) -> Result<AccountData, VMError> {
    session
        .call(TRANSFER_CONTRACT, "account", pk, GAS_LIMIT)
        .map(|r| r.data)
}

// phoenix helper functions

pub fn new_owned_notes_value(
    session: &mut Session,
    height: u64,
    vk: PhoenixViewKey,
) -> u64 {
    let leaves = leaves_from_height(session, height)
        .expect("fetching notes from the height should work");
    let owned_notes =
        filter_notes_owned_by(vk, leaves.into_iter().map(|leaf| leaf.note));
    owned_notes_value(vk, &owned_notes)
}

pub fn owned_notes_value<'a, I: IntoIterator<Item = &'a Note>>(
    vk: PhoenixViewKey,
    notes: I,
) -> u64 {
    notes.into_iter().fold(0, |acc, note| {
        acc + if vk.owns(note.stealth_address()) {
            note.value(Some(&vk)).unwrap()
        } else {
            0
        }
    })
}

pub fn leaves_from_height(
    session: &mut Session,
    height: u64,
) -> Result<Vec<NoteLeaf>, VMError> {
    let (feeder, receiver) = mpsc::channel();

    session.feeder_call::<_, ()>(
        TRANSFER_CONTRACT,
        "leaves_from_height",
        &height,
        GAS_LIMIT,
        feeder,
    )?;

    Ok(receiver
        .iter()
        .map(|bytes| rkyv::from_bytes(&bytes).expect("Should return leaves"))
        .collect())
}

pub fn update_root(session: &mut Session) -> Result<(), VMError> {
    session
        .call(TRANSFER_CONTRACT, "update_root", &(), GAS_LIMIT)
        .map(|r| r.data)
}

/// Returns vector of notes owned by a given view key.
pub fn filter_notes_owned_by<I: IntoIterator<Item = Note>>(
    vk: PhoenixViewKey,
    iter: I,
) -> Vec<Note> {
    iter.into_iter()
        .filter(|note| vk.owns(note.stealth_address()))
        .collect()
}

pub fn existing_nullifiers(
    session: &mut Session,
    nullifiers: &Vec<BlsScalar>,
) -> Result<Vec<BlsScalar>, VMError> {
    session
        .call(
            TRANSFER_CONTRACT,
            "existing_nullifiers",
            &nullifiers.clone(),
            GAS_LIMIT,
        )
        .map(|r| r.data)
}
