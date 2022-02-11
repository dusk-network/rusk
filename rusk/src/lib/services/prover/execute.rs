// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;

impl RuskProver {
    pub(crate) fn prove_execute(
        &self,
        request: &ExecuteProverRequest,
    ) -> Result<Response<ExecuteProverResponse>, Status> {
        let utx =
            UnprovenTransaction::from_slice(&request.utx).map_err(|_| {
                Status::invalid_argument("Failed parsing unproven TX")
            })?;

        let mut circ = ExecuteCircuit::default();

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

        if let Some((crossover, value, blinder)) = utx.crossover() {
            circ.set_fee_crossover(utx.fee(), crossover, *value, *blinder);
        } else {
            circ.set_fee(utx.fee()).map_err(|e| {
                Status::invalid_argument(format!("Failed setting fee: {}", e))
            })?;
        }

        let keys = rusk_profile::keys_for(circ.circuit_id())?;
        let pk = &keys.get_prover()?;
        let pk = ProverKey::from_slice(pk).unwrap();

        let proof = circ.prove(&crate::PUB_PARAMS, &pk).map_err(|e| {
            Status::invalid_argument(format!(
                "Failed proving transaction: {}",
                e
            ))
        })?;

        Ok(Response::new(ExecuteProverResponse {
            proof: proof.to_bytes().to_vec(),
        }))
    }
}
