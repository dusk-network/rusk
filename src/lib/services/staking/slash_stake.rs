// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

//! Staking infrastructure service implementation for the Rusk server.

use super::rusk_proto::{SlashRequest, Transaction};
use super::ServiceRequestHandler;
use tonic::{Request, Response, Status};

/// Implementation of the SlashStake handler.
pub struct SlashStakeHandler<'a> {
    _request: &'a Request<SlashRequest>,
}

impl<'a, 'b> ServiceRequestHandler<'a, 'b, SlashRequest, Transaction>
    for SlashStakeHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<SlashRequest>) -> Self {
        Self { _request: request }
    }

    fn handle_request(&self) -> Result<Response<Transaction>, Status> {
        unimplemented!()
    }
}
