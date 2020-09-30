// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Bid service implementation for the Rusk server.

use super::rusk_proto;
use super::ServiceRequestHandler;
use tonic::{Request, Response, Status};

use rusk_proto::{BidTransaction, BidTransactionRequest};

/// Implementation of the NewBidHandler.
pub struct NewBidHandler<'a> {
    _request: &'a Request<BidTransactionRequest>,
}

impl<'a, 'b>
    ServiceRequestHandler<'a, 'b, BidTransactionRequest, BidTransaction>
    for NewBidHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<BidTransactionRequest>) -> Self {
        Self { _request: request }
    }

    fn handle_request(&self) -> Result<Response<BidTransaction>, Status> {
        unimplemented!()
    }
}
