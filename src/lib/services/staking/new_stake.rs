// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Staking infrastructure service implementation for the Rusk server.

use super::rusk_proto::{Bn256Point, StakeTransactionRequest, Transaction};
use super::ServiceRequestHandler;
use crate::encoding::decode_request_param;
use crate::types::BN256Point;
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
        let value: u64 = self._request.get_ref().value;
        let pk: BN256Point = decode_request_param::<&Bn256Point, BN256Point>(
            self._request.get_ref().public_key_bls.as_ref().as_ref(),
        )?;
        unimplemented!()
    }
}
