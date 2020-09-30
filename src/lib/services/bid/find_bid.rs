// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Bid service implementation for the Rusk server.

use super::rusk_proto;
use super::ServiceRequestHandler;
use crate::encoding::decode_request_param;
use dusk_pki::StealthAddress;
use tonic::{Request, Response, Status};

use rusk_proto::{BidList, FindBidRequest};

/// Implementation of the FindBidHandler.
pub struct FindBidHandler<'a> {
    _request: &'a Request<FindBidRequest>,
}

impl<'a, 'b> ServiceRequestHandler<'a, 'b, FindBidRequest, BidList>
    for FindBidHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<FindBidRequest>) -> Self {
        Self { _request: request }
    }

    fn handle_request(&self) -> Result<Response<BidList>, Status> {
        // Parse the request and try to decode the StealthAddress.
        let address: StealthAddress = decode_request_param::<
            &rusk_proto::StealthAddress,
            StealthAddress,
        >(
            self._request.get_ref().addr.as_ref().as_ref(),
        )?;

        // TODO: add storage fetch logic
        unimplemented!()
    }
}
