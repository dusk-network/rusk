// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

//! Public Key Infrastructure service implementation for the Rusk server.

use super::rusk_proto;
use super::ServiceRequestHandler;
use crate::encoding::decode_request_param;
use dusk_pki::{PublicSpendKey, SecretSpendKey, ViewKey};
use rand::thread_rng;
use tonic::{Code, Request, Response, Status};

// Re-export the main types for PKI Service.
pub use rusk_proto::keys_client::KeysClient;
pub use rusk_proto::keys_server::{Keys, KeysServer};
pub use rusk_proto::{
    GenerateKeysRequest, GenerateKeysResponse, PublicKey, StealthAddress,
};

/// Implementation of the ScoreGeneration Handler.
pub struct KeyGenHandler<'a> {
    request: &'a Request<GenerateKeysRequest>,
}

impl<'a, 'b>
    ServiceRequestHandler<'a, 'b, GenerateKeysRequest, GenerateKeysResponse>
    for KeyGenHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<GenerateKeysRequest>) -> Self {
        Self { request }
    }

    fn handle_request(&self) -> Result<Response<GenerateKeysResponse>, Status> {
        // We don't need to parse anything since this request does
        // not require any fields sent by the client.
        // Generate a random SecretKey
        let sk = SecretSpendKey::random(&mut thread_rng());
        // Derive PublicKey and ViewKey from SecretKey
        let pk = PublicSpendKey::from(sk);
        let vk = ViewKey::from(sk);
        Ok(Response::new(GenerateKeysResponse {
            sk: Some(sk),
            vk: Some(vk),
            pk: Some(pk),
        }))
    }
}
