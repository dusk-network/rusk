// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Staking infrastructure service implementation for the Rusk server.

mod find_stake;
mod new_stake;

use super::rusk_proto;
use crate::services::ServiceRequestHandler;
use crate::Rusk;
use find_stake::FindStakeHandler;
use new_stake::NewStakeHandler;
use tonic::{Request, Response, Status};
use tracing::{error, info};

pub use super::rusk_proto::{
    stake_service_server::StakeService, FindStakeRequest, FindStakeResponse,
    StakeTransactionRequest, Transaction,
};

#[tonic::async_trait]
impl StakeService for Rusk {
    async fn new_stake(
        &self,
        request: Request<StakeTransactionRequest>,
    ) -> Result<Response<Transaction>, Status> {
        let handler = NewStakeHandler::load_request(&request);
        info!("Received NewStake request");
        match handler.handle_request() {
            Ok(response) => {
                info!("NewStake request was successfully processed. Sending response..");
                Ok(response)
            }
            Err(e) => {
                error!("An error ocurred during the NewStake request processing: {:?}", e);
                Err(e)
            }
        }
    }

    async fn find_stake(
        &self,
        request: Request<FindStakeRequest>,
    ) -> Result<Response<FindStakeResponse>, Status> {
        let handler = FindStakeHandler::load_request(&request);
        info!("Received FindStake request");
        match handler.handle_request() {
            Ok(response) => {
                info!("FindStake request was successfully processed. Sending response..");
                Ok(response)
            }
            Err(e) => {
                error!("An error ocurred during the FindStake request processing: {:?}", e);
                Err(e)
            }
        }
    }
}
