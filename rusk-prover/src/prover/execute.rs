// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;
use crate::prover::fetch_prover;
use dusk_wallet_core::UnprovenTransaction;
use phoenix_core::transaction::TRANSFER_TREE_DEPTH;
use transfer_circuits::{
    ExecuteCircuitFourTwo, ExecuteCircuitOneTwo, ExecuteCircuitThreeTwo,
    ExecuteCircuitTwoTwo,
};

pub static EXEC_1_2_PROVER: Lazy<PlonkProver> =
    Lazy::new(|| fetch_prover("ExecuteCircuitOneTwo"));

pub static EXEC_2_2_PROVER: Lazy<PlonkProver> =
    Lazy::new(|| fetch_prover("ExecuteCircuitTwoTwo"));

pub static EXEC_3_2_PROVER: Lazy<PlonkProver> =
    Lazy::new(|| fetch_prover("ExecuteCircuitThreeTwo"));

pub static EXEC_4_2_PROVER: Lazy<PlonkProver> =
    Lazy::new(|| fetch_prover("ExecuteCircuitFourTwo"));

fn fill_circuit<const I: usize>(
    circuit: &mut ExecuteCircuit<I, (), TRANSFER_TREE_DEPTH, 4>,
    utx: &UnprovenTransaction,
) -> Result<(), ProverError> {
    for input in utx.inputs() {
        let cis = CircuitInputSignature::from(input.signature());
        let cinput = CircuitInput::new(
            *input.opening(),
            *input.note(),
            input.pk_r_prime().into(),
            input.value(),
            input.blinding_factor(),
            input.nullifier(),
            cis,
        );

        circuit.add_input(cinput).map_err(|_| {
            ProverError::from(format!(
                "Too many inputs: given {}, expected {}.",
                utx.inputs().len(),
                I
            ))
        })?;
    }

    for (note, value, blinder) in utx.outputs() {
        circuit
            .add_output_with_data(*note, *value, *blinder)
            .map_err(|_| {
                ProverError::from(format!(
                    "Too many outputs: given {}, expected 2.",
                    utx.outputs().len(),
                ))
            })?;
    }

    circuit.set_tx_hash(utx.hash());

    match utx.crossover() {
        Some((crossover, value, blinder)) => {
            circuit.set_fee_crossover(utx.fee(), crossover, *value, *blinder)
        }
        None => circuit.set_fee(utx.fee()),
    }
    Ok(())
}

impl LocalProver {
    pub(crate) fn local_prove_execute(
        &self,
        circuit_inputs: &[u8],
    ) -> Result<Vec<u8>, ProverError> {
        let utx = UnprovenTransaction::from_slice(circuit_inputs)
            .map_err(|e| ProverError::invalid_data("utx", e))?;
        match utx.inputs().len() {
            1 => local_prove_exec_1_2(&utx),
            2 => local_prove_exec_2_2(&utx),
            3 => local_prove_exec_3_2(&utx),
            4 => local_prove_exec_4_2(&utx),
            _ => Err(ProverError::from(format!(
                "Invalid I/O count: {}/{}",
                utx.inputs().len(),
                utx.outputs().len()
            ))),
        }
    }
}

fn local_prove_exec_1_2(
    utx: &UnprovenTransaction,
) -> Result<Vec<u8>, ProverError> {
    const I: usize = 1;
    let mut circuit = ExecuteCircuitOneTwo::new();
    fill_circuit::<I>(&mut circuit, utx)?;

    let (proof, _) =
        EXEC_1_2_PROVER.prove(&mut OsRng, &circuit).map_err(|e| {
            ProverError::with_context("Failed proving the circuit", e)
        })?;
    Ok(proof.to_bytes().to_vec())
}

fn local_prove_exec_2_2(
    utx: &UnprovenTransaction,
) -> Result<Vec<u8>, ProverError> {
    const I: usize = 2;
    let mut circuit = ExecuteCircuitTwoTwo::new();
    fill_circuit::<I>(&mut circuit, utx)?;

    let (proof, _) =
        EXEC_2_2_PROVER.prove(&mut OsRng, &circuit).map_err(|e| {
            ProverError::with_context("Failed proving the circuit", e)
        })?;
    Ok(proof.to_bytes().to_vec())
}

fn local_prove_exec_3_2(
    utx: &UnprovenTransaction,
) -> Result<Vec<u8>, ProverError> {
    const I: usize = 3;
    let mut circuit = ExecuteCircuitThreeTwo::new();
    fill_circuit::<I>(&mut circuit, utx)?;

    let (proof, _) =
        EXEC_3_2_PROVER.prove(&mut OsRng, &circuit).map_err(|e| {
            ProverError::with_context("Failed proving the circuit", e)
        })?;
    Ok(proof.to_bytes().to_vec())
}

fn local_prove_exec_4_2(
    utx: &UnprovenTransaction,
) -> Result<Vec<u8>, ProverError> {
    const I: usize = 4;
    let mut circuit = ExecuteCircuitFourTwo::new();
    fill_circuit::<I>(&mut circuit, utx)?;

    let (proof, _) =
        EXEC_4_2_PROVER.prove(&mut OsRng, &circuit).map_err(|e| {
            ProverError::with_context("Failed proving the circuit", e)
        })?;
    Ok(proof.to_bytes().to_vec())
}
