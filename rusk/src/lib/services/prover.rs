// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Prover service implementation for the Rusk server.

mod stco;
mod stct;
mod wfco;
mod wfct;

use crate::services::rusk_proto;
use crate::Rusk;

use dusk_bytes::DeserializableSlice;
use dusk_bytes::Serializable;
use dusk_pki::PublicSpendKey;
use tonic::{Request, Response, Status};
use tracing::{error, info};

use dusk_plonk::prelude::*;
use dusk_schnorr::Signature;
use dusk_wallet_core::UnprovenTransaction;
use lazy_static::lazy_static;
use phoenix_core::{Crossover, Fee, Message};
use rusk_profile::keys_for;
pub use rusk_proto::prover_client::ProverClient;
pub use rusk_proto::prover_server::{Prover, ProverServer};
pub use rusk_proto::{
    ExecuteProverRequest, ExecuteProverResponse, StcoProverRequest,
    StcoProverResponse, StctProverRequest, StctProverResponse,
    WfcoProverRequest, WfcoProverResponse, WfctProverRequest,
    WfctProverResponse,
};
use std::collections::HashMap;

use transfer_circuits::{
    CircuitInput, CircuitInputSignature, DeriveKey, ExecuteCircuit,
    SendToContractObfuscatedCircuit, SendToContractTransparentCircuit,
    StcoCrossover, StcoMessage, WfoChange, WfoCommitment,
    WithdrawFromObfuscatedCircuit, WithdrawFromTransparentCircuit,
};

macro_rules! handle {
    ($self: ident, $req: ident, $handler: ident, $name: expr) => {{
        info!("Received {} request", $name);
        match $self.$handler(&$req.get_ref()) {
            Ok(response) => {
                info!(
                    "{} was successfully processed. Sending response...",
                    $name
                );
                Ok(response)
            }
            Err(e) => {
                error!("An error occurred processing {}: {:?}", $name, e);
                Err(e)
            }
        }
    }};
}

lazy_static! {
    static ref EXECUTE_PROVER_KEYS: HashMap<(usize, usize), ProverKey> = {
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
    };
}

#[tonic::async_trait]
impl Prover for Rusk {
    async fn prove_execute(
        &self,
        request: Request<ExecuteProverRequest>,
    ) -> Result<Response<ExecuteProverResponse>, Status> {
        return handle!(self, request, prove_execute, "prove_execute");
    }

    async fn prove_stct(
        &self,
        request: Request<StctProverRequest>,
    ) -> Result<Response<StctProverResponse>, Status> {
        return handle!(self, request, prove_stct, "prove_stct");
    }

    async fn prove_stco(
        &self,
        request: Request<StcoProverRequest>,
    ) -> Result<Response<StcoProverResponse>, Status> {
        return handle!(self, request, prove_stco, "prove_stco");
    }

    async fn prove_wfct(
        &self,
        request: Request<WfctProverRequest>,
    ) -> Result<Response<WfctProverResponse>, Status> {
        return handle!(self, request, prove_wfct, "prove_wfct");
    }

    async fn prove_wfco(
        &self,
        request: Request<WfcoProverRequest>,
    ) -> Result<Response<WfcoProverResponse>, Status> {
        return handle!(self, request, prove_wfco, "prove_wfco");
    }
}

impl Rusk {
    fn prove_execute(
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

        let proof = circ.prove(&crate::PUB_PARAMS, &pk).map_err(|e| {
            Status::invalid_argument(format!(
                "Failed proving transaction: {}",
                e
            ))
        })?;

        let tx = utx.prove(proof).to_bytes().map_err(|e| {
            Status::internal(format!("Failed converting tx to bytes: {:?}", e))
        })?;

        // PROPAGATION MUST BE DONE HERE

        Ok(Response::new(ExecuteProverResponse { tx }))
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
