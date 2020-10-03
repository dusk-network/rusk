// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Public Key infrastructure service implementation for the Rusk server.

mod keygen;
mod stealth_gen;

use super::rusk_proto;
use crate::services::ServiceRequestHandler;
use crate::Rusk;
use keygen::KeyGenHandler;
use stealth_gen::StealthAddrGenHandler;
use tonic::{Request, Response, Status};
use tracing::{error, info};

pub use super::rusk_proto::{
    keys_client::KeysClient,
    keys_server::{Keys, KeysServer},
    GenerateKeysRequest, GenerateKeysResponse, PublicKey, StealthAddress,
};

#[tonic::async_trait]
impl Keys for Rusk {
    async fn generate_keys(
        &self,
        request: Request<GenerateKeysRequest>,
    ) -> Result<Response<GenerateKeysResponse>, Status> {
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

    async fn generate_stealth_address(
        &self,
        request: Request<PublicKey>,
    ) -> Result<Response<StealthAddress>, Status> {
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
}
