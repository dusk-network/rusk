// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

//! Staking infrastructure service implementation for the Rusk server.

use super::rusk_proto::{ExtendStakeRequest, Transaction};
use super::ServiceRequestHandler;
use tonic::{Request, Response, Status};

/// Implementation of the ExtendStake handler.
pub struct ExtendStakeHandler<'a> {
    _request: &'a Request<ExtendStakeRequest>,
}

impl<'a, 'b> ServiceRequestHandler<'a, 'b, ExtendStakeRequest, Transaction>
    for ExtendStakeHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<ExtendStakeRequest>) -> Self {
        Self { _request: request }
    }

    fn handle_request(&self) -> Result<Response<Transaction>, Status> {
        unimplemented!()
    }
}
