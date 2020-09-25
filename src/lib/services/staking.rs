// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.
//! Staking infrastructure service implementation for the Rusk server.

mod extend_stake;
mod find_stake;
mod new_stake;
mod slash_stake;
mod withdraw_stake;

use super::rusk_proto;
use crate::services::ServiceRequestHandler;
use crate::Rusk;
use extend_stake::ExtendStakeHandler;
use find_stake::FindStakeHandler;
use new_stake::NewStakeHandler;
use slash_stake::SlashStakeHandler;
use tonic::{Request, Response, Status};
use tracing::{error, info};
use withdraw_stake::WithdrawStakeHandler;

pub use super::rusk_proto::{
    stake_service_server::StakeService, ExtendStakeRequest, FindStakeRequest,
    FindStakeResponse, SlashRequest, StakeTransactionRequest,
    WithdrawStakeRequest, Transaction,
};

#[tonic::async_trait]
impl StakeService for Rusk {
    async fn new_stake(&self, request: Request<StakeTransactionRequest>) -> Result<Response<Transaction>, Status> {
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
    
    async fn find_stake(&self, request: Request<FindStakeRequest>) -> Result<Response<FindStakeResponse>, Status> {
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

    async fn extend_stake(&self, request: Request<ExtendStakeRequest>) -> Result<Response<Transaction>, Status> {
        let handler = ExtendStakeHandler::load_request(&request);
        info!("Received ExtendStake request");
        match handler.handle_request() {
            Ok(response) => {
                info!("ExtendStake request was successfully processed. Sending response..");
                Ok(response)
            }
            Err(e) => {
                error!("An error ocurred during the ExtendStake request processing: {:?}", e);
                Err(e)
            }
        }
    }

    async fn withdraw_stake(&self, request: Request<WithdrawStakeRequest>) -> Result<Response<Transaction>, Status> {
        let handler = WithdrawStakeHandler::load_request(&request);
        info!("Received WithdrawStake request");
        match handler.handle_request() {
            Ok(response) => {
                info!("WithdrawStake request was successfully processed. Sending response..");
                Ok(response)
            }
            Err(e) => {
                error!("An error ocurred during the WithdrawStake request processing: {:?}", e);
                Err(e)
            }
        }
    }

    async fn slash(&self, request: Request<SlashRequest>) -> Result<Response<Transaction>, Status> {
        let handler = SlashStakeHandler::load_request(&request);
        info!("Received Slash request");
        match handler.handle_request() {
            Ok(response) => {
                info!("Slash request was successfully processed. Sending response..");
                Ok(response)
            }
            Err(e) => {
                error!("An error ocurred during the Slash request processing: {:?}", e);
                Err(e)
            }
        }
    }
}
