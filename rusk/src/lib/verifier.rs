// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Prover service implementation for the Rusk server.

use crate::error::Error;
use crate::Result;

use dusk_core::transfer::{
    moonlight::Transaction as MoonlightTransaction,
    phoenix::Transaction as PhoenixTransaction,
};
use rusk_profile::Circuit as CircuitProfile;

use std::sync::LazyLock;

pub static VD_EXEC_1_2: LazyLock<Vec<u8>> =
    LazyLock::new(|| fetch_verifier("TxCircuitOneTwo"));

pub static VD_EXEC_2_2: LazyLock<Vec<u8>> =
    LazyLock::new(|| fetch_verifier("TxCircuitTwoTwo"));

pub static VD_EXEC_3_2: LazyLock<Vec<u8>> =
    LazyLock::new(|| fetch_verifier("TxCircuitThreeTwo"));

pub static VD_EXEC_4_2: LazyLock<Vec<u8>> =
    LazyLock::new(|| fetch_verifier("TxCircuitFourTwo"));

/// Verifies the proof of the incoming transaction.
pub fn verify_proof(tx: &PhoenixTransaction) -> Result<bool> {
    let inputs_len = tx.nullifiers().len();

    let vd = match inputs_len {
        1 => &VD_EXEC_1_2,
        2 => &VD_EXEC_2_2,
        3 => &VD_EXEC_3_2,
        4 => &VD_EXEC_4_2,
        _ => {
            return Err(Error::InvalidCircuitArguments(
                inputs_len,
                tx.outputs().len(),
            ))
        }
    };

    // Maybe we want to handle internal serialization error too,
    // currently they map to `false`.
    Ok(dusk_vm::verify_plonk(
        vd.to_vec(),
        tx.proof().to_vec(),
        tx.public_inputs(),
    ))
}

/// Verifies the signature of the incoming transaction.
pub fn verify_signature(tx: &MoonlightTransaction) -> Result<bool> {
    Ok(dusk_vm::verify_bls(
        tx.signature_message(),
        *tx.sender(),
        *tx.signature(),
    ))
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
