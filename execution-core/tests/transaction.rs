// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use execution_core::signatures::bls::{
    PublicKey as AccountPublicKey, SecretKey as AccountSecretKey,
};
use execution_core::transfer::data::{
    ContractCall, TransactionData, MAX_MEMO_SIZE,
};
use execution_core::transfer::phoenix::{
    Note, NoteOpening, NoteTreeItem, NotesTree, Prove,
    PublicKey as PhoenixPublicKey, SecretKey as PhoenixSecretKey, TxCircuitVec,
};
use execution_core::transfer::Transaction;
use execution_core::{Error, JubJubScalar};
use ff::Field;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

const CHAIN_ID: u8 = 0xFA;
const NOTE_VALUE: u64 = 42;
const GAS_LIMIT: u64 = 1;
const GAS_PRICE: u64 = 1;

struct NoProver;

// Since we don't want to test the circuit itself, we don't generate the proof
// in these tests
impl Prove for NoProver {
    fn prove(&self, tx_circuit_vec_bytes: &[u8]) -> Result<Vec<u8>, Error> {
        Ok(TxCircuitVec::from_slice(tx_circuit_vec_bytes)
            .expect("serialization should be ok")
            .to_var_bytes()
            .to_vec())
    }
}

fn new_phoenix_tx<const I: usize>(
    transfer_value: u64,
    deposit: u64,
    data: Option<impl Into<TransactionData>>,
) -> Result<Transaction, Error> {
    let mut rng = StdRng::seed_from_u64(42);

    // generate random keys
    let sender_sk = PhoenixSecretKey::random(&mut rng);
    let sender_pk = PhoenixPublicKey::from(&sender_sk);
    let refund_pk = &sender_pk;

    let receiver_pk =
        PhoenixPublicKey::from(&PhoenixSecretKey::random(&mut rng));

    // create the input notes owned by the sender and push them to the tree
    let mut notes_tree = NotesTree::new();
    let input_notes: Vec<Note> = (0..I)
        .map(|i| {
            // generate the note and set its position to `i`
            let value_blinder = JubJubScalar::random(&mut rng);
            let sender_blinder = [
                JubJubScalar::random(&mut rng),
                JubJubScalar::random(&mut rng),
            ];
            let mut note = Note::obfuscated(
                &mut rng,
                &sender_pk,
                &sender_pk,
                NOTE_VALUE,
                value_blinder,
                sender_blinder,
            );
            note.set_pos(i as u64);

            // insert the note hash at position `i`
            notes_tree.insert(
                *note.pos(),
                NoteTreeItem {
                    hash: note.hash(),
                    data: (),
                },
            );

            note
        })
        .collect();

    // after all notes have been pushed, we can get their openings and the tree
    // root
    let inputs: Vec<(Note, NoteOpening)> = input_notes
        .into_iter()
        .enumerate()
        .map(|(i, note)| {
            let opening = notes_tree
                .opening(i as u64)
                .expect("there should be a note at the given position");
            (note, opening)
        })
        .collect();
    let root = notes_tree.root().hash;

    // generate the phoenix-transaction
    Transaction::phoenix(
        &mut rng,
        &sender_sk,
        refund_pk,
        &receiver_pk,
        inputs,
        root,
        transfer_value,
        false,
        deposit,
        GAS_LIMIT,
        GAS_PRICE,
        CHAIN_ID,
        data,
        &NoProver,
    )
}

// Creating a valid transaction with 1 input-note works.
#[test]
fn phoenix_1_2() {
    const I: usize = 1;

    let transfer_value = NOTE_VALUE - GAS_LIMIT * GAS_PRICE;
    let deposit = 0;
    let data: Option<TransactionData> = None;

    assert!(new_phoenix_tx::<I>(transfer_value, deposit, data).is_ok());
}

// Creating a valid transaction with 2 input-notes works.
#[test]
fn phoenix_2_2() {
    const I: usize = 2;

    let transfer_value = NOTE_VALUE - GAS_LIMIT * GAS_PRICE;
    let deposit = NOTE_VALUE;
    let data = Some(
        ContractCall::new([0u8; 32], "some fn name", &())
            .expect("Contract-call should generate"),
    );

    assert!(new_phoenix_tx::<I>(transfer_value, deposit, data).is_ok());
}

// Creating a valid transaction with 3 input-notes works.
#[test]
fn phoenix_3_2() {
    const I: usize = 3;

    let transfer_value = 3 * NOTE_VALUE - GAS_LIMIT * GAS_PRICE;
    let deposit = 0;
    let data = Some(String::from("Some memo"));

    assert!(new_phoenix_tx::<I>(transfer_value, deposit, data).is_ok());
}

// Creating a valid transaction with 4 input-notes works.
#[test]
fn phoenix_4_2() {
    const I: usize = 4;

    let transfer_value = 3 * NOTE_VALUE;
    let deposit = 0;
    let data: Option<TransactionData> = None;

    assert!(new_phoenix_tx::<I>(transfer_value, deposit, data).is_ok());
}

