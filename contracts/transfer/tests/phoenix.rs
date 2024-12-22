// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::mpsc;

pub mod common;

use dusk_bytes::Serializable;
use dusk_core::abi::ContractId;
use dusk_core::dusk;
use dusk_core::signatures::bls::{
    PublicKey as AccountPublicKey, SecretKey as AccountSecretKey,
};
use dusk_core::transfer::data::{ContractCall, TransactionData};
use dusk_core::transfer::phoenix::{
    Note, NoteLeaf, NoteOpening, NoteTreeItem, PublicKey as PhoenixPublicKey,
    SecretKey as PhoenixSecretKey, Transaction as PhoenixTransaction,
    ViewKey as PhoenixViewKey,
};
use dusk_core::transfer::withdraw::{
    Withdraw, WithdrawReceiver, WithdrawReplayToken,
};
use dusk_core::transfer::{
    ContractToAccount, ContractToContract, Transaction, TRANSFER_CONTRACT,
};
use dusk_core::{BlsScalar, JubJubScalar, LUX};
use dusk_vm::{execute, ContractData, Error as VMError, Session, VM};
use ff::Field;
use rand::rngs::StdRng;
use rand::{CryptoRng, RngCore, SeedableRng};
use rusk_prover::LocalProver;

use crate::common::utils::{
    account, chain_id, contract_balance, existing_nullifiers,
    filter_notes_owned_by, leaves_from_height, new_owned_notes_value,
    owned_notes_value, update_root,
};

const PHOENIX_GENESIS_VALUE: u64 = dusk(1_200.0);
const ALICE_GENESIS_VALUE: u64 = dusk(2_000.0);

const GAS_LIMIT: u64 = 100_000_000;
const GAS_PRICE: u64 = 1;

const ALICE_ID: ContractId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0xFA;
    ContractId::from_bytes(bytes)
};
const BOB_ID: ContractId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0xFB;
    ContractId::from_bytes(bytes)
};

const OWNER: [u8; 32] = [0; 32];
const CHAIN_ID: u8 = 0xFA;

/// Instantiate the virtual machine with the transfer contract deployed, and the
/// notes tree populated with `N` notes, each carrying `PHOENIX_GENESIS_VALUE /
/// N`, all owned by the given public key, and alice and bob contracts deployed
/// with alice contract owning `ALICE_GENESIS_VALUE`.
fn instantiate<const N: u8>(
    rng: &mut (impl RngCore + CryptoRng),
    phoenix_sk: &PhoenixSecretKey,
) -> Session {
    assert!(N != 0, "We need at least one note in the tree");

    let transfer_bytecode = include_bytes!(
        "../../../target/dusk/wasm64-unknown-unknown/release/transfer_contract.wasm"
    );
    let alice_bytecode = include_bytes!(
        "../../../target/dusk/wasm32-unknown-unknown/release/alice.wasm"
    );
    let bob_bytecode = include_bytes!(
        "../../../target/dusk/wasm32-unknown-unknown/release/bob.wasm"
    );

    let vm = &mut VM::ephemeral().expect("Creating ephemeral VM should work");

    let mut session = vm.genesis_session(CHAIN_ID);

    session
        .deploy(
            transfer_bytecode,
            ContractData::builder()
                .owner(OWNER)
                .contract_id(TRANSFER_CONTRACT),
            GAS_LIMIT,
        )
        .expect("Deploying the transfer contract should succeed");

    session
        .deploy(
            alice_bytecode,
            ContractData::builder().owner(OWNER).contract_id(ALICE_ID),
            GAS_LIMIT,
        )
        .expect("Deploying the alice contract should succeed");

    session
        .deploy(
            bob_bytecode,
            ContractData::builder()
                .owner(OWNER)
                .contract_id(BOB_ID)
                .init_arg(&1u8),
            GAS_LIMIT,
        )
        .expect("Deploying the bob contract should succeed");

    // generate the genesis notes and push them onto the tree
    let phoenix_pk = PhoenixPublicKey::from(phoenix_sk);
    for _ in 0..N {
        let value_blinder = JubJubScalar::random(&mut *rng);
        let sender_blinder = [
            JubJubScalar::random(&mut *rng),
            JubJubScalar::random(&mut *rng),
        ];

        let note = Note::obfuscated(
            rng,
            &phoenix_pk,
            &phoenix_pk,
            PHOENIX_GENESIS_VALUE / N as u64,
            value_blinder,
            sender_blinder,
        );
        // push genesis phoenix note to the contract
        session
            .call::<_, Note>(
                TRANSFER_CONTRACT,
                "push_note",
                &(0u64, note),
                GAS_LIMIT,
            )
            .expect("Pushing genesis note should succeed");
    }

    // update the root after the notes have been inserted
    update_root(&mut session).expect("Updating the root should succeed");

    // insert genesis value to alice contract
    session
        .call::<_, ()>(
            TRANSFER_CONTRACT,
            "add_contract_balance",
            &(ALICE_ID, ALICE_GENESIS_VALUE),
            GAS_LIMIT,
        )
        .expect("Inserting genesis account should succeed");

    // commit the first block, this sets the block height for all subsequent
    // operations to 1
    let base = session.commit().expect("Committing should succeed");
    // start a new session from that base-commit
    let mut session = vm
        .session(base, CHAIN_ID, 1)
        .expect("Instantiating new session should succeed");

    // check that the genesis state is correct:

    let phoenix_vk = PhoenixViewKey::from(phoenix_sk);

    // `N` leaves at genesis block
    let leaves = leaves_from_height(&mut session, 0)
        .expect("Getting leaves from the genesis block should succeed");
    assert_eq!(
        leaves.len(),
        N as usize,
        "There should be `N` notes at genesis state"
    );
    assert_eq!(
        *leaves.last().expect("note to exists").note.pos(),
        N as u64 - 1,
        "The last note should have position `N - 1`"
    );

    let total_num_notes =
        num_notes(&mut session).expect("Getting num_notes should succeed");
    assert_eq!(
        total_num_notes, N as u64,
        "The total amount of notes in the tree should be `N`"
    );

    // the genesis note is owned by the given key and has genesis value
    let ownded_notes = filter_notes_owned_by(
        phoenix_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );
    assert_eq!(
        owned_notes_value(phoenix_vk, &ownded_notes),
        PHOENIX_GENESIS_VALUE,
        "the genesis notes should hold the genesis-value"
    );

    // the alice contract is instantiated with the expected value
    let alice_balance = contract_balance(&mut session, ALICE_ID)
        .expect("Querying the contract balance should succeed");
    assert_eq!(
        alice_balance, ALICE_GENESIS_VALUE,
        "alice contract should have its genesis value"
    );

    // chain-id is as expected
    let chain_id =
        chain_id(&mut session).expect("Getting the chain ID should succeed");
    assert_eq!(chain_id, CHAIN_ID, "the chain id should be as expected");

    session
}

