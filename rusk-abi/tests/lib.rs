// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![deny(clippy::all)]

mod contracts;

use rusk_vm::{Contract, GasMeter, NetworkState};

use canonical_host::MemStore as MS;
use dusk_bls12_381::BlsScalar;
use dusk_bytes::ParseHexStr;
use dusk_bytes::Serializable;
use dusk_pki::{PublicKey, SecretKey};
use dusk_plonk::circuit;
use dusk_plonk::prelude::*;
use schnorr::Signature;

use host_fn::HostFnTest;
use rusk_abi::{PublicInput, RuskModule};

lazy_static::lazy_static! {
    static ref PUB_PARAMS: PublicParameters = {
        let pp = include_bytes!("./pp_test.bin");
        unsafe { PublicParameters::from_slice_unchecked(&pp[..]) }
    };
}

#[test]
fn poseidon_hash() {
    let test_inputs = [
        "bb67ed265bf1db490ded2e1ede55c0d14c55521509dc73f9c354e98ab76c9625",
        "7e74220084d75e10c89e9435d47bb5b8075991b2e29be3b84421dac3b1ee6007",
        "5ce5481a4d78cca03498f72761da1b9f1d2aa8fb300be39f0e4fe2534f9d4308",
    ];

    let test_inputs: Vec<BlsScalar> = test_inputs
        .iter()
        .map(|input| BlsScalar::from_hex_str(input).unwrap())
        .collect();

    let host = HostFnTest::new();

    let store = MS::new();

    let code = include_bytes!(
        "../../target/wasm32-unknown-unknown/release/host_fn.wasm"
    );

    let contract = Contract::new(host, code.to_vec(), &store).unwrap();

    let mut network = NetworkState::<MS>::default();
    let rusk_mod = RuskModule::new(store, &PUB_PARAMS);
    network.register_host_module(rusk_mod);

    let contract_id = network.deploy(contract).unwrap();

    let mut gas = GasMeter::with_limit(1_000_000_000);

    assert_eq!(
        "0xe36f4ea9b858d5c85b02770823c7c5d8253c28787d17f283ca348b906dca8528",
        format!(
            "{:#x}",
            network
                .query::<_, BlsScalar>(
                    contract_id,
                    (host_fn::HASH, test_inputs),
                    &mut gas
                )
                .unwrap()
        )
    );
}

#[test]
fn schnorr_signature() {
    let host = HostFnTest::new();

    let store = MS::new();

    let code = include_bytes!(
        "../../target/wasm32-unknown-unknown/release/host_fn.wasm"
    );

    let contract = Contract::new(host, code.to_vec(), &store).unwrap();

    let rusk_mod = RuskModule::new(store, &PUB_PARAMS);
    let mut network = NetworkState::<MS>::default();
    network.register_host_module(rusk_mod);

    let contract_id = network.deploy(contract).unwrap();

    let mut gas = GasMeter::with_limit(1_000_000_000);

    let sk = SecretKey::random(&mut rand::thread_rng());
    let message = BlsScalar::random(&mut rand::thread_rng());
    let pk = PublicKey::from(&sk);

    let sign = Signature::new(&sk, &mut rand::thread_rng(), message);

    assert!(sign.verify(&pk, message));

    assert!(
        network
            .query::<_, bool>(
                contract_id,
                (host_fn::SCHNORR_SIGNATURE, sign, pk, message),
                &mut gas
            )
            .unwrap(),
        "Signature verification expected to succeed"
    );

    let wrong_sk = SecretKey::random(&mut rand::thread_rng());
    let pk = PublicKey::from(&wrong_sk);

    assert!(
        !network
            .query::<_, bool>(
                contract_id,
                (host_fn::SCHNORR_SIGNATURE, sign, pk, message),
                &mut gas
            )
            .unwrap(),
        "Signature verification expected to fail"
    );
}

#[derive(Debug)]
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
    const CIRCUIT_ID: [u8; 32] = [0xff; 32];

    fn gadget(&mut self, composer: &mut StandardComposer) -> Result<(), Error> {
        let zero =
            composer.add_witness_to_circuit_description(BlsScalar::zero());
        let a = composer.add_input(self.a);
        let b = composer.add_input(self.b);

        // Make first constraint a + b = c
        composer.poly_gate(
            a,
            b,
            zero,
            BlsScalar::zero(),
            BlsScalar::one(),
            BlsScalar::one(),
            BlsScalar::zero(),
            BlsScalar::zero(),
            Some(-self.c),
        );

        Ok(())
    }

    fn padded_circuit_size(&self) -> usize {
        1 << 3
    }
}

#[test]
fn verify_proof() {
    let mut circuit = TestCircuit::new(1, 2);

    let label = b"dusk-network";
    let (pk, verifier_data) = circuit
        .compile(&PUB_PARAMS)
        .expect("Failed to compile the circuit!");

    let proof = circuit
        .gen_proof(&PUB_PARAMS, &pk, label)
        .expect("Failed to generate the proof!");
    let pi = vec![circuit.c.into()];

    // Sanity check
    circuit::verify_proof(
        &PUB_PARAMS,
        &verifier_data.key(),
        &proof,
        pi.as_slice(),
        verifier_data.pi_pos().as_slice(),
        label,
    )
    .expect("Failed to verify the proof!");

    let host = HostFnTest::new();

    let store = MS::new();

    let code = include_bytes!(
        "../../target/wasm32-unknown-unknown/release/host_fn.wasm"
    );

    let contract = Contract::new(host, code.to_vec(), &store).unwrap();

    let rusk_mod = RuskModule::new(store, &PUB_PARAMS);
    let mut network = NetworkState::<MS>::default();
    network.register_host_module(rusk_mod);

    let contract_id = network.deploy(contract).unwrap();

    let mut gas = GasMeter::with_limit(1_000_000_000);

    let proof = proof.to_bytes().to_vec();
    let verifier_data = verifier_data.to_var_bytes();
    let pi: Vec<PublicInput> = vec![circuit.c.into()];

    let proof = (host_fn::VERIFY, proof, verifier_data, pi);

    let ret = network
        .query::<_, bool>(contract_id, proof, &mut gas)
        .expect("Failed to verify the proof with rusk-abi!");
    assert!(ret);
}

#[test]
fn verify_proof_should_fail() {
    let mut circuit = TestCircuit::new(1, 2);

    let label = b"dusk-network";
    let (pk, verifier_data) = circuit
        .compile(&PUB_PARAMS)
        .expect("Failed to compile the circuit!");

    let proof = circuit
        .gen_proof(&PUB_PARAMS, &pk, label)
        .expect("Failed to generate the proof!");

    let host = HostFnTest::new();

    let store = MS::new();

    let code = include_bytes!(
        "../../target/wasm32-unknown-unknown/release/host_fn.wasm"
    );

    let contract = Contract::new(host, code.to_vec(), &store).unwrap();

    let rusk_mod = RuskModule::new(store, &PUB_PARAMS);
    let mut network = NetworkState::<MS>::default();
    network.register_host_module(rusk_mod);

    let contract_id = network.deploy(contract).unwrap();

    let mut gas = GasMeter::with_limit(1_000_000_000);

    let proof = proof.to_bytes().to_vec();
    let verifier_data = verifier_data.to_var_bytes();
    let pi: Vec<PublicInput> = vec![BlsScalar::from(4).into()];

    let proof = (host_fn::VERIFY, proof, verifier_data, pi);

    let ret = network
        .query::<_, bool>(contract_id, proof, &mut gas)
        .expect("Failed to verify the proof with rusk-abi!");
    assert!(!ret);
}
