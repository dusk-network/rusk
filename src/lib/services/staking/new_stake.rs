// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

//! Staking infrastructure service implementation for the Rusk server.

use super::rusk_proto::{StakeTransactionRequest, Transaction};
use super::ServiceRequestHandler;
use tonic::{Request, Response, Status};

/// Implementation of the NewStake handler.
pub struct NewStakeHandler<'a> {
    _request: &'a Request<StakeTransactionRequest>,
}

impl<'a, 'b> ServiceRequestHandler<'a, 'b, StakeTransactionRequest, Transaction>
    for NewStakeHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<StakeTransactionRequest>) -> Self {
        Self { _request: request }
    }

    fn handle_request(&self) -> Result<Response<Transaction>, Status> {
        unimplemented!()
    }
}
