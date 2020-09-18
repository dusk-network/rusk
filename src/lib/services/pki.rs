// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.
//! Echo service implementation for the Rusk server.

mod keygen;

use super::rusk_proto;
use crate::services::ServiceRequestHandler;
use crate::Rusk;
use keygen::KeyGenHandler;
use tonic::{Request, Response, Status};
use tracing::{error, info};

pub use super::rusk_proto::{
    keys_server::Keys, GenerateKeysRequest, GenerateKeysResponse, PublicKey,
    StealthAddress,
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
        unimplemented!()
    }
}
