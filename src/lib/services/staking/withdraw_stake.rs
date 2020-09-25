// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

//! Staking infrastructure service implementation for the Rusk server.

use super::rusk_proto::{Transaction, WithdrawStakeRequest};
use super::ServiceRequestHandler;
use tonic::{Request, Response, Status};

/// Implementation of the WithdrawStake handler.
pub struct WithdrawStakeHandler<'a> {
    _request: &'a Request<WithdrawStakeRequest>,
}

impl<'a, 'b> ServiceRequestHandler<'a, 'b, WithdrawStakeRequest, Transaction>
    for WithdrawStakeHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<WithdrawStakeRequest>) -> Self {
        Self { _request: request }
    }

    fn handle_request(&self) -> Result<Response<Transaction>, Status> {
        unimplemented!()
    }
}
