// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

//! Staking infrastructure service implementation for the Rusk server.

use super::rusk_proto::{Bn256Point, Note, Transaction, WithdrawStakeRequest};
use super::ServiceRequestHandler;
use crate::encoding::decode_request_param;
use crate::types::BN256Point;
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
        let identifier: &[u8] = &self._request.get_ref().identifier;
        let pk: BN256Point = decode_request_param::<&Bn256Point, BN256Point>(
            self._request.get_ref().pk.as_ref().as_ref(),
        )?;

        let sig: BN256Point = decode_request_param::<&Bn256Point, BN256Point>(
            self._request.get_ref().sig.as_ref().as_ref(),
        )?;

        /*
        let note = decode_request_param::<&Note, PhoenixNote>(
            self._request.get_ref().note.as_ref().as_ref(),
        )?;
        */
        unimplemented!()
    }
}
