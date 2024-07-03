// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Prover service implementation for the Rusk server.

use crate::error::Error;
use crate::Result;

use execution_core::transfer::Transaction;
use rusk_profile::Circuit as CircuitProfile;

use std::sync::LazyLock;

pub static VD_EXEC_1_2: LazyLock<Vec<u8>> =
    LazyLock::new(|| fetch_verifier("ExecuteCircuitOneTwo"));

pub static VD_EXEC_2_2: LazyLock<Vec<u8>> =
    LazyLock::new(|| fetch_verifier("ExecuteCircuitTwoTwo"));

pub static VD_EXEC_3_2: LazyLock<Vec<u8>> =
    LazyLock::new(|| fetch_verifier("ExecuteCircuitThreeTwo"));

pub static VD_EXEC_4_2: LazyLock<Vec<u8>> =
    LazyLock::new(|| fetch_verifier("ExecuteCircuitFourTwo"));

pub fn verify_proof(tx: &Transaction) -> Result<bool> {
    let pi: Vec<rusk_abi::PublicInput> =
        tx.public_inputs().iter().map(|pi| pi.into()).collect();

    let inputs_len = tx.payload().tx_skeleton.nullifiers.len();
    let outputs_len = tx.payload().tx_skeleton.outputs.len();

    let vd = match inputs_len {
        1 => &VD_EXEC_1_2,
        2 => &VD_EXEC_2_2,
        3 => &VD_EXEC_3_2,
        4 => &VD_EXEC_4_2,
        _ => {
            return Err(Error::InvalidCircuitArguments(inputs_len, outputs_len))
        }
    };

    // Maybe we want to handle internal serialization error too, currently
    // they map to `false`.
    Ok(rusk_abi::verify_proof(vd.to_vec(), tx.proof().to_vec(), pi))
}

fn fetch_verifier(circuit_name: &str) -> Vec<u8> {
    let circuit_profile = CircuitProfile::from_name(circuit_name)
        .unwrap_or_else(|_| {
            panic!("There should be circuit data stored for {}", circuit_name)
        });
    circuit_profile.get_verifier().unwrap_or_else(|_| {
        panic!("there should be a verifier key stored for {}", circuit_name)
    })
}