/// Perform a simple transfer of funds between two phoenix addresses.
#[test]
fn transfer_1_2() {
    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let phoenix_sender_sk = PhoenixSecretKey::random(rng);
    let phoenix_sender_pk = PhoenixPublicKey::from(&phoenix_sender_sk);
    let phoenix_sender_vk = PhoenixViewKey::from(&phoenix_sender_sk);

    let phoenix_change_pk = phoenix_sender_pk.clone();

    let phoenix_receiver_sk = PhoenixSecretKey::random(rng);
    let phoenix_receiver_pk = PhoenixPublicKey::from(&phoenix_receiver_sk);
    let phoenix_receiver_vk = PhoenixViewKey::from(&phoenix_receiver_sk);

    let session = &mut instantiate::<1>(rng, &phoenix_sender_sk);

    // create the transaction
    let input_note_pos = 0;
    let transfer_value = 42;
    let obfuscate_transfer_note = true;
    let deposit = 0;
    let contract_call: Option<ContractCall> = None;

    let tx = create_phoenix_transaction(
        rng,
        session,
        &phoenix_sender_sk,
        &phoenix_change_pk,
        &phoenix_receiver_pk,
        GAS_LIMIT,
        GAS_PRICE,
        [input_note_pos],
        transfer_value,
        obfuscate_transfer_note,
        deposit,
        contract_call,
    );

    let gas_spent = execute(session, &tx, 0, 0, 0)
        .expect("Executing TX should succeed")
        .gas_spent;
    update_root(session).expect("Updating the root should succeed");

    println!("TRANSFER 1-2: {} gas", gas_spent);

    // check that correct notes have been generated
    let leaves = leaves_from_pos(session, input_note_pos + 1)
        .expect("Getting the notes should succeed");
    assert_eq!(
        leaves.len(),
        3,
        "Transfer, change and refund notes should have been added to the tree"
    );
    let amount_notes =
        num_notes(session).expect("Getting num_notes should succeed");
    assert_eq!(
        amount_notes,
        leaves.last().expect("note to exists").note.pos() + 1,
        "num_notes should match position of last note + 1"
    );

    // check that the genesis note has been nullified
    let input_nullifier =
        gen_nullifiers(session, [input_note_pos], &phoenix_sender_sk);
    let existing_nullifers = existing_nullifiers(session, &input_nullifier)
        .expect("Querrying the nullifiers should work");
    assert_eq!(input_nullifier, existing_nullifers);

    // the sender's balance has decreased
    let owned_notes = filter_notes_owned_by(
        phoenix_sender_vk,
        leaves.iter().map(|leaf| leaf.note.clone()),
    );
    assert_eq!(
        owned_notes.len(),
        2,
        "Change and refund notes should be owned by the sender"
    );
    assert_eq!(
        owned_notes_value(phoenix_sender_vk, &owned_notes),
        PHOENIX_GENESIS_VALUE - gas_spent - transfer_value,
        "the sender's balance should have decreased by the spent gas and transfer value"
    );

    // the receiver's balance has increased
    let new_receiver_notes = filter_notes_owned_by(
        phoenix_receiver_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );
    assert_eq!(
        new_receiver_notes.len(),
        1,
        "The receiver should own the transfer note"
    );
    assert_eq!(
        owned_notes_value(phoenix_receiver_vk, &new_receiver_notes),
        transfer_value,
        "The receiver should own the transfer-value"
    );
}

/// Perform a simple transfer of funds between two phoenix addresses, using two
/// input notes.
#[test]
fn transfer_2_2() {
    const N: u8 = 2;
    // the genesis notes each hold the genesis value / 2, the gas costs will
    // force us to spent both input notes
    const TRANSFER_VALUE: u64 = PHOENIX_GENESIS_VALUE / N as u64;

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let phoenix_sender_sk = PhoenixSecretKey::random(rng);
    let phoenix_sender_pk = PhoenixPublicKey::from(&phoenix_sender_sk);
    let phoenix_sender_vk = PhoenixViewKey::from(&phoenix_sender_sk);

    let phoenix_change_pk = phoenix_sender_pk.clone();

    let phoenix_receiver_sk = PhoenixSecretKey::random(rng);
    let phoenix_receiver_pk = PhoenixPublicKey::from(&phoenix_receiver_sk);
    let phoenix_receiver_vk = PhoenixViewKey::from(&phoenix_receiver_sk);

    let session = &mut instantiate::<N>(rng, &phoenix_sender_sk);

    // create the transaction
    let input_notes_pos = [0, 1];
    let obfuscate_transfer_note = true;
    let deposit = 0;
    let contract_call: Option<ContractCall> = None;

    let tx = create_phoenix_transaction(
        rng,
        session,
        &phoenix_sender_sk,
        &phoenix_change_pk,
        &phoenix_receiver_pk,
        GAS_LIMIT,
        GAS_PRICE,
        input_notes_pos,
        TRANSFER_VALUE,
        obfuscate_transfer_note,
        deposit,
        contract_call,
    );

    let gas_spent = execute(session, &tx, 0, 0, 0)
        .expect("Executing TX should succeed")
        .gas_spent;
    update_root(session).expect("Updating the root should succeed");

    println!("TRANSFER 2-2: {} gas", gas_spent);

    // check that correct notes have been generated
    let leaves = leaves_from_pos(session, N as u64)
        .expect("Getting the new notes should succeed");
    assert_eq!(
        leaves.len(),
        3,
        "Transfer, change and refund notes should have been added to the tree"
    );
    let amount_notes =
        num_notes(session).expect("Getting num_notes should succeed");
    assert_eq!(
        amount_notes,
        leaves.last().expect("note to exists").note.pos() + 1,
        "num_notes should match position of last note + 1"
    );

    // check that the genesis notes have been nullified
    let input_nullifiers =
        gen_nullifiers(session, input_notes_pos, &phoenix_sender_sk);
    let existing_nullifers = existing_nullifiers(session, &input_nullifiers)
        .expect("Querying the nullifiers should work");
    assert_eq!(input_nullifiers, existing_nullifers);

    // the sender's balance has decreased
    let new_sender_notes = filter_notes_owned_by(
        phoenix_sender_vk,
        leaves.iter().map(|leaf| leaf.note.clone()),
    );
    assert_eq!(
        new_sender_notes.len(),
        2,
        "Change and refund notes should be owned by the sender"
    );
    assert_eq!(
        owned_notes_value(phoenix_sender_vk, &new_sender_notes),
        PHOENIX_GENESIS_VALUE - gas_spent - TRANSFER_VALUE,
        "The new notes should carry the genesis-value minus the spent gas and transfer value"
    );

    // the receiver's balance has increased
    let new_receiver_notes = filter_notes_owned_by(
        phoenix_receiver_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );
    assert_eq!(
        new_receiver_notes.len(),
        1,
        "The receiver should own the transfer note"
    );
    assert_eq!(
        owned_notes_value(phoenix_receiver_vk, &new_receiver_notes),
        TRANSFER_VALUE,
        "The receiver should own the transfer-value"
    );
}

