// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Transaction infrastructure service implementation for the Rusk server.

mod new_transaction;

use super::rusk_proto;
use crate::services::ServiceRequestHandler;
use crate::Rusk;
use new_transaction::NewTransactionHandler;
use tonic::{Request, Response, Status};
use tracing::{error, info};

pub use super::rusk_proto::{
    transaction_service_server::TransactionService, TransactionRequest,
};

#[tonic::async_trait]
impl TransactionService for Rusk {
    async fn new_transaction(
        &self,
        request: Request<TransactionRequest>,
    ) -> Result<Response<Transaction>, Status> {
        let handler = NewTransactionHandler::load_request(&request);
        info!("Recieved NewTransaction request");
        match handler.handle_request() {
            Ok(response) => {
                info!("NewTransaction request was successfully processed. Sending response..");
                Ok(response)
            }
            Err(e) => {
                error!("An error ocurred during the NewTransaction request processing: {:?}", e);
                Err(e)
            }
        }
    }
}
