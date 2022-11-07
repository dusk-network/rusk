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
    Signature as BlsSignature, APK,
};
use dusk_bytes::{ParseHexStr, Serializable};
use dusk_pki::{PublicKey, SecretKey};
use dusk_plonk::prelude::*;
use dusk_schnorr::Signature;
use once_cell::sync::OnceCell;
use piecrust::{Session, VM};
use piecrust_uplink::ModuleId;
use rand_core::OsRng;
use rkyv::Deserialize;
use rusk_abi::hash::Hasher;
use rusk_abi::{CircuitType, MetadataType, PublicInput, QueryType};

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

struct ProverVerifier {
    prover: Prover<TestCircuit>,
    verifier: Verifier<TestCircuit>,
}

fn get_prover_verifier() -> &'static ProverVerifier {
    static PROVER_VERIFIER: OnceCell<ProverVerifier> = OnceCell::new();

    let pp = include_bytes!("./pp_test.bin");
    let pp = unsafe { PublicParameters::from_slice_unchecked(&pp[..]) };

    let label = b"dusk-network";

    PROVER_VERIFIER.get_or_init(|| {
        let (prover, verifier) = Compiler::compile(&pp, label)
            .expect("Compiling the circuit should succeed");
        ProverVerifier { prover, verifier }
    })
}

fn hash_host_query(buf: &mut [u8], arg_len: u32) -> u32 {
    let root =
        unsafe { rkyv::archived_root::<Vec<u8>>(&buf[..arg_len as usize]) };
    let bytes: Vec<u8> = root.deserialize(&mut rkyv::Infallible).unwrap();
    let valid = rusk_abi::hash(bytes);

    let bytes = rkyv::to_bytes::<_, 256>(&valid).unwrap();

    buf[..bytes.len()].copy_from_slice(&bytes);
    bytes.len() as u32
}

fn poseidon_host_query(buf: &mut [u8], arg_len: u32) -> u32 {
    let root = unsafe {
        rkyv::archived_root::<Vec<BlsScalar>>(&buf[..arg_len as usize])
    };
    let scalars = root.deserialize(&mut rkyv::Infallible).unwrap();
    let scalar = rusk_abi::poseidon_hash(scalars);

    let bytes = rkyv::to_bytes::<_, 256>(&scalar).unwrap();

    buf[..bytes.len()].copy_from_slice(&bytes);
    bytes.len() as u32
}

fn schnorr_host_query(buf: &mut [u8], arg_len: u32) -> u32 {
    let root = unsafe {
        rkyv::archived_root::<(BlsScalar, PublicKey, Signature)>(
            &buf[..arg_len as usize],
        )
    };
    let (msg, pk, sig): (BlsScalar, PublicKey, Signature) =
        root.deserialize(&mut rkyv::Infallible).unwrap();
    let valid = rusk_abi::verify_schnorr(msg, pk, sig);

    let bytes = rkyv::to_bytes::<_, 256>(&valid).unwrap();

    buf[..bytes.len()].copy_from_slice(&bytes);
    bytes.len() as u32
}

fn bls_host_query(buf: &mut [u8], arg_len: u32) -> u32 {
    let root = unsafe {
        rkyv::archived_root::<(Vec<u8>, APK, BlsSignature)>(
            &buf[..arg_len as usize],
        )
    };

    let (msg, apk, sig): (Vec<u8>, APK, BlsSignature) =
        root.deserialize(&mut rkyv::Infallible).unwrap();
    let valid = rusk_abi::verify_bls(msg, apk, sig);

    let bytes = rkyv::to_bytes::<_, 256>(&valid).unwrap();

    buf[..bytes.len()].copy_from_slice(&bytes);
    bytes.len() as u32
}

fn plonk_host_query(buf: &mut [u8], arg_len: u32) -> u32 {
    let root = unsafe {
        rkyv::archived_root::<(CircuitType, Proof, Vec<PublicInput>)>(
            &buf[..arg_len as usize],
        )
    };

    // Ignore the circuit type here, since we're testing only the ability to
    // prove.
    let (_, proof, public_inputs): (CircuitType, Proof, Vec<PublicInput>) =
        root.deserialize(&mut rkyv::Infallible).unwrap();

    let verifier = &get_prover_verifier().verifier;
    let valid = rusk_abi::verify_proof(verifier, proof, public_inputs);

    let bytes = rkyv::to_bytes::<_, 256>(&valid).unwrap();

    buf[..bytes.len()].copy_from_slice(&bytes);
    bytes.len() as u32
}

