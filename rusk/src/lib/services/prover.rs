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

use crate::Result;

const A: usize = 4;

use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_pki::PublicSpendKey;
use once_cell::sync::Lazy;
use tonic::{Request, Response, Status};
use tracing::{error, info};

use dusk_plonk::prelude::*;
use dusk_schnorr::Signature;
use dusk_wallet_core::{Transaction, UnprovenTransaction};
use phoenix_core::transaction::TRANSFER_TREE_DEPTH;
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
    CircuitInput, CircuitInputSignature, CircuitOutput, DeriveKey,
    ExecuteCircuit, SendToContractObfuscatedCircuit,
    SendToContractTransparentCircuit, StcoCrossover, StcoMessage, WfoChange,
    WfoCommitment, WithdrawFromObfuscatedCircuit,
    WithdrawFromTransparentCircuit,
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
        let tx_hash = rusk_abi::hash(tx.to_hash_input_bytes());

        let inputs = &tx.nullifiers;
        let outputs = &tx.outputs;
        let proof = &tx.proof;

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
        pi.push(tx.anchor.into());
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
                .map(|_| CircuitOutput::ZERO_COMMITMENT.into()),
        );

        let keys = keys_for(circuit.circuit_id())?;
        let vd = keys.get_verifier()?;

        // Maybe we want to handle internal serialization error too, currently
        // they map to `false`.
        Ok(rusk_abi::verify_proof(vd, proof.clone(), pi))
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
