// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg(feature = "serde")]

use serde::{Deserialize, Serialize};

use dusk_core::signatures::bls::{
    PublicKey as AccountPublicKey, SecretKey as AccountSecretKey,
};
use dusk_core::transfer::data::{
    ContractBytecode, ContractCall, ContractDeploy, NotLegacyTransactionData,
    TransactionData,
};
use dusk_core::transfer::moonlight::{Fee, Payload};
use dusk_core::transfer::phoenix::{
    Note, NoteTreeItem, NotesTree, Prove, PublicKey as PhoenixPublicKey,
    SecretKey as PhoenixSecretKey, TxCircuitVec,
};

use dusk_core::transfer::Transaction;
use dusk_core::{BlsScalar, Error, JubJubScalar};
use ff::Field;
use rand::rngs::StdRng;
use rand::{CryptoRng, Rng, RngCore, SeedableRng};

const CHAIN_ID: u8 = 0xFA;

struct TxCircuitVecProver();

// use the serialized TxCircuitVec as proof. This way that serialization is also
// tested.
impl Prove for TxCircuitVecProver {
    fn prove(&self, tx_circuit_vec_bytes: &[u8]) -> Result<Vec<u8>, Error> {
        Ok(TxCircuitVec::from_slice(tx_circuit_vec_bytes)
            .expect("serialization should be ok")
            .to_var_bytes()
            .to_vec())
    }
}

fn new_phoenix_tx<R: RngCore + CryptoRng>(
    rng: &mut R,
    data: Option<TransactionData>,
) -> Transaction {
    // generate the keys
    let sender_sk = PhoenixSecretKey::random(rng);
    let sender_pk = PhoenixPublicKey::from(&sender_sk);
    let refund_pk = &sender_pk;

    let receiver_pk = PhoenixPublicKey::from(&PhoenixSecretKey::random(rng));
    let value_blinder = JubJubScalar::random(&mut *rng);
    let sender_blinder = [
        JubJubScalar::random(&mut *rng),
        JubJubScalar::random(&mut *rng),
    ];

    // create the input notes and their merkle openings
    let mut input_0 = Note::obfuscated(
        rng,
        &sender_pk,
        &sender_pk,
        42,
        value_blinder,
        sender_blinder,
    );
    input_0.set_pos(0);
    let mut input_1 = Note::obfuscated(
        rng,
        &sender_pk,
        &sender_pk,
        8,
        value_blinder,
        sender_blinder,
    );
    input_1.set_pos(1);
    let mut input_2 = Note::obfuscated(
        rng,
        &receiver_pk,
        &sender_pk,
        1000000,
        value_blinder,
        sender_blinder,
    );
    input_2.set_pos(2);
    let notes = vec![input_0, input_1, input_2];

    let mut notes_tree = NotesTree::new();
    for note in notes.iter() {
        let item = NoteTreeItem {
            hash: note.hash(),
            data: (),
        };
        notes_tree.insert(*note.pos(), item);
    }

    let mut inputs = Vec::new();
    for note in notes {
        let opening = notes_tree
            .opening(*note.pos())
            .expect("The note should was added at the given position");
        inputs.push((note, opening));
    }

    // set the remaining parameter
    let transfer_value = 25;
    let obfuscated_transaction = true;
    let root = BlsScalar::from(123);
    let deposit = 10;
    let gas_limit = 50;
    let gas_price = 1;

    Transaction::phoenix(
        rng,
        &sender_sk,
        refund_pk,
        &receiver_pk,
        inputs,
        root,
        transfer_value,
        obfuscated_transaction,
        deposit,
        gas_limit,
        gas_price,
        CHAIN_ID,
        data,
        &TxCircuitVecProver(),
    )
    .expect("transaction generation should work")
}

fn new_moonlight_tx<R: RngCore + CryptoRng>(
    rng: &mut R,
    data: Option<TransactionData>,
) -> Transaction {
    let sender_sk = AccountSecretKey::random(rng);
    let receiver_pk =
        Some(AccountPublicKey::from(&AccountSecretKey::random(rng)));

    let value: u64 = rng.gen();
    let deposit: u64 = rng.gen();
    let gas_limit: u64 = rng.gen();
    let gas_price: u64 = rng.gen();
    let nonce: u64 = rng.gen();

    Transaction::moonlight(
        &sender_sk,
        receiver_pk,
        value,
        deposit,
        gas_limit,
        gas_price,
        nonce,
        CHAIN_ID,
        data,
    )
    .expect("transaction generation should work")
}

