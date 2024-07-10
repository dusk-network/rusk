// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod common;

use crate::common::{
    account, contract_balance, create_moonlight_transaction,
    create_phoenix_transaction, execute, filter_notes_owned_by,
    leaves_from_height, leaves_from_pos, num_notes, update_root,
};

use dusk_bytes::Serializable;
use ff::Field;
use rand::rngs::StdRng;
use rand::{CryptoRng, RngCore, SeedableRng};

use execution_core::{
    transfer::{
        ContractCall, ContractExec, Withdraw, WithdrawReceiver,
        WithdrawReplayToken,
    },
    BlsPublicKey, BlsSecretKey, JubJubScalar, Note, PublicKey, SecretKey,
    ViewKey,
};
use rusk_abi::dusk::{dusk, LUX};
use rusk_abi::{ContractData, ContractId, Session, TRANSFER_CONTRACT, VM};

const PHOENIX_GENESIS_VALUE: u64 = dusk(1_000.0);
const MOONLIGHT_GENESIS_VALUE: u64 = dusk(1_000.0);

const GAS_LIMIT: u64 = 0x10000000;

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

/// Instantiate the virtual machine with the transfer contract deployed, with a
/// single note carrying the `GENESIS_VALUE` owned by the given public key.
fn instantiate<Rng: RngCore + CryptoRng>(
    rng: &mut Rng,
    vm: &VM,
    phoenix_pk: &PublicKey,
    moonlight_pk: &BlsPublicKey,
) -> Session {
    let transfer_bytecode = include_bytes!(
        "../../../target/dusk/wasm64-unknown-unknown/release/transfer_contract.wasm"
    );
    let alice_bytecode = include_bytes!(
        "../../../target/dusk/wasm32-unknown-unknown/release/alice.wasm"
    );
    let bob_bytecode = include_bytes!(
        "../../../target/dusk/wasm32-unknown-unknown/release/alice.wasm"
    );

    let mut session = rusk_abi::new_genesis_session(vm);

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
            ContractData::builder().owner(OWNER).contract_id(BOB_ID),
            GAS_LIMIT,
        )
        .expect("Deploying the bob contract should succeed");

    let sender_blinder = [
        JubJubScalar::random(&mut *rng),
        JubJubScalar::random(&mut *rng),
    ];
    let genesis_note = Note::transparent(
        rng,
        phoenix_pk,
        phoenix_pk,
        PHOENIX_GENESIS_VALUE,
        sender_blinder,
    );

    // push genesis phoenix note to the contract
    session
        .call::<_, Note>(
            TRANSFER_CONTRACT,
            "push_note",
            &(0u64, genesis_note),
            GAS_LIMIT,
        )
        .expect("Pushing genesis note should succeed");

    update_root(&mut session).expect("Updating the root should succeed");

    // insert genesis moonlight account
    session
        .call::<_, ()>(
            TRANSFER_CONTRACT,
            "add_account_balance",
            &(*moonlight_pk, MOONLIGHT_GENESIS_VALUE),
            GAS_LIMIT,
        )
        .expect("Inserting genesis account should succeed");

    // sets the block height for all subsequent operations to 1
    let base = session.commit().expect("Committing should succeed");

    rusk_abi::new_session(vm, base, 1)
        .expect("Instantiating new session should succeed")
}

