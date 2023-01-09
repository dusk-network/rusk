// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![deny(clippy::all)]
#![cfg(feature = "host")]

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey, APK,
};
use dusk_bytes::{ParseHexStr, Serializable};
use dusk_pki::{PublicKey, SecretKey};
use dusk_plonk::prelude::*;
use dusk_schnorr::Signature;
use piecrust::{Session, VM};
use piecrust_uplink::ModuleId;
use rand_core::OsRng;
use rusk_abi::hash::Hasher;
use rusk_abi::{set_block_height, PublicInput};

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

fn instantiate<'a>(vm: &mut VM) -> (Session, ModuleId) {
    let bytecode = include_bytes!(
        "../../target/wasm32-unknown-unknown/release/host_fn.wasm"
    );

    rusk_abi::register_host_queries(vm);

    let mut session = vm.session();
    session.set_point_limit(0x20000);

    let module_id = session
        .deploy(bytecode)
        .expect("Deploying module should succeed");

    (session, module_id)
}

#[test]
fn hash() {
    let mut vm = VM::ephemeral().expect("Instantiating VM should succeed");
    let (mut session, module_id) = instantiate(&mut vm);

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
        .query(module_id, "hash", input)
        .expect("Querying should succeed");

    assert_eq!(
        "0xb9cd735f1296d450b8c5c4b49b07e036b3086ee0e206d22325ecc30467c5170e",
        format!("{:#x}", scalar)
    );
}

#[test]
fn poseidon_hash() {
    let mut vm = VM::ephemeral().expect("Instantiating VM should succeed");
    let (mut session, module_id) = instantiate(&mut vm);

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
        .query(module_id, "poseidon_hash", test_inputs)
        .expect("Querying should succeed");

    assert_eq!(
        "0xe36f4ea9b858d5c85b02770823c7c5d8253c28787d17f283ca348b906dca8528",
        format!("{:#x}", scalar)
    );
}

#[test]
fn schnorr_signature() {
    let mut vm = VM::ephemeral().expect("Instantiating VM should succeed");
    let (mut session, module_id) = instantiate(&mut vm);

    let sk = SecretKey::random(&mut OsRng);
    let message = BlsScalar::random(&mut OsRng);
    let pk = PublicKey::from(&sk);

    let sign = Signature::new(&sk, &mut OsRng, message);

    assert!(sign.verify(&pk, message));

    let valid: bool = session
        .query(module_id, "verify_schnorr", (message, pk, sign))
        .expect("Querying should succeed");

    assert!(valid, "Signature verification expected to succeed");

    let wrong_sk = SecretKey::random(&mut OsRng);
    let pk = PublicKey::from(&wrong_sk);

    let valid: bool = session
        .query(module_id, "verify_schnorr", (message, pk, sign))
        .expect("Querying should succeed");

    assert!(!valid, "Signature verification expected to fail");
}

#[test]
fn bls_signature() {
    let mut vm = VM::ephemeral().expect("Instantiating VM should succeed");
    let (mut session, module_id) = instantiate(&mut vm);

    let message = b"some-message".to_vec();

    let sk = BlsSecretKey::random(&mut OsRng);
    let pk = BlsPublicKey::from(&sk);

    let sign = sk.sign(&pk, &message);

    let valid: bool = session
        .query(module_id, "verify_bls", (message.clone(), pk, sign))
        .expect("Query should succeed");

    assert!(valid, "BLS Signature verification expected to succeed");

    let wrong_sk = BlsSecretKey::random(&mut OsRng);
    let wrong_pk = BlsPublicKey::from(&wrong_sk);

    let valid: bool = session
        .query(module_id, "verify_bls", (message, wrong_pk, sign))
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
    let mut vm = VM::ephemeral().expect("Instantiating VM should succeed");
    let (mut session, module_id) = instantiate(&mut vm);

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

    let public_inputs: Vec<PublicInput> = public_inputs
        .into_iter()
        // FIXME: this should only be From::from, but due to the negative PI
        //  problem we invert them here
        .map(|pi| From::from(-pi))
        .collect();

    let valid: bool = session
        .query(
            module_id,
            "verify_proof",
            (verifier.to_bytes(), proof.clone(), public_inputs),
        )
        .expect("Query should succeed");

    assert!(valid, "The proof should be valid");

    let wrong_public_inputs = vec![BlsScalar::from(0)];
    let wrong_public_inputs: Vec<PublicInput> =
        wrong_public_inputs.into_iter().map(From::from).collect();

    let valid: bool = session
        .query(
            module_id,
            "verify_proof",
            (verifier.to_bytes(), proof, wrong_public_inputs),
        )
        .expect("Query should succeed");

    assert!(!valid, "The proof should be invalid");
}

#[test]
fn block_height() {
    let mut vm = VM::ephemeral().expect("Instantiating VM should succeed");
    let (mut session, module_id) = instantiate(&mut vm);

    const HEIGHT: u64 = 123;

    set_block_height(&mut session, HEIGHT);

    let height: u64 = session
        .query(module_id, "block_height", ())
        .expect("Query should succeed");

    assert_eq!(height, HEIGHT);
}
