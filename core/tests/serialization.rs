// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::signatures::bls::{
    PublicKey as AccountPublicKey, SecretKey as AccountSecretKey,
};
use dusk_core::transfer::data::{
    ContractBytecode, ContractCall, ContractDeploy, TransactionData,
};
use dusk_core::transfer::phoenix::{
    Note, NoteTreeItem, NotesTree, Prove, PublicKey as PhoenixPublicKey,
    SecretKey as PhoenixSecretKey, TxCircuitVec,
};
use dusk_core::transfer::{Transaction, TransactionFormat};
use dusk_core::{BlsScalar, Error, JubJubScalar};
use ff::Field;
use rand::rngs::StdRng;
use rand::{CryptoRng, Rng, RngCore, SeedableRng};

const CHAIN_ID: u8 = 0xFA;

struct TxCircuitVecProver();

#[derive(Clone, Copy)]
enum TxFamily {
    Phoenix,
    Moonlight,
}

#[derive(Clone, Copy)]
enum DataCase {
    Plain,
    Call,
    Deploy,
    Memo,
}

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

    let value: u64 = rng.r#gen();
    let deposit: u64 = rng.r#gen();
    let gas_limit: u64 = rng.r#gen();
    let gas_price: u64 = rng.r#gen();
    let nonce: u64 = rng.r#gen();

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

fn assert_legacy_roundtrip(transaction: Transaction) -> Result<(), Error> {
    let transaction_bytes = transaction.to_var_bytes();
    let deserialized = Transaction::from_slice(&transaction_bytes)?;

    assert_eq!(transaction, deserialized);

    Ok(())
}

fn assert_roundtrip_for_format(
    transaction: Transaction,
    format: TransactionFormat,
) -> Result<(), Error> {
    let transaction_bytes = transaction.encode_for_format(format);
    let decoded = Transaction::decode_with_format(format, &transaction_bytes)?;

    assert_eq!(decoded.transaction, transaction);
    assert_eq!(decoded.format, format);

    Ok(())
}

fn assert_decode_rejected(
    transaction: Transaction,
    encoded_format: TransactionFormat,
    decode_format: TransactionFormat,
) {
    let transaction_bytes = transaction.encode_for_format(encoded_format);
    let err =
        Transaction::decode_with_format(decode_format, &transaction_bytes)
            .unwrap_err();
    assert_eq!(err, dusk_bytes::Error::InvalidData);
}

fn make_data_case<R: RngCore>(
    rng: &mut R,
    case: DataCase,
) -> Option<TransactionData> {
    match case {
        DataCase::Plain => None,
        DataCase::Call => Some(TransactionData::Call(sample_call(rng))),
        DataCase::Deploy => Some(TransactionData::Deploy(sample_deploy(rng))),
        DataCase::Memo => Some(TransactionData::Memo(vec![1u8; 512])),
    }
}

fn new_transaction<R: RngCore + CryptoRng>(
    family: TxFamily,
    rng: &mut R,
    case: DataCase,
) -> Transaction {
    let data = make_data_case(rng, case);

    match family {
        TxFamily::Phoenix => new_phoenix_tx(rng, data),
        TxFamily::Moonlight => new_moonlight_tx(rng, data),
    }
}

fn assert_roundtrip_matrix(
    format: Option<TransactionFormat>,
) -> Result<(), Error> {
    const FAMILIES: [TxFamily; 2] = [TxFamily::Phoenix, TxFamily::Moonlight];
    const CASES: [DataCase; 4] = [
        DataCase::Plain,
        DataCase::Call,
        DataCase::Deploy,
        DataCase::Memo,
    ];

    for family in FAMILIES {
        for case in CASES {
            let mut rng = StdRng::seed_from_u64(42);
            let transaction = new_transaction(family, &mut rng, case);

            match format {
                Some(format) => {
                    assert_roundtrip_for_format(transaction, format)?
                }
                None => assert_legacy_roundtrip(transaction)?,
            }
        }
    }

    Ok(())
}

fn sample_call<R: RngCore>(rng: &mut R) -> ContractCall {
    let mut contract = [0; 32];
    rng.fill_bytes(&mut contract);

    let mut fn_args = vec![0; 100];
    rng.fill_bytes(&mut fn_args);

    ContractCall::new(contract, "deposit").with_raw_args(fn_args)
}

fn sample_deploy<R: RngCore>(rng: &mut R) -> ContractDeploy {
    let mut hash = [0; 32];
    rng.fill_bytes(&mut hash);
    let mut bytes = vec![0; 100];
    rng.fill_bytes(&mut bytes);
    let bytecode = ContractBytecode { hash, bytes };

    let mut owner = [0; 32].to_vec();
    rng.fill_bytes(&mut owner);

    let mut init_args = vec![0; 20];
    rng.fill_bytes(&mut init_args);

    ContractDeploy {
        bytecode,
        owner,
        init_args: Some(init_args),
        nonce: rng.next_u64(),
    }
}

#[test]
fn legacy_roundtrip_matrix() -> Result<(), Error> {
    assert_roundtrip_matrix(None)
}

#[test]
fn boreas_roundtrip_matrix() -> Result<(), Error> {
    assert_roundtrip_matrix(Some(TransactionFormat::Boreas))
}

#[test]
fn ingress_rejects_boreas_before_activation() {
    let mut rng = StdRng::seed_from_u64(42);
    assert_decode_rejected(
        new_moonlight_tx(&mut rng, None),
        TransactionFormat::Boreas,
        TransactionFormat::Aegis,
    );
}

#[test]
fn ingress_rejects_aegis_after_boreas_activation() {
    let mut rng = StdRng::seed_from_u64(42);
    assert_decode_rejected(
        new_moonlight_tx(&mut rng, None),
        TransactionFormat::Aegis,
        TransactionFormat::Boreas,
    );
}

#[test]
fn decode_any_preserves_format() -> Result<(), Error> {
    let mut rng = StdRng::seed_from_u64(42);
    let transaction = new_phoenix_tx(&mut rng, None);
    let transaction_bytes =
        transaction.encode_for_format(TransactionFormat::Boreas);

    let decoded = Transaction::decode_any(&transaction_bytes)?;
    assert_eq!(decoded.transaction, transaction);
    assert_eq!(decoded.format, TransactionFormat::Boreas);

    Ok(())
}

#[test]
fn phoenix_truncated_proof_fails() {
    let mut rng = StdRng::seed_from_u64(42);
    let transaction = new_phoenix_tx(&mut rng, None);

    let mut bytes = transaction.to_var_bytes();
    // Truncate: remove the last byte so proof_len exceeds remaining buffer
    bytes.pop();

    Transaction::from_slice(&bytes)
        .expect_err("truncated proof should fail, not panic");
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
