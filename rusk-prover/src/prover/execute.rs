// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;
use phoenix_core::transaction::TRANSFER_TREE_DEPTH;

impl LocalProver {
    pub(crate) fn local_prove_execute(
        &self,
        utx_bytes: &[u8],
    ) -> Result<Vec<u8>, ProverError> {
        let utx = UnprovenTransaction::from_slice(utx_bytes)
            .map_err(|e| ProverError::invalid_data("utx", e))?;
        let num_inputs = utx.inputs().len();
        let num_outputs = utx.outputs().len();
        let mut circ = circuit_from_numbers(num_inputs, num_outputs).ok_or(
            ProverError::from(format!(
                "Invalid I/O count: {num_inputs}/{num_outputs}"
            )),
        )?;

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

            circ.add_input(cinput);
        }

        for (note, value, blinder) in utx.outputs() {
            circ.add_output_with_data(*note, *value, *blinder);
        }

        circ.set_tx_hash(utx.hash());

        match utx.crossover() {
            Some((crossover, value, blinder)) => {
                circ.set_fee_crossover(utx.fee(), crossover, *value, *blinder)
            }
            None => circ.set_fee(utx.fee()),
        }

        let keys = keys_for(circ.circuit_id()).map_err(|e| {
            ProverError::with_context("Cannot find keys for circuit", e)
        })?;
        let pk = keys.get_prover().map_err(|e| {
            ProverError::with_context("Cannot get provery key for circuit", e)
        })?;

        let (proof, _) = circ.prove(&mut OsRng, &pk).map_err(|e| {
            ProverError::with_context("Failed proving the circuit", e)
        })?;

        Ok(proof.to_bytes().to_vec())
    }
}

pub(crate) fn circuit_from_numbers(
    num_inputs: usize,
    num_outputs: usize,
) -> Option<ExecuteCircuit<(), TRANSFER_TREE_DEPTH, A>> {
    use ExecuteCircuit::*;

    match num_inputs {
        1 if num_outputs < 3 => Some(OneTwo(Default::default())),
        2 if num_outputs < 3 => Some(TwoTwo(Default::default())),
        3 if num_outputs < 3 => Some(ThreeTwo(Default::default())),
        4 if num_outputs < 3 => Some(FourTwo(Default::default())),
        _ => None,
    }
}