#[test]
fn phoenix_transfer() {
    const TRANSFER_FEE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let phoenix_sender_sk = SecretKey::random(rng);
    let phoenix_sender_pk = PublicKey::from(&phoenix_sender_sk);

    let phoenix_receiver_pk = PublicKey::from(&SecretKey::random(rng));

    let moonlight_sk = BlsSecretKey::random(rng);
    let moonlight_pk = BlsPublicKey::from(&moonlight_sk);

    let session = &mut instantiate(rng, vm, &phoenix_sender_pk, &moonlight_pk);

    let leaves = leaves_from_height(session, 0)
        .expect("Getting leaves in the given range should succeed");

    assert_eq!(leaves.len(), 1, "There should be one note in the state");

    let total_num_notes =
        num_notes(session).expect("Getting num_notes should succeed");
    assert_eq!(
        total_num_notes,
        leaves.last().expect("note to exists").note.pos() + 1,
        "num_notes should match position of last note + 1"
    );

    // create the transaction
    let gas_limit = TRANSFER_FEE;
    let gas_price = LUX;
    let input_note_pos = 0;
    let transfer_value = 42;
    let is_obfuscated = true;
    let deposit = 0;
    let contract_call = None;

    let tx = create_phoenix_transaction(
        session,
        &phoenix_sender_sk,
        &phoenix_receiver_pk,
        gas_limit,
        gas_price,
        [input_note_pos],
        transfer_value,
        is_obfuscated,
        deposit,
        contract_call,
    );

    let gas_spent = execute(session, tx).expect("Executing TX should succeed");
    update_root(session).expect("Updating the root should succeed");

    println!("EXECUTE_1_2 : {} gas", gas_spent);

    let leaves = leaves_from_height(session, 1)
        .expect("Getting the notes should succeed");
    assert_eq!(
        leaves.len(),
        3,
        "There should be three notes in the tree at this block height"
    );

    let amount_notes =
        num_notes(session).expect("Getting num_notes should succeed");
    assert_eq!(
        amount_notes,
        leaves.last().expect("note to exists").note.pos() + 1,
        "num_notes should match position of last note + 1"
    );

    let leaves = leaves_from_pos(session, input_note_pos + 1)
        .expect("Getting the notes should succeed");
    assert_eq!(
        leaves.len(),
        3,
        "There should be three notes in the tree at this block height"
    );
}

#[test]
fn moonlight_transfer() {
    const TRANSFER_VALUE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let phoenix_pk = PublicKey::from(&SecretKey::random(rng));

    let moonlight_sender_sk = BlsSecretKey::random(rng);
    let moonlight_sender_pk = BlsPublicKey::from(&moonlight_sender_sk);

    let moonlight_receiver_pk = BlsPublicKey::from(&BlsSecretKey::random(rng));

    let session = &mut instantiate(rng, vm, &phoenix_pk, &moonlight_sender_pk);

    let sender_account = account(session, &moonlight_sender_pk)
        .expect("Getting the sender account should succeed");
    let receiver_account = account(session, &moonlight_receiver_pk)
        .expect("Getting the receiver account should succeed");

    assert_eq!(
        sender_account.balance, MOONLIGHT_GENESIS_VALUE,
        "The sender account should have the genesis value"
    );
    assert_eq!(
        receiver_account.balance, 0,
        "The receiver account should be empty"
    );

    let transaction = create_moonlight_transaction(
        session,
        &moonlight_sender_sk,
        Some(moonlight_receiver_pk),
        TRANSFER_VALUE,
        0,
        GAS_LIMIT,
        LUX,
        None::<ContractExec>,
    );

    let gas_spent =
        execute(session, transaction).expect("Transaction should succeed");

    println!("MOONLIGHT TRANSFER: {} gas", gas_spent);

    let sender_account = account(session, &moonlight_sender_pk)
        .expect("Getting the sender account should succeed");
    let receiver_account = account(session, &moonlight_receiver_pk)
        .expect("Getting the receiver account should succeed");

    assert_eq!(
        sender_account.balance,
        MOONLIGHT_GENESIS_VALUE - gas_spent - TRANSFER_VALUE,
        "The sender account should decrease by the amount spent"
    );
    assert_eq!(
        receiver_account.balance, TRANSFER_VALUE,
        "The receiver account should have the transferred value"
    );
}

