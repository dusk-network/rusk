// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![deny(clippy::all)]
#![cfg(feature = "host")]

use std::sync::OnceLock;

use rand_core::OsRng;

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};
use dusk_bytes::{ParseHexStr, Serializable};
use dusk_pki::{PublicKey, PublicSpendKey, SecretKey, SecretSpendKey};
use dusk_plonk::prelude::*;
use dusk_schnorr::Signature;
use ff::Field;
use rusk_abi::hash::Hasher;
use rusk_abi::PublicInput;
use rusk_abi::{ContractData, ContractId, Session, VM};

const POINT_LIMIT: u64 = 0x700000;

#[test]
fn hash_host() {
    let test_inputs = [
        "bb67ed265bf1db490ded2e1ede55c0d14c55521509dc73f9c354e98ab76c9625",
        "7e74220084d75e10c89e9435d47bb5b8075991b2e29be3b84421dac3b1ee6007",
        "5ce5481a4d78cca03498f72761da1b9f1d2aa8fb300be39f0e4fe2534f9d4308",
    ];

    let test_inputs: Vec<BlsScalar> = test_inputs
        .iter()
        .map(|input| BlsScalar::from_hex_str(input).unwrap())
        .collect();

    let mut input = Vec::with_capacity(3 * BlsScalar::SIZE);
    for scalar in test_inputs {
        input.extend(scalar.to_bytes());
    }

    assert_eq!(
        "0x0e17c56704c3ec2523d206e2e06e08b336e0079bb4c4c5b850d496125f73cdb9",
        format!("{:?}", Hasher::digest(input))
    );
}

fn instantiate(vm: &VM, height: u64) -> (Session, ContractId) {
    let bytecode = include_bytes!(
        "../../target/wasm32-unknown-unknown/release/host_fn.wasm"
    );

    let mut session = rusk_abi::new_genesis_session(vm);

    let contract_id = session
        .deploy(
            bytecode,
            ContractData::builder(get_owner().to_bytes()),
            POINT_LIMIT,
        )
        .expect("Deploying module should succeed");

    let base = session.commit().expect("Committing should succeed");

    let session = rusk_abi::new_session(vm, base, height)
        .expect("Instantiating new session should succeed");

    (session, contract_id)
}

#[test]
fn hash() {
    let vm =
        rusk_abi::new_ephemeral_vm().expect("Instantiating VM should succeed");
    let (mut session, contract_id) = instantiate(&vm, 0);

    let test_inputs = [
        "bb67ed265bf1db490ded2e1ede55c0d14c55521509dc73f9c354e98ab76c9625",
        "7e74220084d75e10c89e9435d47bb5b8075991b2e29be3b84421dac3b1ee6007",
        "5ce5481a4d78cca03498f72761da1b9f1d2aa8fb300be39f0e4fe2534f9d4308",
    ];

    let test_inputs: Vec<BlsScalar> = test_inputs
        .iter()
        .map(|input| BlsScalar::from_hex_str(input).unwrap())
        .collect();

    let mut input = Vec::with_capacity(3 * BlsScalar::SIZE);
    for scalar in test_inputs {
        input.extend(scalar.to_bytes())
    }

    let scalar: BlsScalar = session
        .call(contract_id, "hash", &input, POINT_LIMIT)
        .expect("Querying should succeed")
        .data;

    assert_eq!(
        "0x0e17c56704c3ec2523d206e2e06e08b336e0079bb4c4c5b850d496125f73cdb9",
        format!("{scalar:?}")
    );
}

#[test]
fn poseidon_hash() {
    let vm =
        rusk_abi::new_ephemeral_vm().expect("Instantiating VM should succeed");
    let (mut session, contract_id) = instantiate(&vm, 0);

    let test_inputs = [
        "bb67ed265bf1db490ded2e1ede55c0d14c55521509dc73f9c354e98ab76c9625",
        "7e74220084d75e10c89e9435d47bb5b8075991b2e29be3b84421dac3b1ee6007",
        "5ce5481a4d78cca03498f72761da1b9f1d2aa8fb300be39f0e4fe2534f9d4308",
    ];

    let test_inputs: Vec<BlsScalar> = test_inputs
        .iter()
        .map(|input| BlsScalar::from_hex_str(input).unwrap())
        .collect();

    let scalar: BlsScalar = session
        .call(contract_id, "poseidon_hash", &test_inputs, POINT_LIMIT)
        .expect("Querying should succeed")
        .data;

    assert_eq!(
        "0x2885ca6d908b34ca83f2177d78283c25d8c5c7230877025bc8d558b8a94e6fe3",
        format!("{scalar:?}")
    );
}

#[test]
fn schnorr_signature() {
    let vm =
        rusk_abi::new_ephemeral_vm().expect("Instantiating VM should succeed");
    let (mut session, contract_id) = instantiate(&vm, 0);

    let sk = SecretKey::random(&mut OsRng);
    let message = BlsScalar::random(&mut OsRng);
    let pk = PublicKey::from(&sk);

    let sign = Signature::new(&sk, &mut OsRng, message);

    assert!(sign.verify(&pk, message));

    let valid: bool = session
        .call(
            contract_id,
            "verify_schnorr",
            &(message, pk, sign),
            POINT_LIMIT,
        )
        .expect("Querying should succeed")
        .data;

    assert!(valid, "Signature verification expected to succeed");

    let wrong_sk = SecretKey::random(&mut OsRng);
    let pk = PublicKey::from(&wrong_sk);

    let valid: bool = session
        .call(
            contract_id,
            "verify_schnorr",
            &(message, pk, sign),
            POINT_LIMIT,
        )
        .expect("Querying should succeed")
        .data;

    assert!(!valid, "Signature verification expected to fail");
}

