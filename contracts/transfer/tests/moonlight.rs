// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::abi::{ContractError, ContractId};
use dusk_core::signatures::bls::{
    PublicKey as AccountPublicKey, SecretKey as AccountSecretKey,
};
use dusk_core::transfer::data::{ContractCall, TransactionData};
use dusk_core::transfer::moonlight::Transaction as MoonlightTransaction;
use dusk_core::transfer::phoenix::{
    Note, PublicKey as PhoenixPublicKey, SecretKey as PhoenixSecretKey,
    ViewKey as PhoenixViewKey,
};
use dusk_core::transfer::withdraw::{
    Withdraw, WithdrawReceiver, WithdrawReplayToken,
};
use dusk_core::transfer::{
    ContractToAccount, ContractToContract, Transaction, TRANSFER_CONTRACT,
};
use dusk_core::{dusk, JubJubScalar, LUX};
use dusk_vm::{execute, ContractData, ExecutionConfig, Session, VM};
use ff::Field;
use rand::rngs::StdRng;
use rand::SeedableRng;

pub mod common;
use crate::common::utils::{
    account, chain_id, contract_balance, existing_nullifiers,
    filter_notes_owned_by, leaves_from_height, owned_notes_value, update_root,
};

const MOONLIGHT_GENESIS_VALUE: u64 = dusk(1_000.0);
const MOONLIGHT_GENESIS_NONCE: u64 = 0;
const ALICE_GENESIS_VALUE: u64 = dusk(2_000.0);

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
const CHAIN_ID: u8 = 0xFA;

const NO_CONFIG: ExecutionConfig = ExecutionConfig::DEFAULT;

