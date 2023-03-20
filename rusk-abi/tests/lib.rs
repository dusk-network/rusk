// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![deny(clippy::all)]
#![cfg(feature = "host")]

use dusk_bls12_381::BlsScalar;
use dusk_bytes::{ParseHexStr, Serializable};

#[test]
fn hash() {
    let test_inputs = [
        "bb67ed265bf1db490ded2e1ede55c0d14c55521509dc73f9c354e98ab76c9625",
        "7e74220084d75e10c89e9435d47bb5b8075991b2e29be3b84421dac3b1ee6007",
        "5ce5481a4d78cca03498f72761da1b9f1d2aa8fb300be39f0e4fe2534f9d4308",
    ];

    let mut hasher = rusk_abi::hash::Hasher::new();

    test_inputs
        .iter()
        .map(|input| BlsScalar::from_hex_str(input).unwrap())
        .for_each(|scalar| hasher.update(&scalar.to_bytes()));

    hasher.update(b"dusk network rocks");

    assert_eq!(
        "0xe6a5e94d3715f54c5660dd16395fa869f859f0a5ae4b939fdf80739083fb980c",
        format!("{:#x}", hasher.finalize())
    );
}

use piecrust::VM;

use dusk_bls12_381_sign::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
    APK as AggregatedBlsPublicKey,
};
use dusk_pki::{PublicKey, PublicSpendKey, SecretKey};
use dusk_plonk::prelude::*;
use dusk_schnorr::Signature;
use piecrust::Session;
use piecrust_uplink::ModuleId;
use rkyv::{Archive, Deserialize};
use rusk_abi::PublicInput;

lazy_static::lazy_static! {
    static ref PUB_PARAMS: PublicParameters = {
        let pp = include_bytes!("./pp_test.bin");
        unsafe { PublicParameters::from_slice_unchecked(&pp[..]) }
    };
}