#[test]
fn bls_signature() {
    let vm =
        rusk_abi::new_ephemeral_vm().expect("Instantiating VM should succeed");
    let (mut session, contract_id) = instantiate(&vm, 0);

    let message = b"some-message".to_vec();

    let sk = BlsSecretKey::random(&mut OsRng);
    let pk = BlsPublicKey::from(&sk);

    let sign = sk.sign(&pk, &message);

    let arg = (message, pk, sign);
    let valid: bool = session
        .call(contract_id, "verify_bls", &arg, POINT_LIMIT)
        .expect("Query should succeed")
        .data;

    assert!(valid, "BLS Signature verification expected to succeed");

    let wrong_sk = BlsSecretKey::random(&mut OsRng);
    let wrong_pk = BlsPublicKey::from(&wrong_sk);

    let arg = (arg.0, wrong_pk, arg.2);
    let valid: bool = session
        .call(contract_id, "verify_bls", &arg, POINT_LIMIT)
        .expect("Query should succeed")
        .data;

    assert!(!valid, "BLS Signature verification expected to fail");
}

#[derive(Debug, Default)]
pub struct TestCircuit {
    pub a: BlsScalar,
    pub b: BlsScalar,
    pub c: BlsScalar,
}

impl TestCircuit {
    pub fn new(a: u64, b: u64) -> Self {
        let a = a.into();
        let b = b.into();
        let c = a + b;

        Self { a, b, c }
    }
}

impl Circuit for TestCircuit {
    fn circuit<C: Composer>(&self, composer: &mut C) -> Result<(), Error> {
        let a = composer.append_witness(self.a);
        let b = composer.append_witness(self.b);

        let constraint = Constraint::new()
            .left(-BlsScalar::one())
            .a(a)
            .right(-BlsScalar::one())
            .b(b)
            .public(self.c);

        composer.append_gate(constraint);
        composer.append_dummy_gates();

        Ok(())
    }
}

#[test]
fn plonk_proof() {
    let vm =
        rusk_abi::new_ephemeral_vm().expect("Instantiating VM should succeed");
    let (mut session, contract_id) = instantiate(&vm, 0);

    let pp = include_bytes!("./pp_test.bin");
    let pp = unsafe { PublicParameters::from_slice_unchecked(&pp[..]) };

    let label = b"dusk-network";

    let (prover, verifier) = Compiler::compile::<TestCircuit>(&pp, label)
        .expect("Circuit should compile successfully");

    let a = 1u64;
    let b = 2u64;
    let expected_pi = vec![BlsScalar::from(a) + BlsScalar::from(b)];
    let circuit = TestCircuit::new(a, b);

    let (proof, prover_pi) = prover
        .prove(&mut OsRng, &circuit)
        .expect("Proving circuit should succeed");

    // Check public inputs
    assert_eq!(
        expected_pi, prover_pi,
        "Prover generates different pi than expected"
    );

    // Integrity check
    verifier
        .verify(&proof, &expected_pi)
        .expect("Proof should verify successfully");
    let verifier = verifier.to_bytes();

    let public_inputs: Vec<PublicInput> =
        expected_pi.into_iter().map(|pi| From::from(pi)).collect();

    let proof = proof.to_bytes().to_vec();

    let arg = (verifier, proof, public_inputs);
    let valid: bool = session
        .call(contract_id, "verify_proof", &arg, POINT_LIMIT)
        .expect("Query should succeed")
        .data;

    assert!(valid, "The proof should be valid");

    let wrong_public_inputs = vec![BlsScalar::from(0)];
    let wrong_public_inputs: Vec<PublicInput> =
        wrong_public_inputs.into_iter().map(From::from).collect();

    let arg = (arg.0, arg.1, wrong_public_inputs);
    let valid: bool = session
        .call(contract_id, "verify_proof", &arg, POINT_LIMIT)
        .expect("Query should succeed")
        .data;

    assert!(!valid, "The proof should be invalid");
}

#[test]
fn block_height() {
    const HEIGHT: u64 = 123;

    let vm =
        rusk_abi::new_ephemeral_vm().expect("Instantiating VM should succeed");
    let (mut session, contract_id) = instantiate(&vm, HEIGHT);

    let height: u64 = session
        .call(contract_id, "block_height", &(), POINT_LIMIT)
        .expect("Query should succeed")
        .data;

    assert_eq!(height, HEIGHT);
}

fn get_owner() -> &'static PublicSpendKey {
    static OWNER: OnceLock<PublicSpendKey> = OnceLock::new();
    OWNER.get_or_init(|| {
        let secret = SecretSpendKey::random(&mut OsRng);
        secret.public_spend_key()
    })
}

#[test]
fn owner() {
    let vm =
        rusk_abi::new_ephemeral_vm().expect("Instantiating VM should succeed");
    let (mut session, contract_id) = instantiate(&vm, 0);

    let owner: [u8; 64] = session
        .call(contract_id, "contract_owner", get_owner(), POINT_LIMIT)
        .expect("Query should succeed")
        .data;

    assert_eq!(owner, get_owner().to_bytes());
}
