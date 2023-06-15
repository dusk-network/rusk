// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![deny(clippy::all)]
#![cfg(feature = "host")]

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};
use dusk_bytes::{ParseHexStr, Serializable};
use dusk_pki::{PublicKey, SecretKey};
use dusk_plonk::prelude::*;
use dusk_schnorr::Signature;
use piecrust::{ContractData, Session, VM};
use piecrust_uplink::ContractId;
use rand_core::OsRng;
use rusk_abi::hash::Hasher;
use rusk_abi::PublicInput;

const OWNER: [u8; 32] = [0; 32];

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
        "0xb9cd735f1296d450b8c5c4b49b07e036b3086ee0e206d22325ecc30467c5170e",
        format!("{:#x}", Hasher::digest(input))
    );
}

fn instantiate(vm: &VM, height: u64) -> (Session, ContractId) {
    let bytecode = include_bytes!(
        "../../target/wasm32-unknown-unknown/release/host_fn.wasm"
    );

    let mut session = rusk_abi::new_genesis_session(vm);

    let contract_id = session
        .deploy(bytecode, ContractData::builder(OWNER))
        .expect("Deploying module should succeed");

    let base = session.commit().expect("Committing should succeed");

    let mut session = rusk_abi::new_session(vm, base, height)
        .expect("Instantiating new session should succeed");
    session.set_point_limit(0x20000);

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
        .call(contract_id, "hash", &input)
        .expect("Querying should succeed");

    assert_eq!(
        "0xb9cd735f1296d450b8c5c4b49b07e036b3086ee0e206d22325ecc30467c5170e",
        format!("{scalar:#x}")
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
        .call(contract_id, "poseidon_hash", &test_inputs)
        .expect("Querying should succeed");

    assert_eq!(
        "0xe36f4ea9b858d5c85b02770823c7c5d8253c28787d17f283ca348b906dca8528",
        format!("{scalar:#x}")
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
        .call(contract_id, "verify_schnorr", &(message, pk, sign))
        .expect("Querying should succeed");

    assert!(valid, "Signature verification expected to succeed");

    let wrong_sk = SecretKey::random(&mut OsRng);
    let pk = PublicKey::from(&wrong_sk);

    let valid: bool = session
        .call(contract_id, "verify_schnorr", &(message, pk, sign))
        .expect("Querying should succeed");

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
        .call(contract_id, "verify_bls", &arg)
        .expect("Query should succeed");

    assert!(valid, "BLS Signature verification expected to succeed");

    let wrong_sk = BlsSecretKey::random(&mut OsRng);
    let wrong_pk = BlsPublicKey::from(&wrong_sk);

    let arg = (arg.0, wrong_pk, arg.2);
    let valid: bool = session
        .call(contract_id, "verify_bls", &arg)
        .expect("Query should succeed");

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

        let constraint =
            Constraint::new().left(1).a(a).right(1).b(b).public(-self.c);

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

    let (prover, verifier) = Compiler::compile(&pp, label)
        .expect("Circuit should compile successfully");

    let circuit = TestCircuit::new(1, 2);

    let (proof, public_inputs) = prover
        .prove(&mut OsRng, &circuit)
        .expect("Proving circuit should succeed");

    // Integrity check
    verifier
        .verify(&proof, &public_inputs)
        .expect("Proof should verify successfully");
    let verifier = verifier.to_bytes();

    let public_inputs: Vec<PublicInput> = public_inputs
        .into_iter()
        // FIXME: this should only be From::from, but due to the negative PI
        //  problem we invert them here
        .map(|pi| From::from(-pi))
        .collect();

    let proof = proof.to_bytes().to_vec();

    let arg = (verifier, proof, public_inputs);
    let valid: bool = session
        .call(contract_id, "verify_proof", &arg)
        .expect("Query should succeed");

    assert!(valid, "The proof should be valid");

    let wrong_public_inputs = vec![BlsScalar::from(0)];
    let wrong_public_inputs: Vec<PublicInput> =
        wrong_public_inputs.into_iter().map(From::from).collect();

    let arg = (arg.0, arg.1, wrong_public_inputs);
    let valid: bool = session
        .call(contract_id, "verify_proof", &arg)
        .expect("Query should succeed");

    assert!(!valid, "The proof should be invalid");
}

#[test]
fn block_height() {
    const HEIGHT: u64 = 123;

    let vm =
        rusk_abi::new_ephemeral_vm().expect("Instantiating VM should succeed");
    let (mut session, contract_id) = instantiate(&vm, HEIGHT);

    let height: u64 = session
        .call(contract_id, "block_height", &())
        .expect("Query should succeed");

    assert_eq!(height, HEIGHT);
}
