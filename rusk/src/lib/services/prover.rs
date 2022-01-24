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

use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_pki::PublicSpendKey;
use once_cell::sync::Lazy;
use tonic::{Request, Response, Status};
use tracing::{error, info};

use dusk_plonk::prelude::*;
use dusk_schnorr::Signature;
use dusk_wallet_core::UnprovenTransaction;
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

#[derive(Debug, Default)]
pub struct RuskProver {}

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
