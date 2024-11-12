// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![deny(clippy::all)]

use std::sync::OnceLock;

use dusk_bytes::{ParseHexStr, Serializable};
use execution_core::groth16::bn254::{Bn254, Fr as Bn254Fr};
use execution_core::groth16::relations::lc;
use execution_core::groth16::relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystemRef, Field as Groth16Field,
    SynthesisError, Variable,
};
use execution_core::groth16::serialize::{CanonicalSerialize, Compress};
use execution_core::groth16::verifier::prepare_verifying_key;
use execution_core::groth16::Groth16;
use execution_core::plonk::{
    Circuit, Compiler, Composer, Constraint, Error as PlonkError,
    PublicParameters,
};
use execution_core::signatures::bls::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};
use execution_core::signatures::schnorr::{
    PublicKey as SchnorrPublicKey, SecretKey as SchnorrSecretKey,
};
use execution_core::{BlsScalar, ContractId};
use ff::Field;
use rand::rngs::OsRng;
use rusk_abi::{ContractData, Session, VM};

const POINT_LIMIT: u64 = 0x4000000;
const CHAIN_ID: u8 = 0xFA;

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
        "0x58c751eca2d6a41227e0c52ef579f4688d698b3447a8bcc27fb2831e11d3239e",
        format!("{:?}", BlsScalar::hash_to_scalar(&input[..]))
    );
}

fn instantiate(vm: &VM, height: u64) -> (Session, ContractId) {
    let bytecode = include_bytes!(
        "../../target/dusk/wasm32-unknown-unknown/release/host_fn.wasm"
    );

    let mut session = rusk_abi::new_genesis_session(vm, CHAIN_ID);

    let contract_id = session
        .deploy(
            bytecode,
            ContractData::builder().owner(get_owner().to_bytes()),
            POINT_LIMIT,
        )
        .expect("Deploying module should succeed");

    let base = session.commit().expect("Committing should succeed");

    let session = rusk_abi::new_session(vm, base, CHAIN_ID, height)
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
        "0x58c751eca2d6a41227e0c52ef579f4688d698b3447a8bcc27fb2831e11d3239e",
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
        "0x6ee56db5a9ffb1ed8cc923bba770d01b7f49feb9cd5ffe6e73ba73643089b54a",
        format!("{scalar:?}")
    );
}