#[test]
fn phoenix() -> Result<(), Error> {
    let mut rng = StdRng::seed_from_u64(42);

    let transaction = new_phoenix_tx(&mut rng, None);

    let transaction_bytes = transaction.to_var_bytes();
    let deserialized = Transaction::from_slice(&transaction_bytes)?;

    assert_eq!(transaction, deserialized);

    Ok(())
}

#[test]
fn phoenix_with_call() -> Result<(), Error> {
    let mut rng = StdRng::seed_from_u64(42);

    // build the contract call
    let mut contract = [0; 32];
    rng.fill_bytes(&mut contract);

    let mut fn_args = vec![0; 100];
    rng.fill_bytes(&mut fn_args);

    let call = ContractCall::new(contract, "deposit").with_raw_args(fn_args);

    let transaction =
        new_phoenix_tx(&mut rng, Some(TransactionData::Call(call)));

    let transaction_bytes = transaction.to_var_bytes();
    let deserialized = Transaction::from_slice(&transaction_bytes)?;

    assert_eq!(transaction, deserialized);

    Ok(())
}

#[test]
fn phoenix_with_deploy() -> Result<(), Error> {
    let mut rng = StdRng::seed_from_u64(42);

    // build a contract deployment
    let mut hash = [0; 32];
    rng.fill_bytes(&mut hash);
    let mut bytes = vec![0; 100];
    rng.fill_bytes(&mut bytes);
    let bytecode = ContractBytecode { hash, bytes };

    let mut owner = [0; 32].to_vec();
    rng.fill_bytes(&mut owner);

    let mut init_args = vec![0; 20];
    rng.fill_bytes(&mut init_args);

    let nonce = rng.next_u64();

    let deploy = ContractDeploy {
        bytecode,
        owner,
        init_args: Some(init_args),
        nonce,
    };

    let transaction =
        new_phoenix_tx(&mut rng, Some(TransactionData::Deploy(deploy)));

    let transaction_bytes = transaction.to_var_bytes();
    let deserialized = Transaction::from_slice(&transaction_bytes)?;

    assert_eq!(transaction, deserialized);

    Ok(())
}

#[test]
fn phoenix_with_memo() -> Result<(), Error> {
    let mut rng = StdRng::seed_from_u64(42);

    // build a contract deployment
    let mut hash = [0; 32];
    rng.fill_bytes(&mut hash);
    let mut bytes = vec![0; 100];
    rng.fill_bytes(&mut bytes);

    let mut owner = [0; 32].to_vec();
    rng.fill_bytes(&mut owner);

    let mut init_args = vec![0; 20];
    rng.fill_bytes(&mut init_args);

    let memo = vec![1u8; 512];

    let transaction =
        new_phoenix_tx(&mut rng, Some(TransactionData::Memo(memo)));

    let transaction_bytes = transaction.to_var_bytes();
    let deserialized = Transaction::from_slice(&transaction_bytes)?;

    assert_eq!(transaction, deserialized);

    Ok(())
}

#[test]
fn moonlight() -> Result<(), Error> {
    let mut rng = StdRng::seed_from_u64(42);

    let transaction = new_moonlight_tx(&mut rng, None);

    let transaction_bytes = transaction.to_var_bytes();
    let deserialized = Transaction::from_slice(&transaction_bytes)?;

    assert_eq!(transaction, deserialized);

    Ok(())
}

#[test]
fn moonlight_with_call() -> Result<(), Error> {
    let mut rng = StdRng::seed_from_u64(42);

    // build the contract call
    let mut contract = [0; 32];
    rng.fill_bytes(&mut contract);

    let mut fn_args = vec![0; 100];
    rng.fill_bytes(&mut fn_args);

    let call = ContractCall::new(contract, "deposit").with_raw_args(fn_args);

    let transaction =
        new_moonlight_tx(&mut rng, Some(TransactionData::Call(call)));

    let transaction_bytes = transaction.to_var_bytes();
    let deserialized = Transaction::from_slice(&transaction_bytes)?;

    assert_eq!(transaction, deserialized);

    Ok(())
}

