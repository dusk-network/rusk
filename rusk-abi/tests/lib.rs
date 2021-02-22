// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod contracts;

use rusk_vm::{Contract, GasMeter, NetworkState};

use dusk_bls12_381::BlsScalar;
use dusk_bytes::ParseHexStr;

use canonical_host::MemStore as MS;

use host_fn::HostFnTest;
use rusk_abi::RuskModule;

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

    let hash = HostFnTest::new();

    let store = MS::new();

    let code = include_bytes!(
        "contracts/host_fn/target/wasm32-unknown-unknown/release/host_fn.wasm"
    );

    let contract = Contract::new(hash, code.to_vec(), &store).unwrap();

    let mut network = NetworkState::<MS>::default();

    let rusk_mod = RuskModule::new(store.clone());

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
