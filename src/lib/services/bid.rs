// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Bid service implementation for the Rusk server.

mod find_bid;
mod new_bid;

use super::rusk_proto;
use crate::services::ServiceRequestHandler;
use crate::Rusk;
use find_bid::FindBidHandler;
use new_bid::NewBidHandler;
use tonic::{Request, Response, Status};
use tracing::{error, info};

pub use super::rusk_proto::{
    bid_service_server::BidService, BidList, BidTransaction,
    BidTransactionRequest, FindBidRequest,
};

#[tonic::async_trait]
impl BidService for Rusk {
    async fn new_bid(
        &self,
        request: Request<BidTransactionRequest>,
    ) -> Result<Response<BidTransaction>, Status> {
        let handler = NewBidHandler::load_request(&request);
        info!("Received NewBid request");
        match handler.handle_request() {
            Ok(response) => {
                info!("NewBid request was successfully processed. Sending response..");
                Ok(response)
            }
            Err(e) => {
                error!("An error ocurred during the NewBid request processing: {:?}", e);
                Err(e)
            }
        }
    }

    async fn find_bid(
        &self,
        request: Request<FindBidRequest>,
    ) -> Result<Response<BidList>, Status> {
        let handler = FindBidHandler::load_request(&request);
        info!("Received FindBid request");
        match handler.handle_request() {
            Ok(response) => {
                info!("FindBid request was successfully processed. Sending response..");
                Ok(response)
            }
            Err(e) => {
                error!("An error ocurred during the FindBid request processing: {:?}", e);
                Err(e)
            }
        }
    }
}
