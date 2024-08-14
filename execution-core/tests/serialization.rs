// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381::BlsScalar;
use dusk_bytes::Error;
use dusk_jubjub::JubJubScalar;
use execution_core::{
    signatures::bls::{
        PublicKey as AccountPublicKey, SecretKey as AccountSecretKey,
    },
    transfer::{
        contract_exec::{
            ContractBytecode, ContractCall, ContractDeploy, ContractExec,
        },
        phoenix::{
            Note, Prove, PublicKey as PhoenixPublicKey,
            SecretKey as PhoenixSecretKey, TxCircuitVec, NOTES_TREE_DEPTH,
        },
        Transaction,
    },
};
use ff::Field;
use poseidon_merkle::{Item, Tree};
use rand::rngs::StdRng;
use rand::{CryptoRng, Rng, RngCore, SeedableRng};

struct RandomTestProver();

impl Prove for RandomTestProver {
    type Error = ();

    fn prove(_circuit: TxCircuitVec) -> Result<Vec<u8>, Self::Error> {
        let mut proof = vec![0; 5_000];
        let mut rng = StdRng::seed_from_u64(42);
        rng.fill_bytes(&mut proof);

        Ok(proof)
    }
}

fn new_phoenix_tx<R: RngCore + CryptoRng>(
    rng: &mut R,
    exec: Option<ContractExec>,
) -> Transaction {
    // generate the keys
    let sender_sk = PhoenixSecretKey::random(rng);
    let sender_pk = PhoenixPublicKey::from(&sender_sk);
    let change_pk = &sender_pk;

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

    let mut notes_tree = Tree::<(), NOTES_TREE_DEPTH>::new();
    for note in notes.iter() {
        let item = Item {
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

    Transaction::phoenix::<R, RandomTestProver>(
        rng,
        &sender_sk,
        change_pk,
        &receiver_pk,
        inputs,
        root,
        transfer_value,
        obfuscated_transaction,
        deposit,
        gas_limit,
        gas_price,
        exec,
    )
}

fn new_moonlight_tx<R: RngCore + CryptoRng>(
    rng: &mut R,
    exec: Option<ContractExec>,
) -> Transaction {
    let from_sk = AccountSecretKey::random(rng);
    let to_account =
        Some(AccountPublicKey::from(&AccountSecretKey::random(rng)));

    let value: u64 = rng.gen();
    let deposit: u64 = rng.gen();
    let gas_limit: u64 = rng.gen();
    let gas_price: u64 = rng.gen();
    let nonce: u64 = rng.gen();

    Transaction::moonlight(
        &from_sk, to_account, value, deposit, gas_limit, gas_price, nonce, exec,
    )
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

    let call = ContractCall {
        contract: contract.into(),
        fn_name: String::from("deposit"),
        fn_args,
    };

    let transaction = new_phoenix_tx(&mut rng, Some(ContractExec::Call(call)));

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

    let mut constructor_args = vec![0; 20];
    rng.fill_bytes(&mut constructor_args);

    let nonce = rng.next_u64();

    let deploy = ContractDeploy {
        bytecode,
        owner,
        constructor_args: Some(constructor_args),
        nonce,
    };

    let transaction =
        new_phoenix_tx(&mut rng, Some(ContractExec::Deploy(deploy)));

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

    let call = ContractCall {
        contract: contract.into(),
        fn_name: String::from("deposit"),
        fn_args,
    };

    let transaction =
        new_moonlight_tx(&mut rng, Some(ContractExec::Call(call)));

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

    let mut constructor_args = vec![0; 20];
    rng.fill_bytes(&mut constructor_args);

    let nonce = rng.next_u64();

    let deploy = ContractDeploy {
        bytecode,
        owner,
        constructor_args: Some(constructor_args),
        nonce,
    };

    let transaction =
        new_moonlight_tx(&mut rng, Some(ContractExec::Deploy(deploy)));

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