/// Perform a simple transfer of funds between two phoenix addresses, using
/// three input notes.
#[test]
fn transfer_3_2() {
    const N: u8 = 3;
    // the genesis notes each hold the genesis value / 3, the gas costs will
    // force us to spent all genesis notes
    const TRANSFER_VALUE: u64 =
        PHOENIX_GENESIS_VALUE - PHOENIX_GENESIS_VALUE / N as u64;

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let phoenix_sender_sk = PhoenixSecretKey::random(rng);
    let phoenix_sender_pk = PhoenixPublicKey::from(&phoenix_sender_sk);
    let phoenix_sender_vk = PhoenixViewKey::from(&phoenix_sender_sk);

    let phoenix_change_pk = phoenix_sender_pk.clone();

    let phoenix_receiver_sk = PhoenixSecretKey::random(rng);
    let phoenix_receiver_pk = PhoenixPublicKey::from(&phoenix_receiver_sk);
    let phoenix_receiver_vk = PhoenixViewKey::from(&phoenix_receiver_sk);

    let session = &mut instantiate::<N>(rng, &phoenix_sender_sk);

    // create the transaction
    let input_notes_pos = [0, 1, 2];
    let obfuscate_transfer_note = true;
    let deposit = 0;
    let contract_call: Option<ContractCall> = None;

    let tx = create_phoenix_transaction(
        rng,
        session,
        &phoenix_sender_sk,
        &phoenix_change_pk,
        &phoenix_receiver_pk,
        GAS_LIMIT,
        GAS_PRICE,
        input_notes_pos,
        TRANSFER_VALUE,
        obfuscate_transfer_note,
        deposit,
        contract_call,
    );

    let gas_spent = execute(session, &tx, 0, 0, 0)
        .expect("Executing TX should succeed")
        .gas_spent;
    update_root(session).expect("Updating the root should succeed");

    println!("TRANSFER 3-2: {} gas", gas_spent);

    // check that correct notes have been generated
    let leaves = leaves_from_pos(session, N as u64)
        .expect("Getting the new notes should succeed");
    assert_eq!(
        leaves.len(),
        3,
        "Transfer, change and refund notes should have been added to the tree"
    );
    let amount_notes =
        num_notes(session).expect("Getting num_notes should succeed");
    assert_eq!(
        amount_notes,
        leaves.last().expect("note to exists").note.pos() + 1,
        "num_notes should match position of last note + 1"
    );

    // check that the genesis notes have been nullified
    let input_nullifiers =
        gen_nullifiers(session, input_notes_pos, &phoenix_sender_sk);
    let existing_nullifers = existing_nullifiers(session, &input_nullifiers)
        .expect("Querrying the nullifiers should work");
    assert_eq!(input_nullifiers, existing_nullifers);

    // the sender's balance has decreased
    let new_sender_notes = filter_notes_owned_by(
        phoenix_sender_vk,
        leaves.iter().map(|leaf| leaf.note.clone()),
    );
    assert_eq!(
        new_sender_notes.len(),
        2,
        "Change and refund notes should be owned by the sender"
    );
    assert_eq!(
        owned_notes_value(phoenix_sender_vk, &new_sender_notes),
        PHOENIX_GENESIS_VALUE - gas_spent - TRANSFER_VALUE,
        "The new notes should carry the genesis-value minus the spent gas and transfer value"
    );

    // the receiver's balance has increased
    let new_receiver_notes = filter_notes_owned_by(
        phoenix_receiver_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );
    assert_eq!(
        new_receiver_notes.len(),
        1,
        "The receiver should own the transfer note"
    );
    assert_eq!(
        owned_notes_value(phoenix_receiver_vk, &new_receiver_notes),
        TRANSFER_VALUE,
        "The receiver should own the transfer-value"
    );
}

