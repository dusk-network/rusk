// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

//! Public Key Infrastructure service implementation for the Rusk server.

use super::rusk_proto;
use super::ServiceRequestHandler;
use crate::encoding::decode_request_param;
use dusk_pki::{PublicSpendKey, SecretSpendKey, ViewKey};
use rand::thread_rng;
use tonic::{Request, Response, Status};

// Re-export the needed types for PKI-GenStealthAddr Service.
pub use rusk_proto::{PublicKey, StealthAddress};

/// Implementation of the ScoreGeneration Handler.
pub struct StealthAddrGenHandler<'a> {
    _request: &'a Request<PublicKey>,
}

impl<'a, 'b> ServiceRequestHandler<'a, 'b, PublicKey, StealthAddress>
    for StealthAddrGenHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<PublicKey>) -> Self {
        Self { _request: request }
    }

    fn handle_request(&self) -> Result<Response<StealthAddress>, Status> {
        // Parse the request and try to decode the PublicKey.
        //let pk: PublicSpendKey = self._request.get_ref().try_into()?;
        unimplemented!()
    }
}
