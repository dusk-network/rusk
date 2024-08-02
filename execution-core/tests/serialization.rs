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
        moonlight::{
            Payload as MoonlightPayload, Transaction as MoonlightTransaction,
        },
        phoenix::{
            Fee, Note, Payload as PhoenixPayload, PublicKey, SecretKey,
            Transaction as PhoenixTransaction, TxSkeleton,
        },
        Transaction,
    },
};
use ff::Field;
use rand::rngs::StdRng;
use rand::{CryptoRng, Rng, RngCore, SeedableRng};

fn new_phoenix_tx<R: RngCore + CryptoRng>(
    rng: &mut R,
    exec: Option<ContractExec>,
) -> Transaction {
    // set the general parameters
    let sender_pk = PublicKey::from(&SecretKey::random(rng));
    let receiver_pk = PublicKey::from(&SecretKey::random(rng));

    let gas_limit = 500;
    let gas_price = 42;

    // build the tx-skeleton
    let value = 25;
    let value_blinder = JubJubScalar::random(&mut *rng);
    let sender_blinder = [
        JubJubScalar::random(&mut *rng),
        JubJubScalar::random(&mut *rng),
    ];
    let note = Note::obfuscated(
        rng,
        &sender_pk,
        &receiver_pk,
        value,
        value_blinder,
        sender_blinder,
    );

    let root = BlsScalar::from(123);
    let nullifiers = vec![
        BlsScalar::from(456),
        BlsScalar::from(789),
        BlsScalar::from(6583),
        BlsScalar::from(98978542),
    ];
    let outputs = [note.clone(), note];
    let max_fee = gas_limit * gas_price;
    let deposit = 10;

    let tx_skeleton = TxSkeleton {
        root,
        nullifiers,
        outputs,
        max_fee,
        deposit,
    };

    // build the fee
    let fee = Fee::new(rng, &sender_pk, gas_limit, gas_price);

    // build the payload
    let payload = PhoenixPayload {
        tx_skeleton,
        fee,
        exec,
    };

    // set a random proof
    let proof = [42; 42].to_vec();

    PhoenixTransaction::new(payload, proof).into()
}

fn new_moonlight_tx<R: RngCore + CryptoRng>(
    rng: &mut R,
    exec: Option<ContractExec>,
) -> Transaction {
    let sk = AccountSecretKey::random(rng);
    let pk = AccountPublicKey::from(&sk);

    let payload = MoonlightPayload {
        from: pk,
        to: None,
        value: rng.gen(),
        deposit: rng.gen(),
        gas_limit: rng.gen(),
        gas_price: rng.gen(),
        nonce: rng.gen(),
        exec,
    };

    let msg = payload.to_hash_input_bytes();
    let signature = sk.sign(&msg);

    MoonlightTransaction::new(payload, signature).into()
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
