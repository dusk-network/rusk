// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Public Key Infrastructure service implementation for the Rusk server.

use super::rusk_proto;
use super::ServiceRequestHandler;
use dusk_pki::PublicSpendKey;
use dusk_plonk::jubjub::JubJubScalar;
use std::convert::TryInto;
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
        let pk: PublicSpendKey = self._request.get_ref().try_into()?;

        // Compute a stealth address.
        // First, we need to generate a random scalar.
        let stealth_address = pk.gen_stealth_address(&JubJubScalar::random(
            &mut rand::thread_rng(),
        ));
        Ok(Response::new(StealthAddress::from(stealth_address)))
    }
}
