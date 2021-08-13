// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Echo service implementation for the Rusk server.
mod score_gen_handler;
mod verify_score_handler;

use super::rusk_proto;
use crate::services::ServiceRequestHandler;
use crate::Rusk;
use score_gen_handler::ScoreGenHandler;
use tonic::{Request, Response, Status};
use tracing::{info, warn};
use verify_score_handler::VerifyScoreHandler;

pub use super::rusk_proto::{
    GenerateScoreRequest, GenerateScoreResponse, VerifyScoreRequest,
    VerifyScoreResponse,
};

// Re-export the main types for BlindBid Service.
pub use rusk_proto::blind_bid_service_client::BlindBidServiceClient;
pub use rusk_proto::blind_bid_service_server::{
    BlindBidService, BlindBidServiceServer,
};

pub(crate) const BLINDBID_TRANSCRIPT_INIT: &[u8] = b"dusk-network";

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
        let handler = VerifyScoreHandler::load_request(&request);
        info!("Recieved Score Verification request");
        match handler.handle_request() {
            Ok(response) => {
                info!("Score verification request was successfully processed. Sending response..");
                Ok(response)
            }
            Err(e) => {
                warn!("An error ocurred during the Score verification processing: {:?}", e);
                Err(e)
            }
        }
    }
}
