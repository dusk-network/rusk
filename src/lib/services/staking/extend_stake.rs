// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

//! Staking infrastructure service implementation for the Rusk server.

use super::rusk_proto::{Bn256Point, ExtendStakeRequest, Transaction};
use super::ServiceRequestHandler;
use crate::encoding::decode_request_param;
use crate::types::BN256Point;
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
        let identifier: &[u8] = &self._request.get_ref().identifier;
        let pk: BN256Point = decode_request_param::<&Bn256Point, BN256Point>(
            self._request.get_ref().pk.as_ref().as_ref(),
        )?;

        let sig: BN256Point = decode_request_param::<&Bn256Point, BN256Point>(
            self._request.get_ref().sig.as_ref().as_ref(),
        )?;
        unimplemented!()
    }
}
