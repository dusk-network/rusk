// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::error::Error;
use crate::Rusk;

use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_pki::ViewKey;
use phoenix_core::Note;

use tonic::{Request, Response, Status};
use tracing::info;

pub use super::rusk_proto::state_server::{State, StateServer};
pub use super::rusk_proto::{
    EchoRequest, EchoResponse, ExecuteStateTransitionRequest,
    ExecuteStateTransitionResponse, GetAnchorRequest, GetAnchorResponse,
    GetNotesOwnedByRequest, GetNotesOwnedByResponse, GetOpeningRequest,
    GetOpeningResponse, GetProvisionersRequest, GetProvisionersResponse,
    GetStateRootRequest, GetStateRootResponse, VerifyStateTransitionRequest,
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
        _request: Request<ExecuteStateTransitionRequest>,
    ) -> Result<Response<ExecuteStateTransitionResponse>, Status> {
        info!("Received Accept request");

        Err(Status::unimplemented("Request not implemented"))
    }

    async fn finalize(
        &self,
        _request: Request<ExecuteStateTransitionRequest>,
    ) -> Result<Response<ExecuteStateTransitionResponse>, Status> {
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

    async fn get_state_root(
        &self,
        _request: Request<GetStateRootRequest>,
    ) -> Result<Response<GetStateRootResponse>, Status> {
        info!("Received GetEphemeralStateRoot request");

        let state_root = self.state()?.root().to_vec();
        Ok(Response::new(GetStateRootResponse { state_root }))
    }

    async fn get_notes_owned_by(
        &self,
        request: Request<GetNotesOwnedByRequest>,
    ) -> Result<Response<GetNotesOwnedByResponse>, Status> {
        info!("Received GetNotesOwnedBy request");

        let vk = ViewKey::from_slice(&request.get_ref().vk)
            .map_err(Error::Serialization)?;

        let notes = self
            .state()?
            .fetch_notes(request.get_ref().height, &vk)?
            .iter()
            .map(|n| n.to_bytes().to_vec())
            .collect();
        Ok(Response::new(GetNotesOwnedByResponse { notes }))
    }

    async fn get_anchor(
        &self,
        _request: Request<GetAnchorRequest>,
    ) -> Result<Response<GetAnchorResponse>, Status> {
        info!("Received GetAnchor request");

        let anchor = self.state()?.fetch_anchor()?.to_bytes().to_vec();
        Ok(Response::new(GetAnchorResponse { anchor }))
    }

    async fn get_opening(
        &self,
        request: Request<GetOpeningRequest>,
    ) -> Result<Response<GetOpeningResponse>, Status> {
        info!("Received GetOpening request");

        let note = Note::from_slice(&request.get_ref().note)
            .map_err(Error::Serialization)?;

        let branch = self.state()?.fetch_opening(&note)?.to_bytes().to_vec();
        Ok(Response::new(GetOpeningResponse { branch }))
    }
}
