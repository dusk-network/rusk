// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::mpsc;

use execution_core::{
    signatures::bls::PublicKey as AccountPublicKey,
    transfer::{
        data::TransactionData,
        moonlight::AccountData,
        phoenix::{
            Note, NoteLeaf, NoteOpening, NoteTreeItem, PublicKey, SecretKey,
            Transaction as PhoenixTransaction, ViewKey,
        },
        Transaction, TRANSFER_CONTRACT,
    },
    BlsScalar, ContractError, ContractId,
};
use rusk_abi::{CallReceipt, PiecrustError, Session};
use rusk_prover::LocalProver;

use rand::rngs::StdRng;

const GAS_LIMIT: u64 = 0x10_000_000;

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

pub fn leaves_from_pos(
    session: &mut Session,
    pos: u64,
) -> Result<Vec<NoteLeaf>, PiecrustError> {
    let (feeder, receiver) = mpsc::channel();

    session.feeder_call::<_, ()>(
        TRANSFER_CONTRACT,
        "leaves_from_pos",
        &pos,
        GAS_LIMIT,
        feeder,
    )?;

    Ok(receiver
        .iter()
        .map(|bytes| rkyv::from_bytes(&bytes).expect("Should return leaves"))
        .collect())
}

pub fn num_notes(session: &mut Session) -> Result<u64, PiecrustError> {
    session
        .call(TRANSFER_CONTRACT, "num_notes", &(), u64::MAX)
        .map(|r| r.data)
}

pub fn update_root(session: &mut Session) -> Result<(), PiecrustError> {
    session
        .call(TRANSFER_CONTRACT, "update_root", &(), GAS_LIMIT)
        .map(|r| r.data)
}

pub fn root(session: &mut Session) -> Result<BlsScalar, PiecrustError> {
    session
        .call(TRANSFER_CONTRACT, "root", &(), GAS_LIMIT)
        .map(|r| r.data)
}

pub fn account(
    session: &mut Session,
    pk: &AccountPublicKey,
) -> Result<AccountData, PiecrustError> {
    session
        .call(TRANSFER_CONTRACT, "account", pk, GAS_LIMIT)
        .map(|r| r.data)
}

pub fn contract_balance(
    session: &mut Session,
    contract: ContractId,
) -> Result<u64, PiecrustError> {
    session
        .call(TRANSFER_CONTRACT, "contract_balance", &contract, GAS_LIMIT)
        .map(|r| r.data)
}

pub fn opening(
    session: &mut Session,
    pos: u64,
) -> Result<Option<NoteOpening>, PiecrustError> {
    session
        .call(TRANSFER_CONTRACT, "opening", &pos, GAS_LIMIT)
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

pub fn owned_notes_value<'a, I: IntoIterator<Item = &'a Note>>(
    vk: ViewKey,
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

/// Returns vector of notes owned by a given view key.
pub fn filter_notes_owned_by<I: IntoIterator<Item = Note>>(
    vk: ViewKey,
    iter: I,
) -> Vec<Note> {
    iter.into_iter()
        .filter(|note| vk.owns(note.stealth_address()))
        .collect()
}

/// Generate a TxCircuit given the sender secret-key, receiver public-key, the
/// input note positions in the transaction tree and the new output-notes.
pub fn create_phoenix_transaction<const I: usize>(
    rng: &mut StdRng,
    session: &mut Session,
    sender_sk: &SecretKey,
    change_pk: &PublicKey,
    receiver_pk: &PublicKey,
    gas_limit: u64,
    gas_price: u64,
    input_pos: [u64; I],
    transfer_value: u64,
    obfuscated_transaction: bool,
    deposit: u64,
    data: Option<impl Into<TransactionData>>,
) -> PhoenixTransaction {
    // Get the root of the tree of phoenix-notes.
    let root = root(session).expect("Getting the anchor should be successful");

    // Get input notes and their openings
    let mut inputs = Vec::with_capacity(I);
    for pos in input_pos {
        // fetch the note and opening for the given position
        let leaves = leaves_from_pos(session, pos)
            .expect("Getting leaves in the given range should succeed");
        assert!(
            leaves.len() > 0,
            "There should be a note at the given position"
        );
        let note = &leaves[0].note;
        let opening = opening(session, pos)
            .expect(
                "Querying the opening for the given position should succeed",
            )
            .expect("An opening should exist for a note in the tree");

        // sanity check of the merkle opening
        assert!(opening.verify(NoteTreeItem::new(note.hash(), ())));

        inputs.push((note.clone(), opening));
    }

    let chain_id =
        chain_id(session).expect("Getting the chain ID should succeed");

    PhoenixTransaction::new::<StdRng, LocalProver>(
        rng,
        sender_sk,
        change_pk,
        receiver_pk,
        inputs,
        root,
        transfer_value,
        obfuscated_transaction,
        deposit,
        gas_limit,
        gas_price,
        chain_id,
        data.map(Into::into),
    )
    .expect("creating the creation shouldn't fail")
}
