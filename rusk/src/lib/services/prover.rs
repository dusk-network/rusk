// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Prover service implementation for the Rusk server.

mod handler;

use handler::ProverHandler;

use crate::services::{rusk_proto, ServiceRequestHandler};
use crate::Rusk;

use tonic::{Request, Response, Status};
use tracing::{error, info};

pub use rusk_proto::{
    prover_client::ProverClient,
    prover_server::{Prover, ProverServer},
    ProverRequest, ProverResponse,
};

#[tonic::async_trait]
impl Prover for Rusk {
    async fn prove_and_propagate(
        &self,
        request: Request<ProverRequest>,
    ) -> Result<Response<ProverResponse>, Status> {
        info!("Received Prove request");
        let handler = ProverHandler::load_request(&request);
        match handler.handle_request() {
            Ok(response) => {
                info!("Prove request was successfully processed. Sending response...");
                Ok(response)
            }
            Err(e) => {
                error!(
                    "An error ocurred processing the Prove request: {:?}",
                    e
                );
                Err(e)
            }
        }
    }
}