/// Instantiate the virtual machine with the transfer contract deployed, with a
/// moonlight account owning the `MOONLIGHT_GENESIS_VALUE` and alice and bob
/// contracts deployed with alice contract owning `ALICE_GENESIS_VALUE`.
fn instantiate(moonlight_pk: &AccountPublicKey) -> Session {
    let transfer_bytecode = include_bytes!(
        "../../../target/dusk/wasm64-unknown-unknown/release/transfer_contract.wasm"
    );
    let alice_bytecode = include_bytes!(
        "../../../target/wasm32-unknown-unknown/release/alice.wasm"
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

    // insert genesis value to moonlight account
    session
        .call::<_, ()>(
            TRANSFER_CONTRACT,
            "add_account_balance",
            &(*moonlight_pk, MOONLIGHT_GENESIS_VALUE),
            GAS_LIMIT,
        )
        .expect("Inserting genesis account should succeed");

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

    // check genesis state

    // the moonlight account is instantiated with the expected value
    let sender_account = account(&mut session, &moonlight_pk)
        .expect("Getting the sender account should succeed");
    assert_eq!(
        sender_account.balance, MOONLIGHT_GENESIS_VALUE,
        "The sender moonlight account should have its genesis value"
    );
    // the moonlight account's nonce is 1
    assert_eq!(sender_account.nonce, MOONLIGHT_GENESIS_NONCE);

    // the alice contract balance is instantiated with the expected value
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

/// Perform a simple transfer of funds between two moonlight accounts.
#[test]
fn transfer() {
    const TRANSFER_VALUE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let moonlight_sender_sk = AccountSecretKey::random(rng);
    let moonlight_sender_pk = AccountPublicKey::from(&moonlight_sender_sk);

    let moonlight_receiver_pk =
        AccountPublicKey::from(&AccountSecretKey::random(rng));

    let session = &mut instantiate(&moonlight_sender_pk);

    let transaction = Transaction::moonlight(
        &moonlight_sender_sk,
        Some(moonlight_receiver_pk),
        TRANSFER_VALUE,
        0,
        GAS_LIMIT,
        LUX,
        MOONLIGHT_GENESIS_NONCE + 1,
        CHAIN_ID,
        None::<TransactionData>,
    )
    .expect("Creating moonlight transaction should succeed");

    let gas_spent = execute(session, &transaction, &NO_CONFIG)
        .expect("Transaction should succeed")
        .gas_spent;

    println!("TRANSFER: {} gas", gas_spent);

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

/// Perform a transfer between moonlight accounts, where the left-over gas is
/// refunded to a different account than the sender.
#[test]
fn transfer_with_refund() {
    const TRANSFER_VALUE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let moonlight_sender_sk = AccountSecretKey::random(rng);
    let moonlight_sender_pk = AccountPublicKey::from(&moonlight_sender_sk);

    let moonlight_refund_pk =
        AccountPublicKey::from(&AccountSecretKey::random(rng));

    let moonlight_receiver_pk =
        AccountPublicKey::from(&AccountSecretKey::random(rng));

    let session = &mut instantiate(&moonlight_sender_pk);

    // make sure the receiver has 0 balance before the tx
    let receiver_account = account(session, &moonlight_receiver_pk)
        .expect("Getting the receiver account should succeed");
    assert_eq!(
        receiver_account.balance, 0,
        "The receiver account should be empty"
    );

    let transaction: Transaction = MoonlightTransaction::new_with_refund(
        &moonlight_sender_sk,
        &moonlight_refund_pk,
        Some(moonlight_receiver_pk),
        TRANSFER_VALUE,
        0,
        GAS_LIMIT,
        LUX,
        MOONLIGHT_GENESIS_NONCE + 1,
        CHAIN_ID,
        None::<TransactionData>,
    )
    .expect("Creating moonlight transaction should succeed")
    .into();

    let max_gas = GAS_LIMIT * LUX;
    let gas_spent = execute(session, &transaction, &NO_CONFIG)
        .expect("Transaction should succeed")
        .gas_spent;
    let gas_refund = max_gas - gas_spent;

    println!("TRANSFER WITH REFUND: {} gas", gas_spent);

    let sender_account = account(session, &moonlight_sender_pk)
        .expect("Getting the sender account should succeed");
    let refund_account = account(session, &moonlight_refund_pk)
        .expect("Getting the refund account should succeed");
    let receiver_account = account(session, &moonlight_receiver_pk)
        .expect("Getting the receiver account should succeed");

    assert_eq!(
        sender_account.balance,
        MOONLIGHT_GENESIS_VALUE - max_gas - TRANSFER_VALUE,
        "The sender account should decrease by the amount spent"
    );
    assert_eq!(
        refund_account.balance, gas_refund,
        "The sender account should decrease by the amount spent"
    );
    assert_eq!(
        receiver_account.balance, TRANSFER_VALUE,
        "The receiver account should have the transferred value"
    );
}

/// Checks if a transaction fails when the gas-price is 0.
#[test]
fn transfer_gas_fails() {
    const TRANSFER_VALUE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let moonlight_sender_sk = AccountSecretKey::random(rng);
    let moonlight_sender_pk = AccountPublicKey::from(&moonlight_sender_sk);

    let moonlight_receiver_pk =
        AccountPublicKey::from(&AccountSecretKey::random(rng));

    let session = &mut instantiate(&moonlight_sender_pk);

    // make sure the receiver has 0 balance before the tx
    let receiver_account = account(session, &moonlight_receiver_pk)
        .expect("Getting the receiver account should succeed");
    assert_eq!(
        receiver_account.balance, 0,
        "The receiver account should be empty"
    );

    let transaction = Transaction::moonlight(
        &moonlight_sender_sk,
        Some(moonlight_receiver_pk),
        TRANSFER_VALUE,
        0,
        GAS_LIMIT,
        0,
        MOONLIGHT_GENESIS_NONCE + 1,
        CHAIN_ID,
        None::<TransactionData>,
    )
    .expect("Creating moonlight transaction should succeed");

    let result = execute(session, &transaction, &NO_CONFIG);

    assert!(
        result.is_err(),
        "Transaction should fail due to zero gas price"
    );

    // Since the transaction failed, balances should remain the same
    let sender_account = account(session, &moonlight_sender_pk)
        .expect("Getting the sender account should succeed");
    let receiver_account = account(session, &moonlight_receiver_pk)
        .expect("Getting the receiver account should succeed");

    assert_eq!(
        sender_account.balance, MOONLIGHT_GENESIS_VALUE,
        "The sender account should still have the genesis value"
    );
    assert_eq!(
        receiver_account.balance, 0,
        "The receiver account should still be empty"
    );
}

/// Performs a simple contract-call.
#[test]
fn alice_ping() {
    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let moonlight_sk = AccountSecretKey::random(rng);
    let moonlight_pk = AccountPublicKey::from(&moonlight_sk);

    let session = &mut instantiate(&moonlight_pk);

    let contract_call = ContractCall::new(ALICE_ID, "ping");

    let transaction = Transaction::moonlight(
        &moonlight_sk,
        None,
        0,
        0,
        GAS_LIMIT,
        LUX,
        MOONLIGHT_GENESIS_NONCE + 1,
        CHAIN_ID,
        Some(contract_call),
    )
    .expect("Creating moonlight transaction should succeed");

    let gas_spent = execute(session, &transaction, &NO_CONFIG)
        .expect("Transaction should succeed")
        .gas_spent;

    println!("CONTRACT PING: {} gas", gas_spent);

    let moonlight_account = account(session, &moonlight_pk)
        .expect("Getting the account should succeed");

    assert_eq!(
        moonlight_account.balance,
        MOONLIGHT_GENESIS_VALUE - gas_spent,
        "The account should decrease by the amount spent"
    );
}

/// Convert moonlight DUSK into phoenix DUSK.
#[test]
fn convert_to_phoenix() {
    const CONVERSION_VALUE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let phoenix_sk = PhoenixSecretKey::random(rng);
    let phoenix_vk = PhoenixViewKey::from(&phoenix_sk);
    let phoenix_pk = PhoenixPublicKey::from(&phoenix_sk);

    let moonlight_sk = AccountSecretKey::random(rng);
    let moonlight_pk = AccountPublicKey::from(&moonlight_sk);

    let mut session = &mut instantiate(&moonlight_pk);

    // make sure that the phoenix-key doesn't own any notes yet
    let leaves = leaves_from_height(session, 0)
        .expect("getting the notes should succeed");
    let notes = filter_notes_owned_by(
        phoenix_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );
    assert_eq!(notes.len(), 0, "There should be no notes at this height");

    // generate a new note stealth-address and note-sk for the conversion
    let address =
        phoenix_pk.gen_stealth_address(&JubJubScalar::random(&mut *rng));
    let note_sk = phoenix_sk.gen_note_sk(&address);

    // the moonlight replay token
    let nonce = MOONLIGHT_GENESIS_NONCE + 1;

    // a conversion is a deposit into the transfer-contract paired with a
    // withdrawal
    let contract_call = ContractCall::new(TRANSFER_CONTRACT, "convert")
        .with_args(&Withdraw::new(
            rng,
            &note_sk,
            TRANSFER_CONTRACT,
            // set the conversion-value as a withdrawal
            CONVERSION_VALUE,
            WithdrawReceiver::Phoenix(address),
            WithdrawReplayToken::Moonlight(nonce),
        ))
        .expect("should serialize conversion correctly");

    let tx = Transaction::moonlight(
        &moonlight_sk,
        None,
        0,
        // set the conversion-value as the deposit
        CONVERSION_VALUE,
        GAS_LIMIT,
        LUX,
        nonce,
        CHAIN_ID,
        Some(contract_call),
    )
    .expect("Creating moonlight transaction should succeed");

    let gas_spent = execute(&mut session, &tx, &NO_CONFIG)
        .expect("Executing transaction should succeed")
        .gas_spent;
    update_root(session).expect("Updating the root should succeed");

    println!("CONVERT TO PHOENIX: {} gas", gas_spent);

    let moonlight_account = account(&mut session, &moonlight_pk)
        .expect("Getting account should succeed");

    assert_eq!(
        moonlight_account.balance,
        MOONLIGHT_GENESIS_VALUE - gas_spent - CONVERSION_VALUE,
        "The moonlight account should have had the conversion value subtracted along with gas spent"
    );

    let leaves = leaves_from_height(session, 1)
        .expect("getting the notes should succeed");
    let notes = filter_notes_owned_by(
        phoenix_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );
    let notes_value = owned_notes_value(phoenix_vk, &notes);

    assert_eq!(notes.len(), 1, "A new note should have been created");
    assert_eq!(
        notes_value, CONVERSION_VALUE,
        "The new note should have the conversion value",
    );
}

/// Converting phoenix DUSK into moonlight DUSK with a moonlight transaction
/// should fail.
#[test]
fn convert_to_moonlight_fails() {
    const CONVERSION_VALUE: u64 = dusk(10.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let phoenix_sk = PhoenixSecretKey::random(rng);
    let phoenix_vk = PhoenixViewKey::from(&phoenix_sk);
    let phoenix_pk = PhoenixPublicKey::from(&phoenix_sk);

    let moonlight_sk = AccountSecretKey::random(rng);
    let moonlight_pk = AccountPublicKey::from(&moonlight_sk);

    let mut session = &mut instantiate(&moonlight_pk);

    // Add a phoenix note with the conversion-value
    let value_blinder = JubJubScalar::random(&mut *rng);
    let sender_blinder = [
        JubJubScalar::random(&mut *rng),
        JubJubScalar::random(&mut *rng),
    ];
    let note = Note::obfuscated(
        rng,
        &phoenix_pk,
        &phoenix_pk,
        CONVERSION_VALUE,
        value_blinder,
        sender_blinder,
    );
    // get the nullifier for later check
    let nullifier = note.gen_nullifier(&phoenix_sk);
    // push genesis phoenix note to the contract
    session
        .call::<_, Note>(
            TRANSFER_CONTRACT,
            "push_note",
            &(0u64, note),
            GAS_LIMIT,
        )
        .expect("Pushing genesis note should succeed");

    // update the root after the notes have been inserted
    update_root(&mut session).expect("Updating the root should succeed");

    // make sure that the phoenix-key doesn't own any notes yet
    let leaves = leaves_from_height(session, 0)
        .expect("getting the notes should succeed");
    let notes = filter_notes_owned_by(
        phoenix_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );
    assert_eq!(notes.len(), 1, "There should be one note at this height");

    // a conversion is a deposit into the transfer-contract paired with a
    // withdrawal
    let contract_call = ContractCall::new(TRANSFER_CONTRACT, "convert")
        .with_args(&Withdraw::new(
            rng,
            &moonlight_sk,
            TRANSFER_CONTRACT,
            // set the conversion-value as a withdrawal
            CONVERSION_VALUE,
            WithdrawReceiver::Moonlight(moonlight_pk),
            WithdrawReplayToken::Phoenix(vec![
                notes[0].gen_nullifier(&phoenix_sk)
            ]),
        ))
        .expect("should serialize conversion correctly");

    let tx = Transaction::moonlight(
        &moonlight_sk,
        None,
        0,
        // set the conversion-value as the deposit
        CONVERSION_VALUE,
        GAS_LIMIT,
        LUX,
        MOONLIGHT_GENESIS_NONCE + 1,
        CHAIN_ID,
        Some(contract_call),
    )
    .expect("Creating moonlight transaction should succeed");

    let receipt = execute(&mut session, &tx, &NO_CONFIG)
        .expect("Executing TX should succeed");

    // check that the transaction execution panicked with the correct message
    assert!(receipt.data.is_err());
    assert_eq!(
        format!("{}", receipt.data.unwrap_err()),
        String::from("Panic: Expected Phoenix TX, found Moonlight"),
        "The attempted conversion from phoenix to moonlight when paying gas with moonlight should error"
    );
    assert_eq!(
        receipt.gas_spent,
        GAS_LIMIT * LUX,
        "The max gas should have been spent"
    );

    update_root(session).expect("Updating the root should succeed");

    println!("CONVERT TO MOONLIGHT: {} gas", receipt.gas_spent);

    let moonlight_account = account(&mut session, &moonlight_pk)
        .expect("Getting account should succeed");

    assert_eq!(
        moonlight_account.balance,
        MOONLIGHT_GENESIS_VALUE - receipt.gas_spent,
        "Since the conversion fails, the moonlight account should only have the gas-spent deducted"
    );

    let leaves = leaves_from_height(session, 1)
        .expect("getting the notes should succeed");
    assert_eq!(leaves.len(), 0, "no new leaves should have been created");

    let nullifier = vec![nullifier];
    let existing_nullifers = existing_nullifiers(session, &nullifier)
        .expect("Querrying the nullifiers should work");
    assert!(
        existing_nullifers.is_empty(),
        "the note shouldn't have been nullified"
    );
}

/// Attempts to convert moonlight DUSK into phoenix DUSK but fails due to not
/// targeting the correct contract for the conversion.
#[test]
fn convert_wrong_contract_targeted() {
    const CONVERSION_VALUE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let phoenix_sk = PhoenixSecretKey::random(rng);
    let phoenix_vk = PhoenixViewKey::from(&phoenix_sk);
    let phoenix_pk = PhoenixPublicKey::from(&phoenix_sk);

    let moonlight_sk = AccountSecretKey::random(rng);
    let moonlight_pk = AccountPublicKey::from(&moonlight_sk);

    let mut session = &mut instantiate(&moonlight_pk);

    // make sure that the phoenix-key doesn't own any notes yet
    let leaves = leaves_from_height(session, 1)
        .expect("getting the notes should succeed");
    let notes = filter_notes_owned_by(
        phoenix_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );
    assert_eq!(notes.len(), 0, "There should be no notes at this height");

    // generate a new note stealth-address and note-sk for the conversion
    let address =
        phoenix_pk.gen_stealth_address(&JubJubScalar::random(&mut *rng));
    let note_sk = phoenix_sk.gen_note_sk(&address);

    // the moonlight replay token
    let nonce = MOONLIGHT_GENESIS_NONCE + 1;

    let contract_call = ContractCall::new(TRANSFER_CONTRACT, "convert")
        .with_args(&Withdraw::new(
            rng,
            &note_sk,
            // this should be the transfer contract, but we're testing the
            // "wrong target" case
            ALICE_ID,
            CONVERSION_VALUE,
            WithdrawReceiver::Phoenix(address),
            WithdrawReplayToken::Moonlight(nonce),
        ))
        .expect("should serialize conversion correctly");

    let tx = Transaction::moonlight(
        &moonlight_sk,
        None,
        0,
        CONVERSION_VALUE,
        GAS_LIMIT,
        LUX,
        nonce,
        CHAIN_ID,
        Some(contract_call),
    )
    .expect("Creating moonlight transaction should succeed");

    let receipt = execute(&mut session, &tx, &NO_CONFIG)
        .expect("Executing transaction should succeed");
    update_root(session).expect("Updating the root should succeed");

    let res = receipt.data;
    let gas_spent = receipt.gas_spent;

    assert!(matches!(res, Err(_)), "The contract call should error");

    let moonlight_account = account(&mut session, &moonlight_pk)
        .expect("Getting account should succeed");

    assert_eq!(
        moonlight_account.balance,
        MOONLIGHT_GENESIS_VALUE - gas_spent,
        "The moonlight account should have only the gas spent subtracted"
    );

    let leaves = leaves_from_height(session, 1)
        .expect("getting the notes should succeed");
    let notes = filter_notes_owned_by(
        phoenix_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );

    assert!(notes.is_empty(), "A new note should not been created");
}

/// In this test we call Alice's `contract_to_contract` function, targeting Bob
/// as the receiver of the transfer. The gas will be paid in moonlight.
#[test]
fn contract_to_contract() {
    const TRANSFER_VALUE: u64 = ALICE_GENESIS_VALUE / 2;

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let moonlight_sk = AccountSecretKey::random(rng);
    let moonlight_pk = AccountPublicKey::from(&moonlight_sk);

    let session = &mut instantiate(&moonlight_pk);

    // make sure bob contract has no balance prior to the tx
    let bob_balance = contract_balance(session, BOB_ID)
        .expect("Querying the contract balance should succeed");
    assert_eq!(bob_balance, 0, "Bob must have an initial balance of zero");

    let contract_call = ContractCall::new(ALICE_ID, "contract_to_contract")
        .with_args(&ContractToContract {
            contract: BOB_ID,
            value: TRANSFER_VALUE,
            fn_name: String::from("recv_transfer"),
            data: vec![],
        })
        .expect("Serializing should succeed");

    let transaction = Transaction::moonlight(
        &moonlight_sk,
        None,
        0,
        0,
        GAS_LIMIT,
        LUX,
        MOONLIGHT_GENESIS_NONCE + 1,
        CHAIN_ID,
        Some(contract_call),
    )
    .expect("Creating moonlight transaction should succeed");

    let receipt = execute(session, &transaction, &NO_CONFIG)
        .expect("Transaction should succeed");
    let gas_spent = receipt.gas_spent;

    println!("SEND TO CONTRACT: {:?}", receipt.data);
    println!("SEND TO CONTRACT: {gas_spent} gas");

    let moonlight_account = account(session, &moonlight_pk)
        .expect("Getting the account should succeed");
    let alice_balance = contract_balance(session, ALICE_ID)
        .expect("Querying the contract balance should succeed");
    let bob_balance = contract_balance(session, BOB_ID)
        .expect("Querying the contract balance should succeed");

    assert_eq!(
        moonlight_account.balance,
        MOONLIGHT_GENESIS_VALUE - gas_spent,
        "The account should have decreased by the gas spent"
    );
    assert_eq!(
        alice_balance,
        ALICE_GENESIS_VALUE - TRANSFER_VALUE,
        "Alice's balance should have decreased by the transfer value"
    );
    assert_eq!(
        bob_balance, TRANSFER_VALUE,
        "Bob's balance must have increased by the transfer value"
    );
}

/// In this test we call the Alice contract to trigger a transfer of funds into
/// a moonlight account, the gas will be paid with moonlight.
#[test]
fn contract_to_account() {
    const TRANSFER_VALUE: u64 = ALICE_GENESIS_VALUE / 2;

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let moonlight_sk = AccountSecretKey::random(rng);
    let moonlight_pk = AccountPublicKey::from(&moonlight_sk);

    let session = &mut instantiate(&moonlight_pk);

    let contract_call = ContractCall::new(ALICE_ID, "contract_to_account")
        .with_args(&ContractToAccount {
            account: moonlight_pk,
            value: TRANSFER_VALUE,
        })
        .expect("Serializing should succeed");

    let transaction = Transaction::moonlight(
        &moonlight_sk,
        None,
        0,
        0,
        GAS_LIMIT,
        LUX,
        MOONLIGHT_GENESIS_NONCE + 1,
        CHAIN_ID,
        Some(contract_call),
    )
    .expect("Creating moonlight transaction should succeed");

    let receipt = execute(session, &transaction, &NO_CONFIG)
        .expect("Transaction should succeed");
    let gas_spent = receipt.gas_spent;

    println!("SEND TO ACCOUNT: {:?}", receipt.data);
    println!("SEND TO ACCOUNT: {gas_spent} gas");

    let moonlight_account = account(session, &moonlight_pk)
        .expect("Getting the account should succeed");
    let alice_balance = contract_balance(session, ALICE_ID)
        .expect("Querying the contract balance should succeed");

    assert_eq!(
        moonlight_account.balance,
        MOONLIGHT_GENESIS_VALUE
            - gas_spent
            + TRANSFER_VALUE,
        "The account's balance should have decreased by the spent gas and increased by the transfer value"
    );
    assert_eq!(
        alice_balance,
        ALICE_GENESIS_VALUE - TRANSFER_VALUE,
        "Alice's balance should have decreased by the transfer value"
    );
}

/// In this test we try to transfer some Dusk from a contract to an account,
/// when the contract doesn't have sufficient funds.
#[test]
fn contract_to_account_insufficient_funds() {
    // Transfer value larger than DEPOSIT
    const TRANSFER_VALUE: u64 = ALICE_GENESIS_VALUE + 42;

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let moonlight_sk = AccountSecretKey::random(rng);
    let moonlight_pk = AccountPublicKey::from(&moonlight_sk);

    let session = &mut instantiate(&moonlight_pk);

    let contract_call = ContractCall::new(ALICE_ID, "contract_to_account")
        .with_args(&ContractToAccount {
            account: moonlight_pk,
            value: TRANSFER_VALUE,
        })
        .expect("Serializing should succeed");

    let transaction = Transaction::moonlight(
        &moonlight_sk,
        None,
        0,
        0,
        GAS_LIMIT,
        LUX,
        MOONLIGHT_GENESIS_NONCE + 1,
        CHAIN_ID,
        Some(contract_call),
    )
    .expect("Creating moonlight transaction should succeed");

    let receipt = execute(session, &transaction, &NO_CONFIG)
        .expect("Transaction should succeed");
    let gas_spent = receipt.gas_spent;

    println!("SEND TO ACCOUNT (insufficient funds): {:?}", receipt.data);
    println!("SEND TO ACCOUNT (insufficient funds): {gas_spent} gas");

    let moonlight_account = account(session, &moonlight_pk)
        .expect("Getting the account should succeed");
    let alice_balance = contract_balance(session, ALICE_ID)
        .expect("Querying the contract balance should succeed");

    assert!(
        matches!(receipt.data, Err(_)),
        "Alice should error because the transfer contract panics"
    );
    assert_eq!(
        gas_spent,
        GAS_LIMIT * LUX,
        "Due to the panic, the max gas should have been spent"
    );
    assert_eq!(
        moonlight_account.balance,
        MOONLIGHT_GENESIS_VALUE - gas_spent,
        "The account's balance should decrease by the gas spent"
    );
    assert_eq!(
        alice_balance, ALICE_GENESIS_VALUE,
        "Alice's balance should be unchanged"
    );
}

/// In this test we try to call the function directly - i.e. not initiated by a
/// contract, but by the transaction itself.
#[test]
fn contract_to_account_direct_call() {
    const TRANSFER_VALUE: u64 = MOONLIGHT_GENESIS_VALUE / 2;

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let moonlight_sk = AccountSecretKey::random(rng);
    let moonlight_pk = AccountPublicKey::from(&moonlight_sk);

    let session = &mut instantiate(&moonlight_pk);

    let contract_call =
        ContractCall::new(TRANSFER_CONTRACT, "contract_to_account")
            // calling the transfer-contract directly here instead of alice,
            // should cause a panic
            .with_args(&ContractToAccount {
                account: moonlight_pk,
                value: TRANSFER_VALUE,
            })
            .expect("Serializing should succeed");

    let transaction = Transaction::moonlight(
        &moonlight_sk,
        None,
        0,
        0,
        GAS_LIMIT,
        LUX,
        MOONLIGHT_GENESIS_NONCE + 1,
        CHAIN_ID,
        Some(contract_call),
    )
    .expect("Creating moonlight transaction should succeed");

    let receipt = execute(session, &transaction, &NO_CONFIG)
        .expect("Transaction should succeed");
    let gas_spent = receipt.gas_spent;

    println!(
        "SEND TO ACCOUNT (transfer-contract targeted): {:?}",
        receipt.data
    );
    println!("SEND TO ACCOUNT (transfer-contract targeted): {gas_spent} gas");

    let moonlight_account = account(session, &moonlight_pk)
        .expect("Getting the account should succeed");
    let alice_balance = contract_balance(session, ALICE_ID)
        .expect("Querying the contract balance should succeed");

    assert!(
        matches!(receipt.data, Err(ContractError::Panic(_))),
        "The transfer contract should panic on a direct call"
    );
    assert_eq!(
        gas_spent,
        GAS_LIMIT * LUX,
        "Due to the panic, the max gas should have been spent"
    );
    assert_eq!(
        moonlight_account.balance,
        MOONLIGHT_GENESIS_VALUE - gas_spent,
        "The account should decrease by the amount spent"
    );
    assert_eq!(
        alice_balance, ALICE_GENESIS_VALUE,
        "Alice's balance should be unchanged"
    );
}