fn poseidon_host_query(buf: &mut [u8], arg_len: u32) -> u32 {
    let root = unsafe {
        rkyv::archived_root::<Vec<BlsScalar>>(&buf[..arg_len as usize])
    };
    let scalars: Vec<BlsScalar> =
        root.deserialize(&mut rkyv::Infallible).unwrap();
    let scalar = rusk_abi::poseidon_hash(&scalars);

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

fn instantiate() -> (Session, ModuleId) {
    let bytecode = include_bytes!(
        "../../target/wasm32-unknown-unknown/release/host_fn.wasm"
    );

    let mut vm = VM::ephemeral().expect("Instantiating the VM should succeed");
    vm.register_host_query("poseidon_hash", poseidon_host_query);
    vm.register_host_query("verify_schnorr", schnorr_host_query);

    let mut session = vm.session();

    let module_id = session
        .deploy(bytecode)
        .expect("Deploying module should succeed");

    (session, module_id)
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

// #[test]
// fn hash() {
//     let test_inputs = [
//         "bb67ed265bf1db490ded2e1ede55c0d14c55521509dc73f9c354e98ab76c9625",
//         "7e74220084d75e10c89e9435d47bb5b8075991b2e29be3b84421dac3b1ee6007",
//         "5ce5481a4d78cca03498f72761da1b9f1d2aa8fb300be39f0e4fe2534f9d4308",
//     ];
//
//     let test_inputs: Vec<BlsScalar> = test_inputs
//         .iter()
//         .map(|input| BlsScalar::from_hex_str(input).unwrap())
//         .collect();
//
//     let host = HostFnTest::new();
//
//     let code = include_bytes!(
//         "../../target/wasm32-unknown-unknown/release/host_fn.wasm"
//     );
//
//     let contract = Contract::new(host, code.to_vec());
//
//     let mut network = NetworkState::default();
//     let rusk_mod = RuskModule::new(&PUB_PARAMS);
//     NetworkState::register_host_module(rusk_mod);
//
//     let contract_id = network.deploy(contract).unwrap();
//
//     let mut gas = GasMeter::with_limit(1_000_000_000);
//
//     assert_eq!(
//         "0xe6a5e94d3715f54c5660dd16395fa869f859f0a5ae4b939fdf80739083fb980c",
//         format!(
//             "{:#x}",
//             network
//                 .query::<_, BlsScalar>(
//                     contract_id,
//                     0,
//                     (host_fn::HASH, test_inputs),
//                     &mut gas
//                 )
//                 .unwrap()
//         )
//     );
// }
//
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
//
// #[test]
// fn bls_signature() {
//     let host = HostFnTest::new();
//
//     let code = include_bytes!(
//         "../../target/wasm32-unknown-unknown/release/host_fn.wasm"
//     );
//
//     let contract = Contract::new(host, code.to_vec());
//
//     let rusk_mod = RuskModule::new(&PUB_PARAMS);
//     let mut network = NetworkState::default();
//     NetworkState::register_host_module(rusk_mod);
//
//     let contract_id = network.deploy(contract).unwrap();
//
//     let mut gas = GasMeter::with_limit(1_000_000_000);
//
//     let message = b"some-message".to_vec();
//
//     let sk = BlsSecretKey::random(&mut rand_core::OsRng);
//     let pk = BlsPublicKey::from(&sk);
//     let apk = AggregatedBlsPublicKey::from(&pk);
//
//     let sign = sk.sign(&pk, message.as_slice());
//
//     apk.verify(&sign, message.as_slice())
//         .expect("BLS signature should be valid");
//
//     let res = network
//         .query::<_, bool>(
//             contract_id,
//             0,
//             (host_fn::BLS_SIGNATURE, sign, pk, message.clone()),
//             &mut gas,
//         )
//         .expect("State query failed");
//
//     assert!(res, "BLS Signature verification expected to succeed");
//
//     let wrong_sk = BlsSecretKey::random(&mut rand_core::OsRng);
//     let apk = AggregatedBlsPublicKey::from(&wrong_sk);
//
//     let res = network
//         .query::<_, bool>(
//             contract_id,
//             0,
//             (host_fn::BLS_SIGNATURE, sign, apk, message),
//             &mut gas,
//         )
//         .expect("State query failed");
//
//     assert!(!res, "BLS Signature verification expected to fail");
// }
//
// #[derive(Debug)]
// pub struct TestCircuit {
//     pub a: BlsScalar,
//     pub b: BlsScalar,
//     pub c: BlsScalar,
// }
//
// impl TestCircuit {
//     pub fn new(a: u64, b: u64) -> Self {
//         let a = a.into();
//         let b = b.into();
//         let c = a + b;
//
//         Self { a, b, c }
//     }
// }
//
// impl Circuit for TestCircuit {
//     const CIRCUIT_ID: [u8; 32] = [0xff; 32];
//
//     fn gadget(&mut self, composer: &mut TurboComposer) -> Result<(), Error> {
//         let a = composer.append_witness(self.a);
//         let b = composer.append_witness(self.b);
//
//         let constraint =
//             Constraint::new().left(1).a(a).right(1).b(b).public(-self.c);
//
//         composer.append_gate(constraint);
//         composer.append_dummy_gates();
//
//         Ok(())
//     }
//
//     fn public_inputs(&self) -> Vec<PublicInputValue> {
//         vec![self.c.into()]
//     }
//
//     fn padded_gates(&self) -> usize {
//         1 << 3
//     }
// }
//
// #[test]
// fn verify_proof() {
//     let mut circuit = TestCircuit::new(1, 2);
//
//     let label = b"dusk-network";
//     let (pk, verifier_data) = circuit
//         .compile(&PUB_PARAMS)
//         .expect("Failed to compile the circuit!");
//
//     let proof = circuit
//         .prove(&PUB_PARAMS, &pk, label)
//         .expect("Failed to generate the proof!");
//     let pi = vec![circuit.c.into()];
//
//     // Integrity check
//     circuit::verify(&PUB_PARAMS, &verifier_data, &proof, pi.as_slice(),
// label)         .expect("Failed to verify the proof!");
//
//     let host = HostFnTest::new();
//
//     let code = include_bytes!(
//         "../../target/wasm32-unknown-unknown/release/host_fn.wasm"
//     );
//
//     let contract = Contract::new(host, code.to_vec());
//
//     let rusk_mod = RuskModule::new(&PUB_PARAMS);
//     let mut network = NetworkState::default();
//     NetworkState::register_host_module(rusk_mod);
//
//     let contract_id = network.deploy(contract).unwrap();
//
//     let mut gas = GasMeter::with_limit(1_000_000_000);
//
//     let proof = proof.to_bytes().to_vec();
//     let verifier_data = verifier_data.to_var_bytes();
//     let pi: Vec<PublicInput> = vec![circuit.c.into()];
//
//     let proof = (host_fn::VERIFY, proof, verifier_data, pi);
//
//     let ret = network
//         .query::<_, bool>(contract_id, 0, proof, &mut gas)
//         .expect("Failed to verify the proof with rusk-abi!");
//     assert!(ret);
// }
//
// #[test]
// fn verify_proof_should_fail() {
//     let mut circuit = TestCircuit::new(1, 2);
//
//     let label = b"dusk-network";
//     let (pk, verifier_data) = circuit
//         .compile(&PUB_PARAMS)
//         .expect("Failed to compile the circuit!");
//
//     let proof = circuit
//         .prove(&PUB_PARAMS, &pk, label)
//         .expect("Failed to generate the proof!");
//
//     let host = HostFnTest::new();
//
//     let code = include_bytes!(
//         "../../target/wasm32-unknown-unknown/release/host_fn.wasm"
//     );
//
//     let contract = Contract::new(host, code.to_vec());
//
//     let rusk_mod = RuskModule::new(&PUB_PARAMS);
//     let mut network = NetworkState::default();
//     NetworkState::register_host_module(rusk_mod);
//
//     let contract_id = network.deploy(contract).unwrap();
//
//     let mut gas = GasMeter::with_limit(1_000_000_000);
//
//     let proof = proof.to_bytes().to_vec();
//     let verifier_data = verifier_data.to_var_bytes();
//     let pi: Vec<PublicInput> = vec![BlsScalar::from(4).into()];
//
//     let proof = (host_fn::VERIFY, proof, verifier_data, pi);
//
//     let ret = network
//         .query::<_, bool>(contract_id, 0, proof, &mut gas)
//         .expect("Failed to verify the proof with rusk-abi!");
//     assert!(!ret);
// }
//
// #[test]
// fn payment_info() {
//     let host = HostFnTest::new();
//
//     let code = include_bytes!(
//         "../../target/wasm32-unknown-unknown/release/host_fn.wasm"
//     );
//
//     let contract = Contract::new(host, code.to_vec());
//
//     let rusk_mod = RuskModule::new(&PUB_PARAMS);
//     let mut network = NetworkState::default();
//     NetworkState::register_host_module(rusk_mod);
//
//     let contract_id = network.deploy(contract).unwrap();
//
//     let mut gas = GasMeter::with_limit(1_000_000_000);
//
//     let ret = network
//         .query::<_, PaymentInfo>(
//             contract_id,
//             0,
//             host_fn::GET_PAYMENT_INFO,
//             &mut gas,
//         )
//         .unwrap();
//
//     let expected = PublicSpendKey::new(
//         dusk_jubjub::JubJubExtended::default(),
//         dusk_jubjub::JubJubExtended::default(),
//     )
//     .to_bytes();
//
//     assert!(
//         matches!(ret, PaymentInfo::Any(Some(key)) if key.to_bytes() ==
// expected)     );
// }
