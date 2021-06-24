// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::prelude::*;
use transfer_circuits::*;
use transfer_contract::TransferContract;

fn verifier_data_bytes(id: &[u8; 32]) -> Vec<u8> {
    rusk_profile::keys_for(id)
        .and_then(|keys| keys.get_verifier())
        .expect("Failed to get Rusk profile keys for the provided ID.")
}

#[test]
fn verifier_data_stct() {
    let contract = TransferContract::verifier_data_stct().to_vec();
    let rusk =
        verifier_data_bytes(&SendToContractTransparentCircuit::CIRCUIT_ID);

    assert_eq!(rusk, contract);
}

#[test]
fn verifier_data_stco() {
    let contract = TransferContract::verifier_data_stco().to_vec();
    let rusk =
        verifier_data_bytes(&SendToContractObfuscatedCircuit::CIRCUIT_ID);

    assert_eq!(rusk, contract);
}

#[test]
fn verifier_data_wdft() {
    let contract = TransferContract::verifier_data_wdft().to_vec();
    let rusk = verifier_data_bytes(&WithdrawFromTransparentCircuit::CIRCUIT_ID);

    assert_eq!(rusk, contract);
}

#[test]
fn verifier_data_wdfo() {
    let contract = TransferContract::verifier_data_wdfo().to_vec();
    let rusk = verifier_data_bytes(&WithdrawFromObfuscatedCircuit::CIRCUIT_ID);

    assert_eq!(rusk, contract);
}

#[test]
fn verifier_data_execute() {
    let variants = vec![
        (ExecuteCircuitOneZero::CIRCUIT_ID, 1, 0),
        (ExecuteCircuitOneOne::CIRCUIT_ID, 1, 1),
        (ExecuteCircuitOneTwo::CIRCUIT_ID, 1, 2),
        (ExecuteCircuitTwoZero::CIRCUIT_ID, 2, 0),
        (ExecuteCircuitTwoOne::CIRCUIT_ID, 2, 1),
        (ExecuteCircuitTwoTwo::CIRCUIT_ID, 2, 2),
        (ExecuteCircuitThreeZero::CIRCUIT_ID, 3, 0),
        (ExecuteCircuitThreeOne::CIRCUIT_ID, 3, 1),
        (ExecuteCircuitThreeTwo::CIRCUIT_ID, 3, 2),
        (ExecuteCircuitFourZero::CIRCUIT_ID, 4, 0),
        (ExecuteCircuitFourOne::CIRCUIT_ID, 4, 1),
        (ExecuteCircuitFourTwo::CIRCUIT_ID, 4, 2),
    ];

    for (id, inputs, outputs) in variants {
        let contract =
            TransferContract::verifier_data_execute(inputs, outputs).to_vec();
        let rusk = verifier_data_bytes(&id);

        assert_eq!(rusk, contract);
    }
}
