// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Public Key infrastructure service implementation for the Rusk server.

mod execute;
mod verify;
mod provisioners;

use super::rusk_proto;
use crate::services::ServiceRequestHandler;
use crate::Rusk;
use tonic::{Request, Response, Status};
use tracing::{error, info};

pub use super::rusk_proto::{
    state_client::StateClient,
    state_server::{State, StateServer},
    VerifyStateTransitionRequest, VerifyStateTransitionResponse,
    ExecuteStateTransitionRequest, ExecuteStateTransitionResponse,
};

#[tonic::async_trait]
impl State for Rusk {
    async fn verify_state_transition(
        &self,
        request: Request<VerifyStateTransitionRequest>,
    ) -> Result<Response<VerifyStateTransitionResponse>, Status> {
        let handler = KeyGenHandler::load_request(&request);
        info!("Recieved KeyGen request");
        match handler.handle_request() {
            Ok(response) => {
                info!("KeyGen request was successfully processed. Sending response..");
                Ok(response)
            }
            Err(e) => {
                error!("An error ocurred during the KeyGen request processing: {:?}", e);
                Err(e)
            }
        }
    }

    async fn execute_state_transition(
        &self,
        request: Request<ExecuteStateTransitionRequest>,
    ) -> Result<Response<ExecuteStateTransitionResponse>, Status> {
        let handler = StealthAddrGenHandler::load_request(&request);
        info!("Recieved StealthAddrGen request");
        match handler.handle_request() {
            Ok(response) => {
                info!("StealthAddrGen request was successfully processed. Sending response..");
                Ok(response)
            }
            Err(e) => {
                error!("An error ocurred during the StealthAddrGen request processing: {:?}", e);
                Err(e)
            }
        }
    }

    async fn get_provisioners(
        &self,
        request: Request<PublicKey>,
    ) -> Result<Response<StealthAddress>, Status> {
        unimplemented!()
    }
}
