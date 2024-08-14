// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use execution_core::{
    plonk::{Error as PlonkError, Prover as PlonkProver},
    transfer::phoenix::{Prove, TxCircuit, TxCircuitVec, NOTES_TREE_DEPTH},
};
use once_cell::sync::Lazy;

use rand::rngs::StdRng;
use rand::SeedableRng;

static PHOENIX_TX_1_2_PROVER: Lazy<PlonkProver> =
    Lazy::new(|| fetch_prover("ExecuteCircuitOneTwo"));

static PHOENIX_TX_2_2_PROVER: Lazy<PlonkProver> =
    Lazy::new(|| fetch_prover("ExecuteCircuitTwoTwo"));

static PHOENIX_TX_3_2_PROVER: Lazy<PlonkProver> =
    Lazy::new(|| fetch_prover("ExecuteCircuitThreeTwo"));

static PHOENIX_TX_4_2_PROVER: Lazy<PlonkProver> =
    Lazy::new(|| fetch_prover("ExecuteCircuitFourTwo"));

fn fetch_prover(circuit_name: &str) -> PlonkProver {
    let circuit_profile = rusk_profile::Circuit::from_name(circuit_name)
        .unwrap_or_else(|_| {
            panic!("There should be circuit data stored for {}", circuit_name)
        });
    let pk = circuit_profile.get_prover().unwrap_or_else(|_| {
        panic!("there should be a prover key stored for {}", circuit_name)
    });

    PlonkProver::try_from_bytes(pk).expect("Prover key is expected to by valid")
}

pub struct CachedProver();

impl Prove for CachedProver {
    type Error = PlonkError;

    fn prove(circuit: TxCircuitVec) -> Result<Vec<u8>, Self::Error> {
        let rng = &mut StdRng::seed_from_u64(0xbeef);

        // fetch the prover from the cache and crate the circuit
        let (proof, _pi) = match circuit.input_notes_info.len() {
            1 => PHOENIX_TX_1_2_PROVER.prove(rng, &tx_circuit_1_2(circuit))?,
            2 => PHOENIX_TX_2_2_PROVER.prove(rng, &tx_circuit_2_2(circuit))?,
            3 => PHOENIX_TX_3_2_PROVER.prove(rng, &tx_circuit_3_2(circuit))?,
            4 => PHOENIX_TX_4_2_PROVER.prove(rng, &tx_circuit_4_2(circuit))?,
            _ => panic!(
                "The `TxCircuit` is only implemented for 1,
            2, 3 or 4 input-notes."
            ),
        };

        Ok(proof.to_bytes().to_vec())
    }
}

fn tx_circuit_1_2(circuit: TxCircuitVec) -> TxCircuit<NOTES_TREE_DEPTH, 1> {
    TxCircuit {
        input_notes_info: circuit
            .input_notes_info
            .try_into()
            .expect("There should be exactly one input"),
        output_notes_info: circuit.output_notes_info,
        payload_hash: circuit.payload_hash,
        root: circuit.root,
        deposit: circuit.deposit,
        max_fee: circuit.max_fee,
        sender_pk: circuit.sender_pk,
        signatures: circuit.signatures,
    }
}

fn tx_circuit_2_2(circuit: TxCircuitVec) -> TxCircuit<NOTES_TREE_DEPTH, 2> {
    TxCircuit {
        input_notes_info: circuit
            .input_notes_info
            .try_into()
            .expect("There should be exactly two inputs"),
        output_notes_info: circuit.output_notes_info,
        payload_hash: circuit.payload_hash,
        root: circuit.root,
        deposit: circuit.deposit,
        max_fee: circuit.max_fee,
        sender_pk: circuit.sender_pk,
        signatures: circuit.signatures,
    }
}

fn tx_circuit_3_2(circuit: TxCircuitVec) -> TxCircuit<NOTES_TREE_DEPTH, 3> {
    TxCircuit {
        input_notes_info: circuit
            .input_notes_info
            .try_into()
            .expect("There should be exactly three inputs"),
        output_notes_info: circuit.output_notes_info,
        payload_hash: circuit.payload_hash,
        root: circuit.root,
        deposit: circuit.deposit,
        max_fee: circuit.max_fee,
        sender_pk: circuit.sender_pk,
        signatures: circuit.signatures,
    }
}

fn tx_circuit_4_2(circuit: TxCircuitVec) -> TxCircuit<NOTES_TREE_DEPTH, 4> {
    TxCircuit {
        input_notes_info: circuit
            .input_notes_info
            .try_into()
            .expect("There should be exactly four inputs"),
        output_notes_info: circuit.output_notes_info,
        payload_hash: circuit.payload_hash,
        root: circuit.root,
        deposit: circuit.deposit,
        max_fee: circuit.max_fee,
        sender_pk: circuit.sender_pk,
        signatures: circuit.signatures,
    }
}