#[test]
fn phoenix_alice_ping() {
    const PING_FEE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let phoenix_sender_sk = SecretKey::random(rng);
    let phoenix_sender_pk = PublicKey::from(&phoenix_sender_sk);

    let moonlight_sk = BlsSecretKey::random(rng);
    let moonlight_pk = BlsPublicKey::from(&moonlight_sk);

    let session = &mut instantiate(rng, vm, &phoenix_sender_pk, &moonlight_pk);

    let leaves = leaves_from_height(session, 0)
        .expect("Getting leaves in the given range should succeed");

    assert_eq!(leaves.len(), 1, "There should be one note in the state");

    // create the transaction
    let gas_limit = PING_FEE;
    let gas_price = LUX;
    let input_note_pos = 0;
    let transfer_value = 0;
    let is_obfuscated = false;
    let deposit = 0;
    let contract_call = Some(ContractCall {
        contract: ALICE_ID.to_bytes(),
        fn_name: String::from("ping"),
        fn_args: vec![],
    });

    let tx = create_phoenix_transaction(
        session,
        &phoenix_sender_sk,
        &phoenix_sender_pk,
        gas_limit,
        gas_price,
        [input_note_pos],
        transfer_value,
        is_obfuscated,
        deposit,
        contract_call,
    );

    let gas_spent = execute(session, tx).expect("Executing TX should succeed");
    update_root(session).expect("Updating the root should succeed");

    println!("EXECUTE_PING: {} gas", gas_spent);

    let leaves = leaves_from_height(session, 1)
        .expect("Getting the notes should succeed");
    assert_eq!(
        leaves.len(),
        // since the transfer value is a transparent note with value 0 there is
        // only the change note added to the tree
        2,
        "There should be two notes in the tree after the transaction"
    );
}

#[test]
fn moonlight_alice_ping() {
    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let phoenix_pk = PublicKey::from(&SecretKey::random(rng));

    let moonlight_sk = BlsSecretKey::random(rng);
    let moonlight_pk = BlsPublicKey::from(&moonlight_sk);

    let session = &mut instantiate(rng, vm, &phoenix_pk, &moonlight_pk);

    let acc = account(session, &moonlight_pk)
        .expect("Getting the sender account should succeed");

    let contract_call = Some(ContractCall {
        contract: ALICE_ID.to_bytes(),
        fn_name: String::from("ping"),
        fn_args: vec![],
    });

    assert_eq!(
        acc.balance, MOONLIGHT_GENESIS_VALUE,
        "The account should have the genesis value"
    );

    let transaction = create_moonlight_transaction(
        session,
        &moonlight_sk,
        None,
        0,
        0,
        GAS_LIMIT,
        LUX,
        contract_call,
    );

    let gas_spent =
        execute(session, transaction).expect("Transaction should succeed");

    println!("MOONLIGHT PING: {} gas", gas_spent);

    let acc = account(session, &moonlight_pk)
        .expect("Getting the account should succeed");

    assert_eq!(
        acc.balance,
        MOONLIGHT_GENESIS_VALUE - gas_spent,
        "The account should decrease by the amount spent"
    );
}

