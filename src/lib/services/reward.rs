// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Reward contract infrastructure service implementation for the Rusk server.

mod get_balance;
mod get_withdrawal_time;

use super::rusk_proto;
use crate::services::ServiceRequestHandler;
use crate::Rusk;
use get_balance::GetBalanceHandler;
use get_withdrawal_time::GetWithdrawalTimeHandler;
use tonic::{Request, Response, Status};
use tracing::{error, info};

pub use super::rusk_proto::{
    reward_server::Reward, Bn256Point, GetBalanceResponse,
    GetWithdrawalTimeResponse,
};

#[tonic::async_trait]
impl Reward for Rusk {
    async fn get_balance(
        &self,
        request: Request<Bn256Point>,
    ) -> Result<Response<GetBalanceResponse>, Status> {
        let handler = GetBalanceHandler::load_request(&request);
        info!("Recieved GetBalance request");
        match handler.handle_request() {
            Ok(response) => {
                info!("GetBalance request was successfully processed. Sending response..");
                Ok(response)
            }
            Err(e) => {
                error!("An error ocurred during the GetBalance request processing: {:?}", e);
                Err(e)
            }
        }
    }

    async fn get_withdrawal_time(
        &self,
        request: Request<Bn256Point>,
    ) -> Result<Response<GetWithdrawalTimeResponse>, Status> {
        let handler = GetWithdrawalTimeHandler::load_request(&request);
        info!("Recieved GetWithdrawalTime request");
        match handler.handle_request() {
            Ok(response) => {
                info!("GetWithdrawalTime request was successfully processed. Sending response..");
                Ok(response)
            }
            Err(e) => {
                error!("An error ocurred during the GetWithdrawalTime request processing: {:?}", e);
                Err(e)
            }
        }
    }
}
