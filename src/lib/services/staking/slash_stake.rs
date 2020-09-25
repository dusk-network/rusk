// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

//! Staking infrastructure service implementation for the Rusk server.

use super::rusk_proto::{
    BlsScalar as ProtoBlsScalar, Bn256Point, SlashRequest, Transaction,
};
use super::ServiceRequestHandler;
use crate::encoding::decode_request_param;
use crate::types::BN256Point;
use dusk_plonk::bls12_381::Scalar as BlsScalar;
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
        let pk: BN256Point = decode_request_param::<&Bn256Point, BN256Point>(
            self._request.get_ref().pk.as_ref().as_ref(),
        )?;

        let round: u64 = self._request.get_ref().round;
        let step: u32 = self._request.get_ref().step;

        let m1: BlsScalar = decode_request_param::<&ProtoBlsScalar, BlsScalar>(
            self._request.get_ref().message1.as_ref().as_ref(),
        )?;

        let m2: BlsScalar = decode_request_param::<&ProtoBlsScalar, BlsScalar>(
            self._request.get_ref().message2.as_ref().as_ref(),
        )?;

        let sig1: BN256Point = decode_request_param::<&Bn256Point, BN256Point>(
            self._request.get_ref().sig1.as_ref().as_ref(),
        )?;

        let sig2: BN256Point = decode_request_param::<&Bn256Point, BN256Point>(
            self._request.get_ref().sig2.as_ref().as_ref(),
        )?;
        unimplemented!()
    }
}