fn instantiate() -> (Session, ModuleId) {
    let bytecode = include_bytes!(
        "../../target/wasm32-unknown-unknown/release/host_fn.wasm"
    );

    let mut vm = VM::ephemeral().expect("Instantiating the VM should succeed");

    vm.register_host_query(QueryType::Hash.as_str(), hash_host_query);
    vm.register_host_query(
        QueryType::PoseidonHash.as_str(),
        poseidon_host_query,
    );
    vm.register_host_query(QueryType::VerifyProof.as_str(), plonk_host_query);
    vm.register_host_query(QueryType::VerifyBls.as_str(), bls_host_query);
    vm.register_host_query(
        QueryType::VerifySchnorr.as_str(),
        schnorr_host_query,
    );

    let mut session = vm.session();

    let module_id = session
        .deploy(bytecode)
        .expect("Deploying module should succeed");

    (session, module_id)
}

#[test]
fn hash() {
    let (mut session, module_id) = instantiate();

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
    let (mut session, module_id) = instantiate();

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
    let (mut session, module_id) = instantiate();

    let sk = SecretKey::random(&mut rand_core::OsRng);
    let message = BlsScalar::random(&mut rand_core::OsRng);
    let pk = PublicKey::from(&sk);

    let sign = Signature::new(&sk, &mut rand_core::OsRng, message);

    assert!(sign.verify(&pk, message));

    let valid: bool = session
        .query(module_id, "verify_schnorr", (message, pk, sign))
        .expect("Querying should succeed");

    assert!(valid, "Signature verification expected to succeed");

    let wrong_sk = SecretKey::random(&mut rand_core::OsRng);
    let pk = PublicKey::from(&wrong_sk);

    let valid: bool = session
        .query(module_id, "verify_schnorr", (message, pk, sign))
        .expect("Querying should succeed");

    assert!(!valid, "Signature verification expected to fail");
}

#[test]
fn bls_signature() {
    let (mut session, module_id) = instantiate();

    let message = b"some-message".to_vec();

    let sk = BlsSecretKey::random(&mut OsRng);
    let pk = BlsPublicKey::from(&sk);
    let apk = APK::from(&pk);

    let sign = sk.sign(&pk, &message);

    apk.verify(&sign, &message)
        .expect("BLS signature should be valid");

    let valid: bool = session
        .query(module_id, "verify_bls", (message.clone(), apk, sign))
        .expect("Query should succeed");

    assert!(valid, "BLS Signature verification expected to succeed");

    let wrong_sk = BlsSecretKey::random(&mut OsRng);
    let wrong_pk = BlsPublicKey::from(&wrong_sk);
    let wrong_apk = APK::from(&wrong_pk);

    let valid: bool = session
        .query(module_id, "verify_bls", (message, wrong_apk, sign))
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
    let (mut session, module_id) = instantiate();

    let prover_verifier = get_prover_verifier();
    let prover = &prover_verifier.prover;
    let verifier = &prover_verifier.verifier;

    let circuit = TestCircuit::new(1, 2);

    let (proof, public_inputs) = prover
        .prove(&mut OsRng, &circuit)
        .expect("Proving circuit should succeed");

    // Integrity check
    verifier
        .verify(&proof, &public_inputs)
        .expect("Proof should verify successfully");

    let public_inputs: Vec<PublicInput> =
        public_inputs.into_iter().map(From::from).collect();

    let valid: bool = session
        .query(
            module_id,
            "verify_proof",
            (CircuitType::WFCT, proof.clone(), public_inputs),
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
            (CircuitType::WFCT, proof, wrong_public_inputs),
        )
        .expect("Query should succeed");

    assert!(!valid, "The proof should be invalid");
}

#[test]
fn block_height() {
    let (mut session, module_id) = instantiate();

    const HEIGHT: u64 = 123;

    session.set_meta(MetadataType::BlockHeight.as_str(), HEIGHT);

    let height: u64 = session
        .query(module_id, "block_height", ())
        .expect("Query should succeed");

    assert_eq!(height, HEIGHT);
}
