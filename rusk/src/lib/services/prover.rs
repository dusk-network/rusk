// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Prover service implementation for the Rusk server.

mod execute;
mod stco;
mod stct;
mod wfco;
mod wfct;

use crate::services::rusk_proto;
use crate::{Result, PUB_PARAMS};

use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_pki::PublicSpendKey;
use once_cell::sync::Lazy;
use tonic::{Request, Response, Status};
use tracing::{error, info};

use dusk_plonk::prelude::*;
use dusk_schnorr::Signature;
use dusk_wallet_core::{Transaction, UnprovenTransaction};
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

#[derive(Debug, Default)]
pub struct RuskProver {}

impl RuskProver {
    pub fn preverify(tx: &Transaction) -> Result<bool> {
        let tx_hash = tx.hash();
        let inputs = tx.inputs();
        let outputs = tx.outputs();
        let proof = tx.proof();

        let circuit = circuit_from_numbers(inputs.len(), outputs.len())
            .ok_or_else(|| {
                Status::invalid_argument(format!(
                    "Expected: 0 < (inputs: {}) < 5, 0 ≤ (outputs: {}) < 3",
                    inputs.len(),
                    outputs.len()
                ))
            })?;

        let mut pi: Vec<rusk_abi::PublicInput> =
            Vec::with_capacity(5 + inputs.len() + 2 * outputs.len());

        pi.push(tx_hash.into());
        pi.push(tx.anchor().into());
        pi.extend(inputs.iter().map(|n| n.into()));

        pi.push(
            tx.crossover()
                .copied()
                .unwrap_or_default()
                .value_commitment()
                .into(),
        );

        let fee_value = tx.fee().gas_limit * tx.fee().gas_price;

        pi.push(fee_value.into());
        pi.extend(outputs.iter().map(|n| n.value_commitment().into()));

        let keys = rusk_profile::keys_for(circuit.circuit_id())?;
        let vd = &keys.get_verifier()?;

        // Maybe we want to handle internal serialization error too, currently
        // they map to `false`.
        Ok(rusk_abi::verify_proof(&PUB_PARAMS, proof, vd, pi).unwrap_or(false))
    }
}

#[tonic::async_trait]
impl Prover for RuskProver {
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