/// Perform a simple transfer of funds between two phoenix addresses, using four
/// input notes.
#[test]
fn transfer_4_2() {
    const N: u8 = 4;
    // the genesis notes each hold the genesis value / 4, the gas costs will
    // force us to spent all genesis notes
    const TRANSFER_VALUE: u64 =
        PHOENIX_GENESIS_VALUE - PHOENIX_GENESIS_VALUE / N as u64;

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let phoenix_sender_sk = PhoenixSecretKey::random(rng);
    let phoenix_sender_pk = PhoenixPublicKey::from(&phoenix_sender_sk);
    let phoenix_sender_vk = PhoenixViewKey::from(&phoenix_sender_sk);

    let phoenix_change_pk = phoenix_sender_pk.clone();

    let phoenix_receiver_sk = PhoenixSecretKey::random(rng);
    let phoenix_receiver_pk = PhoenixPublicKey::from(&phoenix_receiver_sk);
    let phoenix_receiver_vk = PhoenixViewKey::from(&phoenix_receiver_sk);

    let session = &mut instantiate::<N>(rng, &phoenix_sender_sk);

    // create the transaction
    let input_notes_pos = [0, 1, 2, 3];
    let obfuscate_transfer_note = true;
    let deposit = 0;
    let contract_call: Option<ContractCall> = None;

    let tx = create_phoenix_transaction(
        rng,
        session,
        &phoenix_sender_sk,
        &phoenix_change_pk,
        &phoenix_receiver_pk,
        GAS_LIMIT,
        GAS_PRICE,
        input_notes_pos,
        TRANSFER_VALUE,
        obfuscate_transfer_note,
        deposit,
        contract_call,
    );

    let gas_spent = execute(session, &tx, 0, 0, 0)
        .expect("Executing TX should succeed")
        .gas_spent;
    update_root(session).expect("Updating the root should succeed");

    println!("TRANSFER 4-2: {} gas", gas_spent);

    // check that correct notes have been generated
    let leaves = leaves_from_pos(session, N as u64)
        .expect("Getting the new notes should succeed");
    assert_eq!(
        leaves.len(),
        3,
        "Transfer, change and refund notes should have been added to the tree"
    );
    let amount_notes =
        num_notes(session).expect("Getting num_notes should succeed");
    assert_eq!(
        amount_notes,
        leaves.last().expect("note to exists").note.pos() + 1,
        "num_notes should match position of last note + 1"
    );

    // check that the genesis notes have been nullified
    let input_nullifiers =
        gen_nullifiers(session, input_notes_pos, &phoenix_sender_sk);
    let existing_nullifers = existing_nullifiers(session, &input_nullifiers)
        .expect("Querrying the nullifiers should work");
    assert_eq!(input_nullifiers, existing_nullifers);

    // the sender's balance has decreased
    let new_sender_notes = filter_notes_owned_by(
        phoenix_sender_vk,
        leaves.iter().map(|leaf| leaf.note.clone()),
    );
    assert_eq!(
        new_sender_notes.len(),
        2,
        "Change and refund notes should be owned by the sender"
    );
    assert_eq!(
        owned_notes_value(phoenix_sender_vk, &new_sender_notes),
        PHOENIX_GENESIS_VALUE - gas_spent - TRANSFER_VALUE,
        "The new notes should carry the genesis-value minus the spent gas and transfer value"
    );

    // the receiver's balance has increased
    let new_receiver_notes = filter_notes_owned_by(
        phoenix_receiver_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );
    assert_eq!(
        new_receiver_notes.len(),
        1,
        "The receiver should own the transfer note"
    );
    assert_eq!(
        owned_notes_value(phoenix_receiver_vk, &new_receiver_notes),
        TRANSFER_VALUE,
        "The receiver should own the transfer-value"
    );
}

/// Checks if a transaction fails when the gas-price is 0.
#[test]
fn transfer_gas_fails() {
    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let phoenix_sender_sk = PhoenixSecretKey::random(rng);
    let phoenix_sender_pk = PhoenixPublicKey::from(&phoenix_sender_sk);
    let phoenix_sender_vk = PhoenixViewKey::from(&phoenix_sender_sk);

    let phoenix_change_pk = phoenix_sender_pk.clone();

    let phoenix_receiver_pk =
        PhoenixPublicKey::from(&PhoenixSecretKey::random(rng));

    let session = &mut instantiate::<1>(rng, &phoenix_sender_sk);

    let gas_price = 0;
    let input_note_pos = 0;
    let transfer_value = 42;
    let obfuscate_transfer_note = true;
    let deposit = 0;
    let contract_call: Option<ContractCall> = None;

    let tx = create_phoenix_transaction(
        rng,
        session,
        &phoenix_sender_sk,
        &phoenix_change_pk,
        &phoenix_receiver_pk,
        GAS_LIMIT,
        gas_price,
        [input_note_pos],
        transfer_value,
        obfuscate_transfer_note,
        deposit,
        contract_call,
    );

    let total_num_notes_before_tx =
        num_notes(session).expect("Getting num_notes should succeed");

    let result = execute(session, &tx, 0, 0, 0);

    assert!(
        result.is_err(),
        "Transaction should fail due to zero gas price"
    );

    // After the failed transaction, verify the state is unchanged
    let leaves_after_fail = leaves_from_pos(session, input_note_pos + 1)
        .expect("Getting the leaves should succeed after failed transaction");

    assert_eq!(
        leaves_after_fail.len(),
        0,
        "No new notes should have been added to the tree"
    );

    let total_num_notes_after_tx =
        num_notes(session).expect("Getting num_notes should succeed");

    assert_eq!(
        total_num_notes_after_tx, total_num_notes_before_tx,
        "num_notes should not increase due to the failed transaction"
    );

    assert_eq!(
        new_owned_notes_value(session, 0, phoenix_sender_vk),
        PHOENIX_GENESIS_VALUE,
        "The sender should still own the genesis value"
    );
}

/// Performs a simple contract-call.
#[test]
fn alice_ping() {
    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let phoenix_sender_sk = PhoenixSecretKey::random(rng);
    let phoenix_sender_pk = PhoenixPublicKey::from(&phoenix_sender_sk);
    let phoenix_sender_vk = PhoenixViewKey::from(&phoenix_sender_sk);

    let phoenix_change_pk = phoenix_sender_pk.clone();

    let session = &mut instantiate::<1>(rng, &phoenix_sender_sk);

    // create the transaction
    let input_note_pos = 0;
    let transfer_value = 0;
    let obfuscate_transfer_note = false;
    let deposit = 0;
    let contract_call = Some(ContractCall {
        contract: ALICE_ID,
        fn_name: String::from("ping"),
        fn_args: vec![],
    });

    let tx = create_phoenix_transaction(
        rng,
        session,
        &phoenix_sender_sk,
        &phoenix_change_pk,
        &phoenix_sender_pk,
        GAS_LIMIT,
        GAS_PRICE,
        [input_note_pos],
        transfer_value,
        obfuscate_transfer_note,
        deposit,
        contract_call,
    );

    let gas_spent = execute(session, &tx, 0, 0, 0)
        .expect("Executing TX should succeed")
        .gas_spent;
    update_root(session).expect("Updating the root should succeed");

    println!("CONTRACT PING: {} gas", gas_spent);

    let leaves = leaves_from_height(session, 1)
        .expect("Getting the notes should succeed");
    assert_eq!(
        leaves.len(),
        // change and refund note were added to the tree
        2,
        "There should be two notes in the tree after the transaction"
    );
    let owned_notes = filter_notes_owned_by(
        phoenix_sender_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );
    assert_eq!(
        owned_notes.len(),
        2,
        "all new notes should belong to the sender"
    );
    assert_eq!(
        owned_notes_value(phoenix_sender_vk, &owned_notes),
        PHOENIX_GENESIS_VALUE - gas_spent,
        "the sender's balance should have decreased by the spent gas"
    );
}

