// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Public Key Infrastructure service implementation for the Rusk server.

use super::rusk_proto;
use super::ServiceRequestHandler;
use crate::encoding::encode_optional_request_param;
use dusk_pki::{PublicSpendKey, SecretSpendKey, ViewKey};
use rand::thread_rng;
use tonic::{Request, Response, Status};

// Re-export the main types needed by PKI-GenerateKeys Service.
pub use rusk_proto::{GenerateKeysRequest, GenerateKeysResponse};

/// Implementation of the ScoreGeneration Handler.
pub struct KeyGenHandler<'a> {
    _request: &'a Request<GenerateKeysRequest>,
}

impl<'a, 'b>
    ServiceRequestHandler<'a, 'b, GenerateKeysRequest, GenerateKeysResponse>
    for KeyGenHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<GenerateKeysRequest>) -> Self {
        Self { _request: request }
    }

    fn handle_request(&self) -> Result<Response<GenerateKeysResponse>, Status> {
        // We don't need to parse anything since this request does
        // not require any fields sent by the client.
        // Generate a random SecretKey
        let sk = SecretSpendKey::random(&mut thread_rng());
        // Derive PublicKey and ViewKey from SecretKey
        let pk = PublicSpendKey::from(sk);
        let vk = ViewKey::from(sk);
        // Encode parameters and send the response.
        Ok(Response::new(GenerateKeysResponse {
            sk: encode_optional_request_param(sk),
            vk: encode_optional_request_param(vk),
            pk: encode_optional_request_param(pk),
        }))
    }
}
