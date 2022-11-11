// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use transfer_circuits::*;
use transfer_contract_types::*;

fn verifier_data_bytes(id: &[u8; 32]) -> Vec<u8> {
    rusk_profile::keys_for(id)
        .and_then(|keys| keys.get_verifier())
        .expect("Failed to get Rusk profile keys for the provided ID.")
}

#[test]
fn vd_stct() {
    let vd = verifier_data_stct();
    let rusk_vd =
        verifier_data_bytes(&SendToContractTransparentCircuit::circuit_id());

    assert_eq!(&rusk_vd, vd);
}

#[test]
fn vd_stco() {
    let vd = verifier_data_stco();
    let rusk_vd =
        verifier_data_bytes(&SendToContractObfuscatedCircuit::circuit_id());

    assert_eq!(&rusk_vd, vd);
}

#[test]
fn vd_wfct() {
    let vd = verifier_data_wfct();
    let rusk_vd =
        verifier_data_bytes(&WithdrawFromTransparentCircuit::circuit_id());

    assert_eq!(&rusk_vd, vd);
}

#[test]
fn vd_wfco() {
    let vd = verifier_data_wfco();
    let rusk_vd =
        verifier_data_bytes(&WithdrawFromObfuscatedCircuit::circuit_id());

    assert_eq!(&rusk_vd, vd);
}

#[test]
fn vd_execute() {
    let variants = vec![
        (ExecuteCircuitOneTwo::circuit_id(), 1),
        (ExecuteCircuitTwoTwo::circuit_id(), 2),
        (ExecuteCircuitThreeTwo::circuit_id(), 3),
        (ExecuteCircuitFourTwo::circuit_id(), 4),
    ];

    for (id, inputs) in variants {
        let contract = verifier_data_execute(inputs)
            .expect("Specified number of inputs should exist")
            .to_vec();
        let rusk = verifier_data_bytes(&id);

        assert_eq!(rusk, contract);
    }
}
