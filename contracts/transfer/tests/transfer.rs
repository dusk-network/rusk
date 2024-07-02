// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod common;

use crate::common::{
    contract_balance, create_transaction, execute, filter_notes_owned_by,
    leaves_from_height, leaves_from_pos, num_notes, update_root,
};

use dusk_bytes::Serializable;
use ff::Field;
use rand::rngs::StdRng;
use rand::{CryptoRng, RngCore, SeedableRng};

use execution_core::{
    transfer::{ContractCall, Mint},
    BlsPublicKey, BlsScalar, BlsSecretKey, JubJubScalar, Note, PublicKey,
    SecretKey, ViewKey,
};
use rusk_abi::dusk::{dusk, LUX};
use rusk_abi::{ContractData, ContractId, Session, TRANSFER_CONTRACT, VM};

const GENESIS_VALUE: u64 = dusk(1_000.0);
const POINT_LIMIT: u64 = 0x10000000;

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
    pk: &PublicKey,
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
            POINT_LIMIT,
        )
        .expect("Deploying the transfer contract should succeed");

    session
        .deploy(
            alice_bytecode,
            ContractData::builder().owner(OWNER).contract_id(ALICE_ID),
            POINT_LIMIT,
        )
        .expect("Deploying the alice contract should succeed");

    session
        .deploy(
            bob_bytecode,
            ContractData::builder().owner(OWNER).contract_id(BOB_ID),
            POINT_LIMIT,
        )
        .expect("Deploying the bob contract should succeed");

    let sender_blinder = [
        JubJubScalar::random(&mut *rng),
        JubJubScalar::random(&mut *rng),
    ];
    let genesis_note =
        Note::transparent(rng, pk, pk, GENESIS_VALUE, sender_blinder);

    // push genesis note to the contract
    session
        .call::<_, Note>(
            TRANSFER_CONTRACT,
            "push_note",
            &(0u64, genesis_note),
            POINT_LIMIT,
        )
        .expect("Pushing genesis note should succeed");

    update_root(&mut session).expect("Updating the root should succeed");

    // sets the block height for all subsequent operations to 1
    let base = session.commit().expect("Committing should succeed");

    rusk_abi::new_session(vm, base, 1)
        .expect("Instantiating new session should succeed")
}

#[test]
fn transfer() {
    const TRANSFER_FEE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let sender_sk = SecretKey::random(rng);
    let sender_pk = PublicKey::from(&sender_sk);

    let receiver_pk = PublicKey::from(&SecretKey::random(rng));

    let session = &mut instantiate(rng, vm, &sender_pk);

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

    let tx = create_transaction(
        session,
        &sender_sk,
        &receiver_pk,
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
fn alice_ping() {
    const PING_FEE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let sender_sk = SecretKey::random(rng);
    let sender_pk = PublicKey::from(&sender_sk);

    let session = &mut instantiate(rng, vm, &sender_pk);

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

    let tx = create_transaction(
        session,
        &sender_sk,
        &sender_pk,
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
fn deposit_and_withdraw() {
    const DEPOSIT_FEE: u64 = dusk(1.0);
    const WITHDRAW_FEE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let sender_sk = SecretKey::random(rng);
    let sender_vk = ViewKey::from(&sender_sk);
    let sender_pk = PublicKey::from(&sender_sk);

    let session = &mut instantiate(rng, vm, &sender_pk);

    let leaves = leaves_from_height(session, 0)
        .expect("Getting leaves in the given range should succeed");

    assert_eq!(leaves.len(), 1, "There should be one note in the state");

    // create the deposit transaction
    let gas_limit = DEPOSIT_FEE;
    let gas_price = LUX;
    let input_note_pos = 0;
    let transfer_value = 0;
    let is_obfuscated = false;
    let deposit_value = GENESIS_VALUE / 2;
    let contract_call = Some(ContractCall {
        contract: ALICE_ID.to_bytes(),
        fn_name: String::from("deposit"),
        fn_args: deposit_value.to_bytes().into(),
    });

    let tx = create_transaction(
        session,
        &sender_sk,
        &sender_pk,
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
        GENESIS_VALUE,
        transfer_value
            + tx.payload().tx_skeleton.deposit
            + tx.payload().tx_skeleton.max_fee
            + tx.payload().tx_skeleton().outputs[1]
                .value(Some(&ViewKey::from(&sender_sk)))
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
        sender_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );

    assert_eq!(
        input_notes.len(),
        2,
        "All new notes should be owned by our view key"
    );
    let alice_user_account =
        BlsPublicKey::from(&BlsSecretKey::from(BlsScalar::from(42)));
    let mint = Mint {
        value: (GENESIS_VALUE / 2),
        address: sender_pk
            .gen_stealth_address(&JubJubScalar::random(&mut *rng)),
        sender: alice_user_account,
    };

    let gas_limit = WITHDRAW_FEE;
    let gas_price = LUX;
    let input_notes_pos = [*input_notes[0].pos(), *input_notes[1].pos()];
    let transfer_value = 0;
    let is_obfuscated = false;
    let deposit_value = 0;
    let contract_call = Some(ContractCall {
        contract: ALICE_ID.to_bytes(),
        fn_name: String::from("withdraw"),
        fn_args: rkyv::to_bytes::<_, 1024>(&mint)
            .expect("should serialize Mint correctly")
            .to_vec(),
    });

    let tx = create_transaction(
        session,
        &sender_sk,
        &sender_pk,
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
