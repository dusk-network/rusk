// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::{rusk_proto, ServiceRequestHandler};
use dusk_bytes::Serializable;

use dusk_plonk::prelude::*;
use dusk_wallet_core::UnprovenTransaction;
use lazy_static::lazy_static;
use rusk_profile::keys_for;
use rusk_proto::{ProverRequest, ProverResponse};
use tonic::{Request, Response, Status};
use transfer_circuits::{CircuitInput, CircuitInputSignature, ExecuteCircuit};

lazy_static! {
    static ref PP: PublicParameters = unsafe {
        let pp = rusk_profile::get_common_reference_string().unwrap();

        PublicParameters::from_slice_unchecked(pp.as_slice())
    };
}

pub struct ProverHandler<'a> {
    _request: &'a Request<ProverRequest>,
}

impl<'a, 'b> ServiceRequestHandler<'a, 'b, ProverRequest, ProverResponse>
    for ProverHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<ProverRequest>) -> Self {
        Self { _request: request }
    }

    fn handle_request(&self) -> Result<Response<ProverResponse>, Status> {
        let utx = UnprovenTransaction::from_slice(&self._request.get_ref().utx)
            .map_err(|_| {
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

        let keys = keys_for(circ.circuit_id()).map_err(|_| Status::failed_precondition(format!("Couldn't find keys for circuit with number of inputs {} and outputs {}", num_inputs, num_outputs)))?;
        let pk = keys.get_prover().map_err(|_| Status::internal(format!("Couldn't find prover key for circuit with number of inputs {} and outputs {}", num_inputs, num_outputs)))?;
        let pk = ProverKey::from_slice(&pk).map_err(|e| {
            Status::internal(format!("Prover key malformed: {}", e))
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

        let proof = circ.prove(&PP, &pk).map_err(|e| {
            Status::invalid_argument(format!(
                "Failed proving transaction: {}",
                e
            ))
        })?;

        let tx = utx.prove(proof).to_bytes();

        // PROPAGATION MUST BE DONE HERE

        Ok(Response::new(ProverResponse {}))
    }
}

fn circuit_from_numbers(
    num_inputs: usize,
    num_outputs: usize,
) -> Option<ExecuteCircuit> {
    use ExecuteCircuit::*;

    match (num_inputs, num_outputs) {
        (1, 0) => Some(ExecuteCircuitOneZero(Default::default())),
        (1, 1) => Some(ExecuteCircuitOneOne(Default::default())),
        (1, 2) => Some(ExecuteCircuitOneTwo(Default::default())),
        (2, 0) => Some(ExecuteCircuitTwoZero(Default::default())),
        (2, 1) => Some(ExecuteCircuitTwoOne(Default::default())),
        (2, 2) => Some(ExecuteCircuitTwoTwo(Default::default())),
        (3, 0) => Some(ExecuteCircuitThreeZero(Default::default())),
        (3, 1) => Some(ExecuteCircuitThreeOne(Default::default())),
        (3, 2) => Some(ExecuteCircuitThreeTwo(Default::default())),
        (4, 0) => Some(ExecuteCircuitFourZero(Default::default())),
        (4, 1) => Some(ExecuteCircuitFourOne(Default::default())),
        (4, 2) => Some(ExecuteCircuitFourTwo(Default::default())),
        _ => None,
    }
}