#[test]
fn phoenix_deposit_and_withdraw() {
    const DEPOSIT_FEE: u64 = dusk(1.0);
    const WITHDRAW_FEE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let phoenix_sender_sk = SecretKey::random(rng);
    let phoenix_sender_vk = ViewKey::from(&phoenix_sender_sk);
    let phoenix_sender_pk = PublicKey::from(&phoenix_sender_sk);

    let moonlight_sk = BlsSecretKey::random(rng);
    let moonlight_pk = BlsPublicKey::from(&moonlight_sk);

    let session = &mut instantiate(rng, vm, &phoenix_sender_pk, &moonlight_pk);

    let leaves = leaves_from_height(session, 0)
        .expect("Getting leaves in the given range should succeed");

    assert_eq!(leaves.len(), 1, "There should be one note in the state");

    // create the deposit transaction
    let gas_limit = DEPOSIT_FEE;
    let gas_price = LUX;
    let input_note_pos = 0;
    let transfer_value = 0;
    let is_obfuscated = false;
    let deposit_value = PHOENIX_GENESIS_VALUE / 2;
    let contract_call = Some(ContractCall {
        contract: ALICE_ID.to_bytes(),
        fn_name: String::from("deposit"),
        fn_args: deposit_value.to_bytes().into(),
    });

    let tx = create_phoenix_transaction(
        session,
        &phoenix_sender_sk,
        &phoenix_sender_pk,
        gas_limit,
        gas_price,
        [input_note_pos],
        transfer_value,
        is_obfuscated,
        deposit_value,
        contract_call,
    );

    let gas_spent =
        execute(session, tx.clone()).expect("Executing TX should succeed");
    update_root(session).expect("Updating the root should succeed");

    println!("EXECUTE_DEPOSIT: {} gas", gas_spent);

    let leaves = leaves_from_height(session, 1)
        .expect("Getting the notes should succeed");
    assert_eq!(
        PHOENIX_GENESIS_VALUE,
        transfer_value
            + tx.payload().tx_skeleton.deposit
            + tx.payload().tx_skeleton.max_fee
            + tx.payload().tx_skeleton.outputs[1]
                .value(Some(&ViewKey::from(&phoenix_sender_sk)))
                .unwrap()
    );
    assert_eq!(
        leaves.len(),
        // since the transfer value is a transparent note with value 0 there is
        // only the change note added to the tree
        2,
        "There should be two notes in the tree at this block height"
    );

    // the alice contract has the correct balance

    let alice_balance = contract_balance(session, ALICE_ID)
        .expect("Querying the contract balance should succeed");
    assert_eq!(
        alice_balance, deposit_value,
        "Alice should have the value of the input crossover"
    );

    // start withdrawing the amount just transferred to the alice contract
    // this is done by calling the alice contract directly, which then calls the
    // transfer contract

    let input_notes = filter_notes_owned_by(
        phoenix_sender_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );

    assert_eq!(
        input_notes.len(),
        2,
        "All new notes should be owned by our view key"
    );

    let address =
        phoenix_sender_pk.gen_stealth_address(&JubJubScalar::random(&mut *rng));
    let note_sk = phoenix_sender_sk.gen_note_sk(&address);

    let withdraw = Withdraw::new(
        rng,
        &note_sk,
        ALICE_ID.to_bytes(),
        PHOENIX_GENESIS_VALUE / 2,
        WithdrawReceiver::Phoenix(address),
        WithdrawReplayToken::Phoenix(vec![
            input_notes[0].gen_nullifier(&phoenix_sender_sk),
            input_notes[1].gen_nullifier(&phoenix_sender_sk),
        ]),
    );

    let gas_limit = WITHDRAW_FEE;
    let gas_price = LUX;
    let input_notes_pos = [*input_notes[0].pos(), *input_notes[1].pos()];
    let transfer_value = 0;
    let is_obfuscated = false;
    let deposit_value = 0;
    let contract_call = Some(ContractCall {
        contract: ALICE_ID.to_bytes(),
        fn_name: String::from("withdraw"),
        fn_args: rkyv::to_bytes::<_, 1024>(&withdraw)
            .expect("should serialize Mint correctly")
            .to_vec(),
    });

    let tx = create_phoenix_transaction(
        session,
        &phoenix_sender_sk,
        &phoenix_sender_pk,
        gas_limit,
        gas_price,
        input_notes_pos.try_into().unwrap(),
        transfer_value,
        is_obfuscated,
        deposit_value,
        contract_call,
    );

    let gas_spent = execute(session, tx).expect("Executing TX should succeed");
    update_root(session).expect("Updating the root should succeed");

    println!("EXECUTE_WITHDRAW: {} gas", gas_spent);

    let alice_balance = contract_balance(session, ALICE_ID)
        .expect("Querying the contract balance should succeed");
    assert_eq!(
        alice_balance, 0,
        "Alice should have no balance after it is withdrawn"
    );
}
