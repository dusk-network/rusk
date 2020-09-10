// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.
//! Echo service implementation for the Rusk server.

mod score_gen_handler;

use super::rusk_proto;
use crate::services::ServiceRequestHandler;
use crate::Rusk;
use score_gen_handler::ScoreGenHandler;
use tonic::{Request, Response, Status};
use tracing::{info, warn};

pub use super::rusk_proto::{
    GenerateScoreRequest, GenerateScoreResponse, VerifyScoreRequest,
    VerifyScoreResponse,
};

// Re-export the main types for BlindBid Service.
use rusk_proto::blind_bid_service_server::BlindBidService;

#[tonic::async_trait]
impl BlindBidService for Rusk {
    async fn generate_score(
        &self,
        request: Request<GenerateScoreRequest>,
    ) -> Result<Response<GenerateScoreResponse>, Status> {
        let handler = ScoreGenHandler::load_request(&request);
        info!("Recieved Score generation request");
        match handler.handle_request() {
            Ok(response) => {
                info!("Score generation request was successfully processed. Sending response..");
                Ok(response)
            }
            Err(e) => {
                warn!("An error ocurred during the Score generation request processing: {:?}", e);
                Err(e)
            }
        }
    }

    async fn verify_score(
        &self,
        request: Request<VerifyScoreRequest>,
    ) -> Result<Response<VerifyScoreResponse>, Status> {
        unimplemented!()
    }
}
