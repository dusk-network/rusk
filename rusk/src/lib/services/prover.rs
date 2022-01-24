// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Prover service implementation for the Rusk server.

mod handler;

use handler::{
    ExecuteProverHandler, StcoProverHandler, StctProverHandler,
    WfcoProverHandler, WfctProverHandler,
};

use crate::services::{rusk_proto, ServiceRequestHandler};
use crate::Rusk;

use tonic::{Request, Response, Status};
use tracing::{error, info};

pub use rusk_proto::{
    prover_client::ProverClient,
    prover_server::{Prover, ProverServer},
    ExecuteProverRequest, ExecuteProverResponse, StcoProverRequest,
    StcoProverResponse, StctProverRequest, StctProverResponse,
    WfcoProverRequest, WfcoProverResponse, WfctProverRequest,
    WfctProverResponse,
};

macro_rules! handle {
    ($req: ident, $handler: ty, $name: expr) => {{
        info!("Received {} request", $name);
        let handler = <$handler>::load_request(&$req);
        match handler.handle_request() {
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
        handle!(request, ExecuteProverHandler, "ProveExecute")
    }

    async fn prove_stct(
        &self,
        request: Request<StctProverRequest>,
    ) -> Result<Response<StctProverResponse>, Status> {
        handle!(request, StctProverHandler, "ProveStct")
    }

    async fn prove_stco(
        &self,
        request: Request<StcoProverRequest>,
    ) -> Result<Response<StcoProverResponse>, Status> {
        handle!(request, StcoProverHandler, "ProveStco")
    }

    async fn prove_wfct(
        &self,
        request: Request<WfctProverRequest>,
    ) -> Result<Response<WfctProverResponse>, Status> {
        handle!(request, WfctProverHandler, "ProveWfct")
    }

    async fn prove_wfco(
        &self,
        request: Request<WfcoProverRequest>,
    ) -> Result<Response<WfcoProverResponse>, Status> {
        handle!(request, WfcoProverHandler, "ProveWfco")
    }
}