#[test]
fn schnorr_signature() {
    let vm =
        rusk_abi::new_ephemeral_vm().expect("Instantiating VM should succeed");
    let (mut session, contract_id) = instantiate(&vm, 0);

    let sk = SchnorrSecretKey::random(&mut OsRng);
    let message = BlsScalar::random(&mut OsRng);
    let pk = SchnorrPublicKey::from(&sk);

    let sig = sk.sign(&mut OsRng, message);

    assert!(pk.verify(&sig, message).is_ok());

    let valid: bool = session
        .call(
            contract_id,
            "verify_schnorr",
            &(message, pk, sig),
            POINT_LIMIT,
        )
        .expect("Querying should succeed")
        .data;

    assert!(valid, "Signature verification expected to succeed");

    let wrong_sk = SchnorrSecretKey::random(&mut OsRng);
    let pk = SchnorrPublicKey::from(&wrong_sk);

    let valid: bool = session
        .call(
            contract_id,
            "verify_schnorr",
            &(message, pk, sig),
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

    let sig = sk.sign(&message);

    let arg = (message, pk, sig);
    let valid: bool = session
        .call(contract_id, "verify_bls", &arg, POINT_LIMIT)
        .expect("Query should succeed")
        .data;

    assert!(valid, "Stake Signature verification expected to succeed");

    let wrong_sk = BlsSecretKey::random(&mut OsRng);
    let wrong_pk = BlsPublicKey::from(&wrong_sk);

    let arg = (arg.0, wrong_pk, arg.2);
    let valid: bool = session
        .call(contract_id, "verify_bls", &arg, POINT_LIMIT)
        .expect("Query should succeed")
        .data;

    assert!(!valid, "Stake Signature verification expected to fail");
}

#[test]
fn bls_multisig_signature() {
    let vm =
        rusk_abi::new_ephemeral_vm().expect("Instantiating VM should succeed");
    let (mut session, contract_id) = instantiate(&vm, 0);

    let message = b"some-message".to_vec();

    let sk0 = BlsSecretKey::random(&mut OsRng);
    let pk0 = BlsPublicKey::from(&sk0);
    let sig0 = sk0.sign_multisig(&pk0, &message);

    let sk1 = BlsSecretKey::random(&mut OsRng);
    let pk1 = BlsPublicKey::from(&sk1);
    let sig1 = sk1.sign_multisig(&pk1, &message);

    let sig = sig0.aggregate(&[sig1]);
    let mut arg = (message, vec![pk0, pk1], sig);

    let valid: bool = session
        .call(contract_id, "verify_bls_multisig", &arg, POINT_LIMIT)
        .expect("Query should succeed")
        .data;

    assert!(valid, "Stake Signature verification expected to succeed");

    let wrong_sk = BlsSecretKey::random(&mut OsRng);
    let wrong_pk = BlsPublicKey::from(&wrong_sk);

    arg.1[1] = wrong_pk;

    let valid: bool = session
        .call(contract_id, "verify_bls_multisig", &arg, POINT_LIMIT)
        .expect("Query should succeed")
        .data;

    assert!(!valid, "Multisig Signature verification expected to fail");
}

#[derive(Debug, Default)]
pub struct PlonkTestCircuit {
    pub a: BlsScalar,
    pub b: BlsScalar,
    pub c: BlsScalar,
}

impl PlonkTestCircuit {
    pub fn new(a: u64, b: u64) -> Self {
        let a = a.into();
        let b = b.into();
        let c = a + b;

        Self { a, b, c }
    }
}

impl Circuit for PlonkTestCircuit {
    fn circuit(&self, composer: &mut Composer) -> Result<(), PlonkError> {
        // append 3 gates that always evaluate to true

        let a = composer.append_witness(self.a);
        let b = composer.append_witness(self.b);
        let six = composer.append_witness(BlsScalar::from(6));
        let one = composer.append_witness(BlsScalar::from(1));
        let seven = composer.append_witness(BlsScalar::from(7));
        let min_twenty = composer.append_witness(-BlsScalar::from(20));

        let constraint = Constraint::new()
            .left(-BlsScalar::one())
            .a(a)
            .right(-BlsScalar::one())
            .b(b)
            .public(self.c);
        composer.append_gate(constraint);

        let constraint = Constraint::new()
            .mult(1)
            .left(2)
            .right(3)
            .fourth(1)
            .constant(4)
            .output(4)
            .a(six)
            .b(seven)
            .d(one)
            .c(min_twenty);
        composer.append_gate(constraint);

        let constraint = Constraint::new()
            .mult(1)
            .left(1)
            .right(1)
            .constant(127)
            .output(1)
            .a(min_twenty)
            .b(six)
            .c(seven);
        composer.append_gate(constraint);

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

    let (prover, verifier) = Compiler::compile::<PlonkTestCircuit>(&pp, label)
        .expect("Circuit should compile successfully");

    let a = 1u64;
    let b = 2u64;
    let expected_pi = vec![BlsScalar::from(a) + BlsScalar::from(b)];
    let circuit = PlonkTestCircuit::new(a, b);

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

    let proof = proof.to_bytes().to_vec();

    let arg = (verifier, proof, expected_pi);
    let valid: bool = session
        .call(contract_id, "verify_plonk", &arg, POINT_LIMIT)
        .expect("Query should succeed")
        .data;

    assert!(valid, "The proof should be valid");

    let wrong_public_inputs = vec![BlsScalar::from(0)];

    let arg = (arg.0, arg.1, wrong_public_inputs);
    let valid: bool = session
        .call(contract_id, "verify_plonk", &arg, POINT_LIMIT)
        .expect("Query should succeed")
        .data;

    assert!(!valid, "The proof should be invalid");
}

#[derive(Debug, Clone, Copy)]
struct Groth16TestCircuit<F> {
    a: F,
    b: F,
    c: F,
}

impl<F: Groth16Field> ConstraintSynthesizer<F> for Groth16TestCircuit<F> {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<F>,
    ) -> Result<(), SynthesisError> {
        let a = cs.new_witness_variable(|| Ok(self.a))?;
        let b = cs.new_witness_variable(|| Ok(self.b))?;
        let c = cs.new_input_variable(|| Ok(self.c))?;

        cs.enforce_constraint(lc!() + a + b, lc!() + Variable::One, lc!() + c)?;
        cs.finalize();

        Ok(())
    }
}

#[test]
fn groth16_proof() {
    let vm =
        rusk_abi::new_ephemeral_vm().expect("Instantiating VM should succeed");
    let (mut session, contract_id) = instantiate(&vm, 0);

    let test_circuit = Groth16TestCircuit {
        a: Bn254Fr::from(1u8),
        b: Bn254Fr::from(2u8),
        c: Bn254Fr::from(3u8),
    };

    let pk = Groth16::<Bn254>::generate_random_parameters_with_reduction(
        test_circuit,
        &mut OsRng,
    )
    .expect("generating random parameters should succeed");

    let proof = Groth16::<Bn254>::create_random_proof_with_reduction(
        test_circuit,
        &pk,
        &mut OsRng,
    )
    .expect("creating proof should succeed");

    let pvk = prepare_verifying_key(&pk.vk);
    let inputs = Groth16::<Bn254>::prepare_inputs(&pvk, &[test_circuit.c])
        .expect("preparing inputs should succeed");

    // integrity check
    let is_proof_valid = Groth16::<Bn254>::verify_proof_with_prepared_inputs(
        &pvk, &proof, &inputs,
    )
    .expect("verifying the proof should succeed");

    assert!(
        is_proof_valid,
        "the proof should be valid for a valid circuit"
    );

    let mut pvk_bytes: Vec<u8> =
        Vec::with_capacity(pvk.serialized_size(Compress::No));
    pvk.serialize_uncompressed(&mut pvk_bytes)
        .expect("serializing should succeed");

    let mut proof_bytes: Vec<u8> =
        Vec::with_capacity(proof.serialized_size(Compress::Yes));
    proof
        .serialize_compressed(&mut proof_bytes)
        .expect("serializing should succeed");

    let mut inputs_bytes: Vec<u8> =
        Vec::with_capacity(inputs.serialized_size(Compress::Yes));
    inputs
        .serialize_compressed(&mut inputs_bytes)
        .expect("serializing should succeed");

    let mut arg = (pvk_bytes, proof_bytes, inputs_bytes);

    let is_proof_valid: bool = session
        .call(contract_id, "verify_groth16_bn254", &arg, POINT_LIMIT)
        .expect("Query should succeed")
        .data;

    assert!(
        is_proof_valid,
        "the proof should be valid for a valid circuit"
    );

    let wrong_inputs =
        Groth16::<Bn254>::prepare_inputs(&pvk, &[test_circuit.a])
            .expect("preparing inputs should succeed");
    let mut wrong_inputs_bytes: Vec<u8> =
        Vec::with_capacity(wrong_inputs.serialized_size(Compress::Yes));
    wrong_inputs
        .serialize_compressed(&mut wrong_inputs_bytes)
        .expect("serializing should succeed");

    arg.2 = wrong_inputs_bytes;

    let is_proof_valid: bool = session
        .call(contract_id, "verify_groth16_bn254", &arg, POINT_LIMIT)
        .expect("Query should succeed")
        .data;

    assert!(
        !is_proof_valid,
        "the proof should be invalid with wrong inputs"
    );
}

#[test]
fn chain_id() {
    const HEIGHT: u64 = 123;

    let vm =
        rusk_abi::new_ephemeral_vm().expect("Instantiating VM should succeed");
    let (mut session, contract_id) = instantiate(&vm, HEIGHT);

    let chain_id: u8 = session
        .call(contract_id, "chain_id", &(), POINT_LIMIT)
        .expect("Query should succeed")
        .data;

    assert_eq!(chain_id, CHAIN_ID);
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

fn get_owner() -> &'static BlsPublicKey {
    static OWNER: OnceLock<BlsPublicKey> = OnceLock::new();
    OWNER.get_or_init(|| {
        let sk = BlsSecretKey::random(&mut OsRng);
        BlsPublicKey::from(&sk)
    })
}

#[test]
fn owner_raw() {
    let vm =
        rusk_abi::new_ephemeral_vm().expect("Instantiating VM should succeed");
    let (mut session, contract_id) = instantiate(&vm, 0);

    let owner: [u8; 96] = session
        .call(contract_id, "contract_owner_raw", get_owner(), POINT_LIMIT)
        .expect("Query should succeed")
        .data;

    assert_eq!(owner, get_owner().to_bytes());
}

#[test]
fn owner() {
    let vm =
        rusk_abi::new_ephemeral_vm().expect("Instantiating VM should succeed");
    let (mut session, contract_id) = instantiate(&vm, 0);

    let owner: BlsPublicKey = session
        .call(contract_id, "contract_owner", get_owner(), POINT_LIMIT)
        .expect("Query should succeed")
        .data;

    assert_eq!(owner, get_owner().to_owned());
}
