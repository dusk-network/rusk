// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Reward contract infrastructure service implementation for the Rusk server.

use super::rusk_proto;
use super::ServiceRequestHandler;
use crate::types::BN256Point;
use std::convert::TryInto;
use tonic::{Request, Response, Status};

// Re-export the main types needed by PKI-GenerateKeys Service.
pub use rusk_proto::{Bn256Point, GetWithdrawalTimeResponse};

/// Implementation of the GetWithdrawalTimeHandler.
pub struct GetWithdrawalTimeHandler<'a> {
    _request: &'a Request<Bn256Point>,
}

impl<'a, 'b>
    ServiceRequestHandler<'a, 'b, Bn256Point, GetWithdrawalTimeResponse>
    for GetWithdrawalTimeHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<Bn256Point>) -> Self {
        Self { _request: request }
    }

    fn handle_request(
        &self,
    ) -> Result<Response<GetWithdrawalTimeResponse>, Status> {
        let pk: BN256Point = self._request.get_ref().try_into()?;

        unimplemented!()
    }
}
