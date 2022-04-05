// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use transfer_circuits::*;
use transfer_contract::TransferContract;

use dusk_plonk::prelude::*;

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
        (ExecuteCircuitOneTwo::CIRCUIT_ID, 1),
        (ExecuteCircuitTwoTwo::CIRCUIT_ID, 2),
        (ExecuteCircuitThreeTwo::CIRCUIT_ID, 3),
        (ExecuteCircuitFourTwo::CIRCUIT_ID, 4),
    ];

    for (id, inputs) in variants {
        let contract = TransferContract::verifier_data_execute(inputs).to_vec();
        let rusk = verifier_data_bytes(&id);

        assert_eq!(rusk, contract);
    }
}