/// Deposit funds into a contract.
#[test]
fn contract_deposit() {
    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let phoenix_sender_sk = PhoenixSecretKey::random(rng);
    let phoenix_sender_vk = PhoenixViewKey::from(&phoenix_sender_sk);
    let phoenix_sender_pk = PhoenixPublicKey::from(&phoenix_sender_sk);

    let phoenix_change_pk = phoenix_sender_pk.clone();

    let session = &mut instantiate::<1>(rng, &phoenix_sender_sk);

    // create the deposit transaction
    let input_note_pos = 0;
    let transfer_value = 0;
    let obfuscate_transfer_note = false;
    let deposit_value = PHOENIX_GENESIS_VALUE / 2;
    let contract_call = Some(ContractCall {
        contract: ALICE_ID,
        fn_name: String::from("deposit"),
        fn_args: deposit_value.to_bytes().into(),
    });

    let tx = create_phoenix_transaction(
        rng,
        session,
        &phoenix_sender_sk,
        &phoenix_change_pk,
        &phoenix_sender_pk,
        GAS_LIMIT,
        GAS_PRICE,
        [input_note_pos],
        transfer_value,
        obfuscate_transfer_note,
        deposit_value,
        contract_call,
    );

    let gas_spent = execute(session, &tx, 0, 0, 0)
        .expect("Executing TX should succeed")
        .gas_spent;
    update_root(session).expect("Updating the root should succeed");

    println!("CONTRACT DEPOSIT: {} gas", gas_spent);

    let leaves = leaves_from_height(session, 1)
        .expect("getting the notes should succeed");
    assert_eq!(
        PHOENIX_GENESIS_VALUE,
        transfer_value
            + tx.deposit()
            + tx.gas_limit() * tx.gas_price()
            + tx.outputs()[1]
                .value(Some(&PhoenixViewKey::from(&phoenix_sender_sk)))
                .unwrap()
    );
    assert_eq!(
        leaves.len(),
        2,
        "Change and refund notes should have been added to the tree"
    );
    let new_notes = filter_notes_owned_by(
        phoenix_sender_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );
    assert_eq!(
        new_notes.len(),
        2,
        "All new notes should be owned by our view key"
    );
    let new_notes_value = owned_notes_value(phoenix_sender_vk, &new_notes);
    assert_eq!(
        new_notes_value,
        PHOENIX_GENESIS_VALUE - deposit_value - gas_spent,
        "The sender's balance should have decreased by the deposit and the spent gas"
    );

    // check that alice contract has the correct balance

    let alice_balance = contract_balance(session, ALICE_ID)
        .expect("Querying the contract balance should succeed");
    assert_eq!(
        alice_balance,
        deposit_value + ALICE_GENESIS_VALUE,
        "Alice's balance should have increased by the value of the deposit"
    );
}

// Withdraw funds from a contract.
#[test]
fn contract_withdraw() {
    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let phoenix_sender_sk = PhoenixSecretKey::random(rng);
    let phoenix_sender_vk = PhoenixViewKey::from(&phoenix_sender_sk);
    let phoenix_sender_pk = PhoenixPublicKey::from(&phoenix_sender_sk);

    let phoenix_change_pk = phoenix_sender_pk.clone();

    let session = &mut instantiate::<1>(rng, &phoenix_sender_sk);

    // withdraw alice's genesis balance, this is done by calling the alice
    // contract directly, which then calls the `withdraw` method of the transfer
    // contract
    let transfer_value = 0;
    let obfuscate_transfer_note = false;
    let deposit_value = 0;

    let address =
        phoenix_sender_pk.gen_stealth_address(&JubJubScalar::random(&mut *rng));
    let note_sk = phoenix_sender_sk.gen_note_sk(&address);
    let genesis_note_nullifier = leaves_from_pos(session, 0)
        .expect("Getting leaves from genesis note should succeed")[0]
        .note
        .gen_nullifier(&phoenix_sender_sk);

    let contract_call = Some(ContractCall {
        contract: ALICE_ID,
        fn_name: String::from("withdraw"),
        fn_args: rkyv::to_bytes::<_, 1024>(&Withdraw::new(
            rng,
            &note_sk,
            ALICE_ID,
            ALICE_GENESIS_VALUE,
            WithdrawReceiver::Phoenix(address),
            WithdrawReplayToken::Phoenix(vec![genesis_note_nullifier]),
        ))
        .expect("should serialize Mint correctly")
        .to_vec(),
    });

    let tx = create_phoenix_transaction(
        rng,
        session,
        &phoenix_sender_sk,
        &phoenix_change_pk,
        &phoenix_sender_pk,
        GAS_LIMIT,
        GAS_PRICE,
        [0],
        transfer_value,
        obfuscate_transfer_note,
        deposit_value,
        contract_call,
    );

    let gas_spent = execute(session, &tx, 0, 0, 0)
        .expect("Executing TX should succeed")
        .gas_spent;
    update_root(session).expect("Updating the root should succeed");

    println!("CONTRACT WITHDRAW: {} gas", gas_spent);

    let leaves = leaves_from_height(session, 1)
        .expect("getting the notes should succeed");
    assert_eq!(
        leaves.len(),
        3,
        "Withdrawal, Change and refund notes should have been added to the tree"
    );
    let new_notes = filter_notes_owned_by(
        phoenix_sender_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );
    assert_eq!(
        new_notes.len(),
        3,
        "All new notes should be owned by our view key"
    );
    let new_notes_value = owned_notes_value(phoenix_sender_vk, &new_notes);
    assert_eq!(
        new_notes_value,
        PHOENIX_GENESIS_VALUE + ALICE_GENESIS_VALUE - gas_spent,
        "The sender's balance should have increased by the withdrawal value minus the spent gas"
    );

    let alice_balance = contract_balance(session, ALICE_ID)
        .expect("Querying the contract balance should succeed");
    assert_eq!(
        alice_balance, 0,
        "Alice should have no balance after it is withdrawn"
    );
}

