// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::mpsc;

use poseidon_merkle::Opening as PoseidonOpening;
use rand::rngs::StdRng;

use execution_core::{
    transfer::{
        contract_exec::ContractExec,
        phoenix::{
            Note, PublicKey as PhoenixPublicKey, SecretKey as PhoenixSecretKey,
            Transaction as PhoenixTransaction, TreeLeaf,
            ViewKey as PhoenixViewKey, NOTES_TREE_DEPTH,
        },
        Transaction, TRANSFER_CONTRACT,
    },
    BlsScalar, ContractError,
};
use rusk_abi::{CallReceipt, PiecrustError, Session};
use rusk_prover::LocalProver;

const POINT_LIMIT: u64 = 0x100000000;

pub fn leaves_from_height(
    session: &mut Session,
    height: u64,
) -> Result<Vec<TreeLeaf>, PiecrustError> {
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
) -> Result<Vec<TreeLeaf>, PiecrustError> {
    let (feeder, receiver) = mpsc::channel();

    session.feeder_call::<_, ()>(
        TRANSFER_CONTRACT,
        "leaves_from_pos",
        &pos,
        POINT_LIMIT,
        feeder,
    )?;

    Ok(receiver
        .iter()
        .map(|bytes| rkyv::from_bytes(&bytes).expect("Should return leaves"))
        .collect())
}

pub fn update_root(session: &mut Session) -> Result<(), PiecrustError> {
    session
        .call(TRANSFER_CONTRACT, "update_root", &(), POINT_LIMIT)
        .map(|r| r.data)
}

pub fn root(session: &mut Session) -> Result<BlsScalar, PiecrustError> {
    session
        .call(TRANSFER_CONTRACT, "root", &(), POINT_LIMIT)
        .map(|r| r.data)
}

pub fn opening(
    session: &mut Session,
    pos: u64,
) -> Result<Option<PoseidonOpening<(), NOTES_TREE_DEPTH>>, PiecrustError> {
    session
        .call(TRANSFER_CONTRACT, "opening", &pos, POINT_LIMIT)
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

/// Executes a transaction, returning the call receipt
pub fn execute(
    session: &mut Session,
    tx: impl Into<Transaction>,
) -> Result<CallReceipt<Result<Vec<u8>, ContractError>>, PiecrustError> {
    let tx = tx.into();

    // Spend the inputs and execute the call. If this errors the transaction is
    // unspendable.
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

    // Refund the appropriate amount to the transaction. This call is guaranteed
    // to never error. If it does, then a programming error has occurred. As
    // such, the call to `Result::expect` is warranted.
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

/// Generate a TxCircuit given the sender secret-key, receiver public-key, the
/// input note positions in the transaction tree and the new output-notes.
pub fn create_transaction<const I: usize>(
    rng: &mut StdRng,
    session: &mut Session,
    sender_sk: &PhoenixSecretKey,
    change_pk: &PhoenixPublicKey,
    receiver_pk: &PhoenixPublicKey,
    gas_limit: u64,
    gas_price: u64,
    input_pos: [u64; I],
    transfer_value: u64,
    obfuscated_transaction: bool,
    deposit: u64,
    exec: Option<impl Into<ContractExec>>,
) -> Transaction {
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
        assert!(opening.verify(poseidon_merkle::Item::new(
            rusk_abi::poseidon_hash(note.hash_inputs().to_vec()),
            ()
        )));

        inputs.push((note.clone(), opening));
    }

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
        exec.map(Into::into),
    )
    .into()
}
