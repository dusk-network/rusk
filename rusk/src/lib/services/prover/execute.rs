// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;

pub static EXECUTE_PROVER_KEYS: Lazy<HashMap<(usize, usize), ProverKey>> =
    Lazy::new(|| {
        let mut map = HashMap::new();

        for ninputs in [1, 2, 3, 4] {
            for noutputs in [0, 1, 2] {
                let circ = circuit_from_numbers(ninputs, noutputs)
                    .expect("circuit to exist");

                let keys =
                    keys_for(circ.circuit_id()).expect("keys to be available");
                let pk = keys.get_prover().expect("prover to be available");
                let pk =
                    ProverKey::from_slice(&pk).expect("prover key to be valid");

                map.insert((ninputs, noutputs), pk);
            }
        }

        map
    });

impl RuskProver {
    pub(crate) fn prove_execute(
        &self,
        request: &ExecuteProverRequest,
    ) -> Result<Response<ExecuteProverResponse>, Status> {
        let utx =
            UnprovenTransaction::from_slice(&request.utx).map_err(|_| {
                Status::invalid_argument("Failed parsing unproven TX")
            })?;

        let (num_inputs, num_outputs) =
            (utx.inputs().len(), utx.outputs().len());
        let mut circ = circuit_from_numbers(num_inputs, num_outputs)
            .ok_or_else(|| {
                Status::invalid_argument(format!(
                    "No circuit found for number of inputs {} and outputs {}",
                    num_inputs, num_outputs
                ))
            })?;

        let pk = EXECUTE_PROVER_KEYS
            .get(&(num_inputs, num_outputs))
            .ok_or_else(|| {
                Status::invalid_argument(format!(
                    "Couldn't find prover key for circuit with number of inputs {} and outputs {}",
                    num_inputs, num_outputs
                ))
            })?;

        for input in utx.inputs() {
            let cis = CircuitInputSignature::from(input.signature());
            let cinput = CircuitInput::new(
                input.opening().clone(),
                *input.note(),
                input.pk_r_prime().into(),
                input.value(),
                input.blinding_factor(),
                input.nullifier(),
                cis,
            );
            circ.add_input(cinput).map_err(|e| {
                Status::internal(format!(
                    "Failed adding input to circuit: {}",
                    e
                ))
            })?;
        }
        for (note, value, blinder) in utx.outputs() {
            circ.add_output_with_data(*note, *value, *blinder).map_err(
                |e| {
                    Status::internal(format!(
                        "Failed adding output to circuit: {}",
                        e
                    ))
                },
            )?;
        }

        circ.set_tx_hash(utx.hash());
        circ.set_fee(utx.fee()).map_err(|e| {
            Status::invalid_argument(format!("Failed setting fee: {}", e))
        })?;

        let (crossover, value, blinder) = utx.crossover();
        circ.set_fee_crossover(utx.fee(), crossover, *value, *blinder);

        let proof = circ.prove(&crate::PUB_PARAMS, pk).map_err(|e| {
            Status::invalid_argument(format!(
                "Failed proving transaction: {}",
                e
            ))
        })?;

        let tx = utx.prove(proof).to_bytes();

        Ok(Response::new(ExecuteProverResponse { tx }))
    }
}