/// Converting moonlight DUSK into phoenix DUSK with a phoenix transaction
/// should fail.
#[test]
fn convert_to_phoenix_fails() {
    const CONVERSION_VALUE: u64 = dusk(10.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let phoenix_sender_sk = PhoenixSecretKey::random(rng);
    let phoenix_sender_vk = PhoenixViewKey::from(&phoenix_sender_sk);
    let phoenix_sender_pk = PhoenixPublicKey::from(&phoenix_sender_sk);

    let phoenix_change_pk = phoenix_sender_pk.clone();

    let moonlight_sk = AccountSecretKey::random(rng);
    let moonlight_pk = AccountPublicKey::from(&moonlight_sk);

    let mut session = &mut instantiate::<1>(rng, &phoenix_sender_sk);

    // Add the conversion value to the moonlight account
    session
        .call::<_, ()>(
            TRANSFER_CONTRACT,
            "add_account_balance",
            &(moonlight_pk, CONVERSION_VALUE),
            GAS_LIMIT,
        )
        .expect("Inserting genesis account should succeed");

    // make sure the moonlight account owns the conversion value
    let moonlight_account = account(&mut session, &moonlight_pk)
        .expect("Getting account should succeed");
    assert_eq!(
        moonlight_account.balance, CONVERSION_VALUE,
        "The moonlight account should own the conversion value before the transaction"
    );

    // we need to retrieve the genesis-note to generate its nullifier
    let leaves = leaves_from_height(session, 0)
        .expect("getting the notes should succeed");
    let notes = filter_notes_owned_by(
        phoenix_sender_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );
    assert_eq!(notes.len(), 1, "There should be one note at this height");

    // generate a new note stealth-address and note-sk for the conversion
    let address =
        phoenix_sender_pk.gen_stealth_address(&JubJubScalar::random(&mut *rng));
    let note_sk = phoenix_sender_sk.gen_note_sk(&address);

    // the moonlight replay token
    let nonce = 1;

    // a conversion is a deposit into the transfer-contract paired with a
    // withdrawal
    let contract_call = ContractCall {
        contract: TRANSFER_CONTRACT,
        fn_name: String::from("convert"),
        fn_args: rkyv::to_bytes::<_, 1024>(&Withdraw::new(
            rng,
            &note_sk,
            TRANSFER_CONTRACT,
            // set the conversion-value as a withdrawal
            CONVERSION_VALUE,
            WithdrawReceiver::Phoenix(address),
            WithdrawReplayToken::Moonlight(nonce),
        ))
        .expect("should serialize conversion correctly")
        .to_vec(),
    };

    let tx = create_phoenix_transaction(
        rng,
        session,
        &phoenix_sender_sk,
        &phoenix_change_pk,
        &phoenix_sender_pk,
        GAS_LIMIT,
        LUX,
        [0],
        0,
        false,
        // set the conversion-value as the deposit
        CONVERSION_VALUE,
        Some(contract_call),
    );

    let receipt =
        execute(session, &tx, 0, 0, 0).expect("Executing TX should succeed");

    // check that the transaction execution panicked with the correct message
    assert!(receipt.data.is_err());
    assert_eq!(
        format!("{}", receipt.data.unwrap_err()),
        String::from("Panic: Expected Moonlight TX, found Phoenix"),
        "The attempted conversion from moonlight to phoenix when paying gas with phoenix should error"
    );
    assert_eq!(
        receipt.gas_spent,
        GAS_LIMIT * LUX,
        "The max gas should have been spent"
    );

    update_root(session).expect("Updating the root should succeed");

    println!("CONVERT TO PHOENIX: {} gas", receipt.gas_spent);

    let moonlight_account = account(&mut session, &moonlight_pk)
        .expect("Getting account should succeed");

    assert_eq!(
        moonlight_account.balance,
        CONVERSION_VALUE,
        "Since the conversion failed, the moonlight account should still own the conversion value"
    );

    let leaves = leaves_from_height(session, 1)
        .expect("getting the notes should succeed");
    let notes = filter_notes_owned_by(
        phoenix_sender_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );
    let notes_value = owned_notes_value(phoenix_sender_vk, &notes);

    assert_eq!(
        notes.len(),
        2,
        "Change and refund notes should have been created"
    );
    assert_eq!(
        notes_value,
        PHOENIX_GENESIS_VALUE - receipt.gas_spent,
        "The new notes should have the original value minus the spent gas"
    );
}

/// Convert phoenix DUSK into moonlight DUSK.
#[test]
fn convert_to_moonlight() {
    const CONVERSION_VALUE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let phoenix_sender_sk = PhoenixSecretKey::random(rng);
    let phoenix_sender_vk = PhoenixViewKey::from(&phoenix_sender_sk);
    let phoenix_sender_pk = PhoenixPublicKey::from(&phoenix_sender_sk);

    let phoenix_change_pk = phoenix_sender_pk.clone();

    let moonlight_sk = AccountSecretKey::random(rng);
    let moonlight_pk = AccountPublicKey::from(&moonlight_sk);

    let mut session = &mut instantiate::<1>(rng, &phoenix_sender_sk);

    // make sure the moonlight account doesn't own any funds before the
    // conversion
    let moonlight_account = account(&mut session, &moonlight_pk)
        .expect("Getting account should succeed");
    assert_eq!(
        moonlight_account.balance, 0,
        "The moonlight account should have 0 dusk before the conversion"
    );

    // we need to retrieve the genesis-note to generate its nullifier
    let leaves = leaves_from_height(session, 0)
        .expect("getting the notes should succeed");
    let notes = filter_notes_owned_by(
        phoenix_sender_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );
    assert_eq!(notes.len(), 1, "There should be one note at this height");

    // a conversion is a deposit into the transfer-contract paired with a
    // withdrawal
    let contract_call = ContractCall {
        contract: TRANSFER_CONTRACT,
        fn_name: String::from("convert"),
        fn_args: rkyv::to_bytes::<_, 1024>(&Withdraw::new(
            rng,
            &moonlight_sk,
            TRANSFER_CONTRACT,
            // set the conversion-value as a withdrawal
            CONVERSION_VALUE,
            WithdrawReceiver::Moonlight(moonlight_pk),
            WithdrawReplayToken::Phoenix(vec![
                notes[0].gen_nullifier(&phoenix_sender_sk)
            ]),
        ))
        .expect("should serialize conversion correctly")
        .to_vec(),
    };

    let tx = create_phoenix_transaction(
        rng,
        session,
        &phoenix_sender_sk,
        &phoenix_change_pk,
        &phoenix_sender_pk,
        GAS_LIMIT,
        LUX,
        [0],
        0,
        false,
        // set the conversion-value as the deposit
        CONVERSION_VALUE,
        Some(contract_call),
    );

    let gas_spent = execute(session, &tx, 0, 0, 0)
        .expect("Executing TX should succeed")
        .gas_spent;
    update_root(session).expect("Updating the root should succeed");

    println!("CONVERT TO MOONLIGHT: {} gas", gas_spent);

    let moonlight_account = account(&mut session, &moonlight_pk)
        .expect("Getting account should succeed");

    assert_eq!(
        moonlight_account.balance, CONVERSION_VALUE,
        "The moonlight account should have conversion value added"
    );

    let leaves = leaves_from_height(session, 1)
        .expect("getting the notes should succeed");
    let notes = filter_notes_owned_by(
        phoenix_sender_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );
    let notes_value = owned_notes_value(phoenix_sender_vk, &notes);

    assert_eq!(
        notes.len(),
        2,
        "New notes should have been created as change and refund (transparent notes with the value 0 are not appended to the tree)"
    );
    assert_eq!(
        notes_value,
        PHOENIX_GENESIS_VALUE - gas_spent - CONVERSION_VALUE,
        "The new notes should have the original value minus the conversion value and gas spent"
    );
}

/// Attempts to convert phoenix DUSK into moonlight DUSK but fails due to not
/// targeting the correct contract for the conversion.
#[test]
fn convert_wrong_contract_targeted() {
    const CONVERSION_VALUE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let phoenix_sender_sk = PhoenixSecretKey::random(rng);
    let phoenix_sender_vk = PhoenixViewKey::from(&phoenix_sender_sk);
    let phoenix_sender_pk = PhoenixPublicKey::from(&phoenix_sender_sk);

    let phoenix_change_pk = phoenix_sender_pk.clone();

    let moonlight_sk = AccountSecretKey::random(rng);
    let moonlight_pk = AccountPublicKey::from(&moonlight_sk);

    let mut session = &mut instantiate::<1>(rng, &phoenix_sender_sk);

    // make sure the moonlight account doesn't own any funds before the
    // conversion
    let moonlight_account = account(&mut session, &moonlight_pk)
        .expect("Getting account should succeed");
    assert_eq!(
        moonlight_account.balance, 0,
        "The moonlight account should have 0 dusk before the conversion"
    );

    // we need to retrieve the genesis-note to generate its nullifier
    let leaves = leaves_from_height(session, 0)
        .expect("getting the notes should succeed");
    let notes = filter_notes_owned_by(
        phoenix_sender_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );
    assert_eq!(
        notes.len(),
        1,
        "There should be the genesis note at height 0"
    );

    let contract_call = ContractCall {
        contract: TRANSFER_CONTRACT,
        fn_name: String::from("convert"),
        fn_args: rkyv::to_bytes::<_, 1024>(&Withdraw::new(
            rng,
            &moonlight_sk,
            // this should be the transfer contract, but we're testing the
            // "wrong target" case
            ALICE_ID,
            CONVERSION_VALUE,
            WithdrawReceiver::Moonlight(moonlight_pk),
            WithdrawReplayToken::Phoenix(vec![
                notes[0].gen_nullifier(&phoenix_sender_sk)
            ]),
        ))
        .expect("should serialize conversion correctly")
        .to_vec(),
    };

    let tx = create_phoenix_transaction(
        rng,
        session,
        &phoenix_sender_sk,
        &phoenix_change_pk,
        &phoenix_sender_pk,
        GAS_LIMIT,
        LUX,
        [0],
        0,
        false,
        CONVERSION_VALUE,
        Some(contract_call),
    );

    let receipt = execute(&mut session, &tx, 0, 0, 0)
        .expect("Executing transaction should succeed");
    update_root(session).expect("Updating the root should succeed");

    let res = receipt.data;
    let gas_spent = receipt.gas_spent;

    println!(
        "CONVERT TO MOONLIGHT (wrong contract targeted): {} gas",
        gas_spent
    );

    assert!(matches!(res, Err(_)), "The contract call should error");

    let moonlight_account = account(&mut session, &moonlight_pk)
        .expect("Getting account should succeed");

    assert_eq!(
        moonlight_account.balance, 0,
        "The moonlight account should not have received funds"
    );

    let leaves = leaves_from_height(session, 1)
        .expect("getting the notes should succeed");
    let notes = filter_notes_owned_by(
        phoenix_sender_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );
    let notes_value = owned_notes_value(phoenix_sender_vk, &notes);

    assert_eq!(
        notes.len(),
        2,
        "New notes should have been created as change and refund (transparent notes with the value 0 are not appended to the tree)"
    );
    assert_eq!(
        notes_value,
        PHOENIX_GENESIS_VALUE - gas_spent,
        "The new notes should have the original value minus gas spent"
    );
}

/// In this test we call Alice's `contract_to_contract` function, targeting Bob
/// as the receiver of the transfer. The gas will be paid in phoenix.
#[test]
fn contract_to_contract() {
    const TRANSFER_VALUE: u64 = ALICE_GENESIS_VALUE / 2;

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let phoenix_sender_sk = PhoenixSecretKey::random(rng);
    let phoenix_sender_vk = PhoenixViewKey::from(&phoenix_sender_sk);
    let phoenix_sender_pk = PhoenixPublicKey::from(&phoenix_sender_sk);

    let phoenix_change_pk = phoenix_sender_pk.clone();

    let session = &mut instantiate::<1>(rng, &phoenix_sender_sk);

    // make sure bob contract has no balance prior to the tx
    let bob_balance = contract_balance(session, BOB_ID)
        .expect("Querying the contract balance should succeed");
    assert_eq!(bob_balance, 0, "Bob must have an initial balance of zero");

    let contract_call = ContractCall {
        contract: ALICE_ID,
        fn_name: String::from("contract_to_contract"),
        fn_args: rkyv::to_bytes::<_, 256>(&ContractToContract {
            contract: BOB_ID,
            value: TRANSFER_VALUE,
            fn_name: String::from("recv_transfer"),
            data: vec![],
        })
        .expect("Serializing should succeed")
        .to_vec(),
    };

    let tx = create_phoenix_transaction(
        rng,
        session,
        &phoenix_sender_sk,
        &phoenix_change_pk,
        &phoenix_sender_pk,
        GAS_LIMIT,
        LUX,
        [0],
        0,
        false,
        0,
        Some(contract_call),
    );

    let receipt =
        execute(session, &tx, 0, 0, 0).expect("Transaction should succeed");
    let gas_spent = receipt.gas_spent;

    println!("CONTRACT TO CONTRACT: {gas_spent} gas");

    let alice_balance = contract_balance(session, ALICE_ID)
        .expect("Querying the contract balance should succeed");
    let bob_balance = contract_balance(session, BOB_ID)
        .expect("Querying the contract balance should succeed");

    assert_eq!(
        alice_balance,
        ALICE_GENESIS_VALUE - TRANSFER_VALUE,
        "Alice's balance should have decreased by the transfer value"
    );
    assert_eq!(
        bob_balance, TRANSFER_VALUE,
        "Bob's balance must have increased by the transfer value"
    );

    let leaves = leaves_from_height(session, 1)
        .expect("getting the notes should succeed");
    let notes = filter_notes_owned_by(
        phoenix_sender_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );
    let notes_value = owned_notes_value(phoenix_sender_vk, &notes);
    assert_eq!(
        notes.len(),
        2,
        "New notes should have been created as change and refund (transparent notes with the value 0 are not appended to the tree)"
    );
    assert_eq!(
        notes_value,
        PHOENIX_GENESIS_VALUE - gas_spent,
        "The new notes should have the original value minus gas spent"
    );
}

/// In this test we call the Alice contract to trigger a transfer of funds into
/// a moonlight account, while the gas will be paid with phoenix.
#[test]
fn contract_to_account() {
    const TRANSFER_VALUE: u64 = ALICE_GENESIS_VALUE / 2;

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let phoenix_sender_sk = PhoenixSecretKey::random(rng);
    let phoenix_sender_vk = PhoenixViewKey::from(&phoenix_sender_sk);
    let phoenix_sender_pk = PhoenixPublicKey::from(&phoenix_sender_sk);

    let phoenix_change_pk = phoenix_sender_pk.clone();

    let moonlight_sk = AccountSecretKey::random(rng);
    let moonlight_pk = AccountPublicKey::from(&moonlight_sk);

    let mut session = &mut instantiate::<1>(rng, &phoenix_sender_sk);

    // make sure the moonlight account doesn't own any funds before the
    // conversion
    let moonlight_account = account(&mut session, &moonlight_pk)
        .expect("Getting account should succeed");
    assert_eq!(
        moonlight_account.balance, 0,
        "The moonlight account should have 0 dusk before the transfer"
    );

    let contract_call = ContractCall {
        contract: ALICE_ID,
        fn_name: String::from("contract_to_account"),
        fn_args: rkyv::to_bytes::<_, 256>(&ContractToAccount {
            account: moonlight_pk,
            value: TRANSFER_VALUE,
        })
        .expect("Serializing should succeed")
        .to_vec(),
    };

    let tx = create_phoenix_transaction(
        rng,
        session,
        &phoenix_sender_sk,
        &phoenix_change_pk,
        &phoenix_sender_pk,
        GAS_LIMIT,
        LUX,
        [0],
        0,
        false,
        0,
        Some(contract_call),
    );

    let receipt =
        execute(session, &tx, 0, 0, 0).expect("Transaction should succeed");
    let gas_spent = receipt.gas_spent;

    println!("CONTRACT TO ACCOUNT: {gas_spent} gas");

    let moonlight_account = account(session, &moonlight_pk)
        .expect("Getting the account should succeed");
    let alice_balance = contract_balance(session, ALICE_ID)
        .expect("Querying the contract balance should succeed");

    assert_eq!(
        moonlight_account.balance, TRANSFER_VALUE,
        "The account's balance should have increased by the transfer value"
    );
    assert_eq!(
        alice_balance,
        ALICE_GENESIS_VALUE - TRANSFER_VALUE,
        "Alice's balance should have decreased by the transfer value"
    );

    let leaves = leaves_from_height(session, 1)
        .expect("getting the notes should succeed");
    let notes = filter_notes_owned_by(
        phoenix_sender_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );
    let notes_value = owned_notes_value(phoenix_sender_vk, &notes);
    assert_eq!(
        notes.len(),
        2,
        "New notes should have been created as change and refund"
    );
    assert_eq!(
        notes_value,
        PHOENIX_GENESIS_VALUE - gas_spent,
        "The new notes should have the original value minus gas spent"
    );
}

// ----------------
// helper functions

fn leaves_from_pos(
    session: &mut Session,
    pos: u64,
) -> Result<Vec<NoteLeaf>, VMError> {
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

fn num_notes(session: &mut Session) -> Result<u64, VMError> {
    session
        .call(TRANSFER_CONTRACT, "num_notes", &(), u64::MAX)
        .map(|r| r.data)
}

fn root(session: &mut Session) -> Result<BlsScalar, VMError> {
    session
        .call(TRANSFER_CONTRACT, "root", &(), GAS_LIMIT)
        .map(|r| r.data)
}

fn opening(
    session: &mut Session,
    pos: u64,
) -> Result<Option<NoteOpening>, VMError> {
    session
        .call(TRANSFER_CONTRACT, "opening", &pos, GAS_LIMIT)
        .map(|r| r.data)
}

fn gen_nullifiers(
    session: &mut Session,
    notes_pos: impl AsRef<[u64]>,
    sk: &PhoenixSecretKey,
) -> Vec<BlsScalar> {
    notes_pos
        .as_ref()
        .iter()
        .map(|pos| {
            let note = &leaves_from_pos(session, *pos)
                .expect("the position should exist")[0]
                .note;
            note.gen_nullifier(sk)
        })
        .collect()
}

/// Generate a TxCircuit given the sender secret-key, receiver public-key, the
/// input note positions in the transaction tree and the new output-notes.
fn create_phoenix_transaction<const I: usize>(
    rng: &mut StdRng,
    session: &mut Session,
    sender_sk: &PhoenixSecretKey,
    refund_pk: &PhoenixPublicKey,
    receiver_pk: &PhoenixPublicKey,
    gas_limit: u64,
    gas_price: u64,
    input_pos: [u64; I],
    transfer_value: u64,
    obfuscated_transaction: bool,
    deposit: u64,
    data: Option<impl Into<TransactionData>>,
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
        assert!(opening.verify(NoteTreeItem::new(note.hash(), ())));

        inputs.push((note.clone(), opening));
    }

    PhoenixTransaction::new(
        rng,
        sender_sk,
        refund_pk,
        receiver_pk,
        inputs,
        root,
        transfer_value,
        obfuscated_transaction,
        deposit,
        gas_limit,
        gas_price,
        CHAIN_ID,
        data.map(Into::into),
        &LocalProver,
    )
    .expect("creating the creation shouldn't fail")
    .into()
}
