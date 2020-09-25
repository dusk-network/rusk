// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

//! Staking infrastructure service implementation for the Rusk server.

use super::rusk_proto::{Bn256Point, FindStakeRequest, FindStakeResponse};
use super::ServiceRequestHandler;
use crate::encoding::decode_request_param;
use crate::types::BN256Point;
use tonic::{Request, Response, Status};

/// Implementation of the FindStake handler.
pub struct FindStakeHandler<'a> {
    _request: &'a Request<FindStakeRequest>,
}

impl<'a, 'b> ServiceRequestHandler<'a, 'b, FindStakeRequest, FindStakeResponse>
    for FindStakeHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<FindStakeRequest>) -> Self {
        Self { _request: request }
    }

    fn handle_request(&self) -> Result<Response<FindStakeResponse>, Status> {
        let pk: BN256Point = decode_request_param::<&Bn256Point, BN256Point>(
            self._request.get_ref().pk.as_ref().as_ref(),
        )?;
        unimplemented!()
    }
}
