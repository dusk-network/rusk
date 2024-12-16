// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::mpsc;

use execution_core::transfer::data::TransactionData;
use execution_core::transfer::phoenix::{
    Note, NoteLeaf, NoteOpening, NoteTreeItem, PublicKey as PhoenixPublicKey,
    SecretKey as PhoenixSecretKey, Transaction as PhoenixTransaction,
    ViewKey as PhoenixViewKey,
};
use execution_core::transfer::{Transaction, TRANSFER_CONTRACT};
use execution_core::{BlsScalar, LUX};
use rand::rngs::StdRng;
use rusk_abi::{PiecrustError, Session};
use rusk_prover::LocalProver;

pub const GAS_LIMIT: u64 = 0x100_000_000;
pub const GAS_PRICE: u64 = LUX;

pub fn leaves_from_height(
    session: &mut Session,
    height: u64,
) -> Result<Vec<NoteLeaf>, PiecrustError> {
    let (feeder, receiver) = mpsc::channel();

    session.feeder_call::<_, ()>(
        TRANSFER_CONTRACT,
        "leaves_from_height",
        &height,
        u64::MAX,
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

pub fn filter_notes_owned_by<I: IntoIterator<Item = Note>>(
    vk: PhoenixViewKey,
    iter: I,
) -> Vec<Note> {
    iter.into_iter()
        .filter(|note| vk.owns(note.stealth_address()))
        .collect()
}

/// Generate a TxCircuit given the sender secret-key, receiver public-key, the
/// input note positions in the transaction tree and the new output-notes.
pub fn create_transaction<const I: usize>(
    rng: &mut StdRng,
    session: &mut Session,
    sender_sk: &PhoenixSecretKey,
    refund_pk: &PhoenixPublicKey,
    gas_limit: u64,
    gas_price: u64,
    input_pos: [u64; I],
    deposit: u64,
    data: Option<impl Into<TransactionData>>,
) -> Transaction {
    // in stake transactions the sender, receiver and refund keys are the same
    let receiver_pk = refund_pk;

    // in stake transactions there is no transfer value
    let transfer_value = 0;

    // in stake transactions the transfer-note is transparent
    let obfuscate_transfer_note = false;

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

    PhoenixTransaction::new(
        rng,
        sender_sk,
        refund_pk,
        receiver_pk,
        inputs,
        root,
        transfer_value,
        obfuscate_transfer_note,
        deposit,
        gas_limit,
        gas_price,
        chain_id,
        data.map(Into::into),
        &LocalProver,
    )
    .expect("creating the creation shouldn't fail")
    .into()
}