// Creating transaction where the input doesn't cover the transaction costs
// fails.
#[test]
fn phoenix_insufficient_balance() {
    const I: usize = 4;

    let transfer_value = 3 * NOTE_VALUE;
    let deposit = NOTE_VALUE;
    let data: Option<TransactionData> = None;

    assert_eq!(
        new_phoenix_tx::<I>(transfer_value, deposit, data).unwrap_err(),
        Error::InsufficientBalance
    );
}

// Creating transaction without any input notes fails.
#[test]
fn phoenix_no_inputs() {
    const I: usize = 0;

    let transfer_value = 0;
    let deposit = 0;
    let data: Option<TransactionData> = None;

    assert_eq!(
        new_phoenix_tx::<I>(transfer_value, deposit, data).unwrap_err(),
        Error::InsufficientBalance
    );
}

// Proof creation fails when one of the input-notes isn't owned by the sender.
#[test]
fn phoenix_ownership() {
    let mut rng = StdRng::seed_from_u64(42);

    // generate random keys
    let sender_sk = PhoenixSecretKey::random(&mut rng);
    let sender_pk = PhoenixPublicKey::from(&sender_sk);
    let refund_pk = &sender_pk;

    let receiver_sk = PhoenixSecretKey::random(&mut rng);
    let receiver_pk = PhoenixPublicKey::from(&receiver_sk);

    // create the input note owned by the receiver and push it to the tree
    let mut notes_tree = NotesTree::new();
    let value_blinder = JubJubScalar::random(&mut rng);
    let sender_blinder = [
        JubJubScalar::random(&mut rng),
        JubJubScalar::random(&mut rng),
    ];
    // create a note that is owned by the receiver
    let mut note = Note::obfuscated(
        &mut rng,
        &sender_pk,
        &receiver_pk,
        NOTE_VALUE,
        value_blinder,
        sender_blinder,
    );
    note.set_pos(0);

    // insert the note hash at position `0`
    notes_tree.insert(
        *note.pos(),
        NoteTreeItem {
            hash: note.hash(),
            data: (),
        },
    );

    // after the note has been pushed, we can get the opening and the tree root
    let opening = notes_tree
        .opening(0)
        .expect("there should be a note at the given position");
    let inputs = vec![(note, opening)];
    let root = notes_tree.root().hash;

    let transfer_value = NOTE_VALUE - 10;
    let deposit = 0;
    let data: Option<TransactionData> = None;

    // transaction creation should fail because the circuit doesn't validate
    assert_eq!(
        Transaction::phoenix(
            &mut rng,
            &sender_sk,
            refund_pk,
            &receiver_pk,
            inputs,
            root,
            transfer_value,
            false,
            deposit,
            GAS_LIMIT,
            GAS_PRICE,
            CHAIN_ID,
            data,
            &NoProver,
        )
        .unwrap_err(),
        Error::PhoenixOwnership
    );
}

// Transaction creation fails when memo is too large.
#[test]
fn phoenix_memo_too_large() {
    const I: usize = 1;
    const MEMO_SIZE: usize = MAX_MEMO_SIZE + 1;

    let transfer_value = 0;
    let deposit = 0;
    let data = Some(vec![1; MEMO_SIZE]);

    assert_eq!(
        new_phoenix_tx::<I>(transfer_value, deposit, data).unwrap_err(),
        Error::MemoTooLarge(MEMO_SIZE)
    );
}

fn new_moonlight_tx(
    data: Option<impl Into<TransactionData>>,
) -> Result<Transaction, Error> {
    let mut rng = StdRng::seed_from_u64(42);

    // generate random keys
    let sender_sk = AccountSecretKey::random(&mut rng);
    let receiver_pk =
        Some(AccountPublicKey::from(&AccountSecretKey::random(&mut rng)));

    // generate random transaction values
    let transfer_value: u64 = rng.gen();
    let deposit: u64 = rng.gen();
    let nonce: u64 = rng.gen();

    Transaction::moonlight(
        &sender_sk,
        receiver_pk,
        transfer_value,
        deposit,
        GAS_LIMIT,
        GAS_PRICE,
        nonce,
        CHAIN_ID,
        data,
    )
}

// Creating a valid transaction works.
#[test]
fn moonlight() {
    let data: Option<TransactionData> = None;

    assert!(new_moonlight_tx(data).is_ok());
}

// Transaction creation fails when memo is too large.
#[test]
fn moonlight_memo_too_large() {
    const MEMO_SIZE: usize = MAX_MEMO_SIZE + 1;
    let data = Some(vec![1; MEMO_SIZE]);

    assert_eq!(
        new_moonlight_tx(data).unwrap_err(),
        Error::MemoTooLarge(MEMO_SIZE)
    );
}
