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
use dusk_bytes::{ParseHexStr, Serializable};
use dusk_jubjub::JubJubAffine;
use dusk_pki::{PublicKey, SecretKey, PublicSpendKey};
use dusk_plonk::prelude::*;
use schnorr::Signature;


use host_fn::HostFnTest;
use rusk_abi::{PublicInput, PaymentInfo};
use rusk_abi::RuskModule;

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

#[test]
fn verify_proof() {
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

    // Read VerifierKey
    let vk = include_bytes!("./vk_test.bin");

    // Read the Proof
    let proof = include_bytes!("./proof_test.bin");

    // Public Input Values
    let pi_values: Vec<PublicInput> = vec![
        PublicInput::BlsScalar(BlsScalar::from(25u64)),
        PublicInput::BlsScalar(BlsScalar::from(100u64)),
        PublicInput::Point(dusk_jubjub::GENERATOR),
        PublicInput::Point(JubJubAffine::from(
            dusk_jubjub::GENERATOR_EXTENDED * JubJubScalar::from(2u64),
        )),
    ];

    // Public Input Positions
    let pi_positions = vec![3u32, 20, 21, 22, 2041, 2042];

    assert!(network
        .query::<_, bool>(
            contract_id,
            (
                host_fn::VERIFY,
                proof.to_vec(),
                vk.to_vec(),
                pi_values,
                pi_positions
            ),
            &mut gas
        )
        .unwrap());
}

#[test]
fn payment_info() {
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

    let ret = network
        .query::<_, PaymentInfo>(
            contract_id,
            host_fn::GET_PAYMENT_INFO,
            &mut gas
        )
        .unwrap();

    let expected = PublicSpendKey::new(dusk_jubjub::JubJubExtended::default(), dusk_jubjub::JubJubExtended::default()).to_bytes();

    assert!(matches!(ret, PaymentInfo::Any(Some(key)) if key.to_bytes() == expected));
}    

