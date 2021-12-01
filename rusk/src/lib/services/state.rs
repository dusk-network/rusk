// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Rusk;

use tonic::{Request, Response, Status};
use tracing::info;

pub use super::rusk_proto::state_server::{State, StateServer};
pub use super::rusk_proto::{
    AcceptRequest, AcceptResponse, EchoRequest, EchoResponse,
    ExecuteStateTransitionRequest, ExecuteStateTransitionResponse,
    FinalizeRequest, FinalizeResponse, GetEphemeralStateRootRequest,
    GetEphemeralStateRootResponse, GetFinalizedStateRootRequest,
    GetFinalizedStateRootResponse, GetProvisionersRequest,
    GetProvisionersResponse, VerifyStateTransitionRequest,
    VerifyStateTransitionResponse,
};

#[tonic::async_trait]
impl State for Rusk {
    async fn echo(
        &self,
        _request: Request<EchoRequest>,
    ) -> Result<Response<EchoResponse>, Status> {
        info!("Received Echo request");

        Err(Status::unimplemented("Request not implemented"))
    }

    async fn execute_state_transition(
        &self,
        _request: Request<ExecuteStateTransitionRequest>,
    ) -> Result<Response<ExecuteStateTransitionResponse>, Status> {
        info!("Received ExecuteStateTransition request");

        Err(Status::unimplemented("Request not implemented"))
    }

    async fn verify_state_transition(
        &self,
        _request: Request<VerifyStateTransitionRequest>,
    ) -> Result<Response<VerifyStateTransitionResponse>, Status> {
        info!("Received VerifyStateTransition request");

        Err(Status::unimplemented("Request not implemented"))
    }

    async fn accept(
        &self,
        _request: Request<AcceptRequest>,
    ) -> Result<Response<AcceptResponse>, Status> {
        info!("Received Accept request");

        Err(Status::unimplemented("Request not implemented"))
    }

    async fn finalize(
        &self,
        _request: Request<FinalizeRequest>,
    ) -> Result<Response<FinalizeResponse>, Status> {
        info!("Received Finalize request");

        Err(Status::unimplemented("Request not implemented"))
    }

    async fn get_provisioners(
        &self,
        _request: Request<GetProvisionersRequest>,
    ) -> Result<Response<GetProvisionersResponse>, Status> {
        info!("Received GetProvisioners request");

        Err(Status::unimplemented("Request not implemented"))
    }

    async fn get_ephemeral_state_root(
        &self,
        _request: Request<GetEphemeralStateRootRequest>,
    ) -> Result<Response<GetEphemeralStateRootResponse>, Status> {
        info!("Received GetEphemeralStateRoot request");

        Err(Status::unimplemented("Request not implemented"))
    }

    async fn get_finalized_state_root(
        &self,
        _request: Request<GetFinalizedStateRootRequest>,
    ) -> Result<Response<GetFinalizedStateRootResponse>, Status> {
        info!("Received GetFinalizedStateRoot request");

        Err(Status::unimplemented("Request not implemented"))
    }
}
