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

use crate::{Result, PUB_PARAMS};

use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_pki::PublicSpendKey;
use once_cell::sync::Lazy;
use tonic::{Request, Response, Status};

use dusk_plonk::prelude::*;
use dusk_schnorr::Signature;
use dusk_wallet_core::{Transaction, UnprovenTransaction};
use phoenix_core::{Crossover, Fee, Message};
use rusk_profile::keys_for;
pub use rusk_schema::prover_client::ProverClient;
pub use rusk_schema::prover_server::{Prover, ProverServer};
pub use rusk_schema::{
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
            Vec::with_capacity(9 + inputs.len());

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
        pi.extend(
            (0usize..2usize.saturating_sub(outputs.len()))
                .map(|_| ExecuteCircuit::ZERO_COMMITMENT.into()),
        );

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
        self.prove_execute(request.get_ref())
    }

    async fn prove_stct(
        &self,
        request: Request<StctProverRequest>,
    ) -> Result<Response<StctProverResponse>, Status> {
        self.prove_stct(request.get_ref())
    }

    async fn prove_stco(
        &self,
        request: Request<StcoProverRequest>,
    ) -> Result<Response<StcoProverResponse>, Status> {
        self.prove_stco(request.get_ref())
    }

    async fn prove_wfct(
        &self,
        request: Request<WfctProverRequest>,
    ) -> Result<Response<WfctProverResponse>, Status> {
        self.prove_wfct(request.get_ref())
    }

    async fn prove_wfco(
        &self,
        request: Request<WfcoProverRequest>,
    ) -> Result<Response<WfcoProverResponse>, Status> {
        self.prove_wfco(request.get_ref())
    }
}

fn circuit_from_numbers(
    num_inputs: usize,
    num_outputs: usize,
) -> Option<ExecuteCircuit> {
    use ExecuteCircuit::*;

    match num_inputs {
        1 if num_outputs < 3 => Some(ExecuteCircuitOneTwo(Default::default())),
        2 if num_outputs < 3 => Some(ExecuteCircuitTwoTwo(Default::default())),
        3 if num_outputs < 3 => {
            Some(ExecuteCircuitThreeTwo(Default::default()))
        }
        4 if num_outputs < 3 => Some(ExecuteCircuitFourTwo(Default::default())),
        _ => None,
    }
}
