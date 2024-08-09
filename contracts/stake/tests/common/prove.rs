// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use execution_core::{
    plonk::{Circuit, Prover},
    transfer::phoenix::{
        InputNoteInfo, Prove, TxCircuit, TxCircuitVec, NOTES_TREE_DEPTH,
    },
};

use rand::rngs::StdRng;
use rand::SeedableRng;

pub struct CachedProver();

impl Prove for CachedProver {
    fn prove(circuit: TxCircuitVec) -> Vec<u8> {
        // fetch the prover from the cache and crate the circuit
        match circuit.input_notes_info.len() {
            1 => {
                let (circuit, circuit_name) = tx_circuit_1_2(
                    circuit
                );
                fetch_prover_and_prove(circuit, &circuit_name)
            }
            2 => {
                let (circuit, circuit_name) = tx_circuit_2_2(
                    circuit
                );
                fetch_prover_and_prove(circuit, &circuit_name)
            }
            3 => {
                let (circuit, circuit_name) = tx_circuit_3_2(
                    circuit
                );
                fetch_prover_and_prove(circuit, &circuit_name)
            }
            4 => {
                let (circuit, circuit_name) = tx_circuit_4_2(
                    circuit
                );
                fetch_prover_and_prove(circuit, &circuit_name)
            }
            _ => panic!("The `TxCircuit` is only implemented for 1, 2, 3 or 4 input-notes.")
        }
    }
}

fn fetch_prover_and_prove(
    circuit: impl Circuit,
    circuit_name: &str,
) -> Vec<u8> {
    let rng = &mut StdRng::seed_from_u64(0xbeef);

    let circuit_profile = rusk_profile::Circuit::from_name(circuit_name)
        .expect(&format!(
            "There should be circuit data stored for {}",
            circuit_name
        ));
    let (pk, _vd) = circuit_profile
        .get_keys()
        .expect(&format!("there should be keys stored for {}", circuit_name));

    let prover = Prover::try_from_bytes(pk).unwrap();

    let (proof, _pi) = prover
        .prove(rng, &circuit)
        .expect("The circuit should be correct");

    proof.to_bytes().into()
}

fn tx_circuit_1_2(
    circuit: TxCircuitVec,
) -> (TxCircuit<NOTES_TREE_DEPTH, 1>, String) {
    let input_notes_info: [InputNoteInfo<NOTES_TREE_DEPTH>; 1] = circuit
        .input_notes_info
        .try_into()
        .expect("There should be exactly one input");
    let circuit = TxCircuit {
        input_notes_info,
        output_notes_info: circuit.output_notes_info,
        payload_hash: circuit.payload_hash,
        root: circuit.root,
        deposit: circuit.deposit,
        max_fee: circuit.max_fee,
        sender_pk: circuit.sender_pk,
        signatures: circuit.signatures,
    };
    (circuit, "ExecuteCircuitOneTwo".into())
}

fn tx_circuit_2_2(
    circuit: TxCircuitVec,
) -> (TxCircuit<NOTES_TREE_DEPTH, 2>, String) {
    let input_notes_info: [InputNoteInfo<NOTES_TREE_DEPTH>; 2] = circuit
        .input_notes_info
        .try_into()
        .expect("There should be exactly two inputs");
    let circuit = TxCircuit {
        input_notes_info,
        output_notes_info: circuit.output_notes_info,
        payload_hash: circuit.payload_hash,
        root: circuit.root,
        deposit: circuit.deposit,
        max_fee: circuit.max_fee,
        sender_pk: circuit.sender_pk,
        signatures: circuit.signatures,
    };
    (circuit, "ExecuteCircuitTwoTwo".into())
}

fn tx_circuit_3_2(
    circuit: TxCircuitVec,
) -> (TxCircuit<NOTES_TREE_DEPTH, 3>, String) {
    let input_notes_info: [InputNoteInfo<NOTES_TREE_DEPTH>; 3] = circuit
        .input_notes_info
        .try_into()
        .expect("There should be exactly three inputs");
    let circuit = TxCircuit {
        input_notes_info,
        output_notes_info: circuit.output_notes_info,
        payload_hash: circuit.payload_hash,
        root: circuit.root,
        deposit: circuit.deposit,
        max_fee: circuit.max_fee,
        sender_pk: circuit.sender_pk,
        signatures: circuit.signatures,
    };
    (circuit, "ExecuteCircuitThreeTwo".into())
}

fn tx_circuit_4_2(
    circuit: TxCircuitVec,
) -> (TxCircuit<NOTES_TREE_DEPTH, 4>, String) {
    let input_notes_info: [InputNoteInfo<NOTES_TREE_DEPTH>; 4] = circuit
        .input_notes_info
        .try_into()
        .expect("There should be exactly four inputs");
    let circuit = TxCircuit {
        input_notes_info,
        output_notes_info: circuit.output_notes_info,
        payload_hash: circuit.payload_hash,
        root: circuit.root,
        deposit: circuit.deposit,
        max_fee: circuit.max_fee,
        sender_pk: circuit.sender_pk,
        signatures: circuit.signatures,
    };
    (circuit, "ExecuteCircuitFourTwo".into())
}
