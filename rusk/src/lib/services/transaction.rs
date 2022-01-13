// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Transaction helper service implementation for the Rusk server.

use crate::Rusk;
use tonic::{Request, Response, Status};
use tracing::info;

pub use super::rusk_proto::{
    transaction_service_client::TransactionServiceClient,
    CoinbaseTransactionRequest, Transaction, TransactionRequest,
};

pub use super::rusk_proto::transaction_service_server::{
    TransactionService, TransactionServiceServer,
};

#[tonic::async_trait]
impl TransactionService for Rusk {
    async fn coinbase_transaction(
        &self,
        _: Request<CoinbaseTransactionRequest>,
    ) -> Result<Response<Transaction>, Status> {
        info!("Recieved coinbase_transaction request");
        Err(Status::unimplemented("Request not implemented"))
    }

    async fn new_transaction(
        &self,
        _: Request<TransactionRequest>,
    ) -> Result<Response<Transaction>, Status> {
        info!("Recieved coinbase_transaction request");
        Err(Status::unimplemented("Request not implemented"))
    }
}
