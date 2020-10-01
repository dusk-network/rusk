// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Transaction infrastructure service implementation for the Rusk server.

use super::rusk_proto;
use super::ServiceRequestHandler;
use tonic::{Request, Response, Status};

// Re-export the main types needed by PKI-GenerateKeys Service.
pub use rusk_proto::{Bn256Point, Transaction, TransactionRequest};

/// Implementation of the NewTransactionHandler.
pub struct NewTransactionHandler<'a> {
    _request: &'a Request<TransactionRequest>,
}

impl<'a, 'b> ServiceRequestHandler<'a, 'b, TransactionRequest, Transaction>
    for NewTransactionHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<TransactionRequest>) -> Self {
        Self { _request: request }
    }

    fn handle_request(&self) -> Result<Response<Transaction>, Status> {
        // Currently blocked by tx module
        unimplemented!()
    }
}
