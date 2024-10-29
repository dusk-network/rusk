// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::mpsc;

use execution_core::{
    signatures::bls::PublicKey as AccountPublicKey,
    transfer::{
        moonlight::AccountData,
        phoenix::{Note, NoteLeaf, ViewKey as PhoenixViewKey},
        Transaction, TRANSFER_CONTRACT,
    },
    ContractError, ContractId,
};
use rusk_abi::{CallReceipt, PiecrustError, Session};

const GAS_LIMIT: u64 = 0x10_000_000;

pub fn contract_balance(
    session: &mut Session,
    contract: ContractId,
) -> Result<u64, PiecrustError> {
    session
        .call(TRANSFER_CONTRACT, "contract_balance", &contract, GAS_LIMIT)
        .map(|r| r.data)
}

pub fn chain_id(session: &mut Session) -> Result<u8, PiecrustError> {
    session
        .call(TRANSFER_CONTRACT, "chain_id", &(), GAS_LIMIT)
        .map(|r| r.data)
}

/// Executes a transaction.
/// Returns result containing gas spent.
pub fn execute(
    session: &mut Session,
    tx: impl Into<Transaction>,
) -> Result<CallReceipt<Result<Vec<u8>, ContractError>>, PiecrustError> {
    let tx = tx.into();

    let mut receipt = session.call::<_, Result<Vec<u8>, ContractError>>(
        TRANSFER_CONTRACT,
        "spend_and_execute",
        &tx,
        tx.gas_limit(),
    )?;

    // Ensure all gas is consumed if there's an error in the contract call
    if receipt.data.is_err() {
        receipt.gas_spent = receipt.gas_limit;
    }

    let refund_receipt = session
        .call::<_, ()>(
            TRANSFER_CONTRACT,
            "refund",
            &receipt.gas_spent,
            u64::MAX,
        )
        .expect("Refunding must succeed");

    receipt.events.extend(refund_receipt.events);

    Ok(receipt)
}

// moonlight helper functions

pub fn account(
    session: &mut Session,
    pk: &AccountPublicKey,
) -> Result<AccountData, PiecrustError> {
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
) -> Result<Vec<NoteLeaf>, PiecrustError> {
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

pub fn update_root(session: &mut Session) -> Result<(), PiecrustError> {
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