#[test]
fn moonlight_with_deploy() -> Result<(), Error> {
    let mut rng = StdRng::seed_from_u64(42);

    let mut hash = [0; 32];
    rng.fill_bytes(&mut hash);
    let mut bytes = vec![0; 100];
    rng.fill_bytes(&mut bytes);
    let bytecode = ContractBytecode { hash, bytes };

    let mut owner = [0; 32].to_vec();
    rng.fill_bytes(&mut owner);

    let mut init_args = vec![0; 20];
    rng.fill_bytes(&mut init_args);

    let nonce = rng.next_u64();

    let deploy = ContractDeploy {
        bytecode,
        owner,
        init_args: Some(init_args),
        nonce,
    };

    let transaction =
        new_moonlight_tx(&mut rng, Some(TransactionData::Deploy(deploy)));

    let transaction_bytes = transaction.to_var_bytes();
    let deserialized = Transaction::from_slice(&transaction_bytes)?;

    assert_eq!(transaction, deserialized);

    Ok(())
}

#[test]
fn moonlight_with_memo() -> Result<(), Error> {
    let mut rng = StdRng::seed_from_u64(42);

    let mut hash = [0; 32];
    rng.fill_bytes(&mut hash);
    let mut bytes = vec![0; 100];
    rng.fill_bytes(&mut bytes);

    let mut owner = [0; 32].to_vec();
    rng.fill_bytes(&mut owner);

    let mut init_args = vec![0; 20];
    rng.fill_bytes(&mut init_args);

    let memo = vec![1u8; 512];

    let transaction =
        new_moonlight_tx(&mut rng, Some(TransactionData::Memo(memo)));

    let transaction_bytes = transaction.to_var_bytes();
    let deserialized = Transaction::from_slice(&transaction_bytes)?;

    assert_eq!(transaction, deserialized);

    Ok(())
}

#[test]
fn nonsense_bytes_fails() -> Result<(), Error> {
    let mut data = [0u8; 2 ^ 16];
    for exp in 3..16 {
        rand::thread_rng().fill_bytes(&mut data[..2 ^ exp]);
        let transaction_bytes = data.to_vec();
        Transaction::from_slice(&transaction_bytes)
            .expect_err("deserialization should fail");
    }
    Ok(())
}

// Test to ensure that the signature message generated by `signature_message`
// and `new_signature_message` methods are equivalent for a given payload.
#[test]
fn signature_message() -> Result<(), Error> {
    let mut rng = StdRng::seed_from_u64(42);
    // Generate a random secret key and corresponding public key
    let sender_sk = AccountSecretKey::random(&mut rng);
    let sender_pk = AccountPublicKey::from(&sender_sk);

    let hashes = vec![1, 2, 3, 4];

    // Create a sample payload with Memo
    let payload_memo = Payload {
        chain_id: 1,
        sender: sender_pk,
        receiver: sender_pk,
        value: 100,
        deposit: 0,
        fee: Fee {
            gas_limit: 21000,
            gas_price: 1,
            refund_address: sender_pk,
        },
        nonce: 42,
        data: Some(TransactionData::Memo(hashes.clone())),
    };

    // Both methods should produce the same signature message
    let msg_memo = payload_memo.signature_message();

    // // Also test with TransactionData::Blob (should be treated as Memo)
    let payload_blob = Payload {
        chain_id: 1,
        sender: sender_pk,
        receiver: sender_pk,
        value: 100,
        deposit: 0,
        fee: Fee {
            gas_limit: 21000,
            gas_price: 1,
            refund_address: sender_pk,
        },
        nonce: 42,
        data: Some(NotLegacyTransactionData::Blob(hashes.clone(), None)),
    };

    // signature_message() will panic or todo! for Blob, so only test
    // new signature_message
    let msg_blob = payload_blob.signature_message();

    assert_eq!(msg_blob, msg_memo, "Signature messages do not match");

    // Serialize and deserialize the payloads to ensure consistency

    // Memo-payload
    let encoded_memo = payload_memo.to_var_bytes();
    let deserialized_memo = Payload::<TransactionData>::from_slice(&encoded_memo)?;

    let msg_memo2 = deserialized_memo.signature_message();

    // Blob-payload
    let encoded_blob = payload_blob.to_var_bytes();
    let deserialized_blob = Payload::<NotLegacyTransactionData>::from_slice(&encoded_blob)?;
    let msg_blob2 = deserialized_blob.signature_message();

    assert_eq!(
        msg_memo2, msg_blob2,
        "Signature messages differ after serialization"
    );


    Ok(())
}
