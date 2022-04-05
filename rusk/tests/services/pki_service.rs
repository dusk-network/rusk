// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::common::setup;
use dusk_bytes::DeserializableSlice;
use dusk_pki::{PublicSpendKey, SecretSpendKey, ViewKey};
use rusk::services::pki::{KeysServer, RuskKeys};
use rusk_schema::keys_client::KeysClient;
use rusk_schema::GenerateKeysRequest;
use tonic::transport::Server;

#[tokio::test(flavor = "multi_thread")]
pub async fn pki_walkthrough_uds() -> Result<(), Box<dyn std::error::Error>> {
    let (channel, incoming) = setup().await;

    tokio::spawn(async move {
        Server::builder()
            .add_service(KeysServer::new(RuskKeys::default()))
            .serve_with_incoming(incoming)
            .await
    });

    let mut client = KeysClient::new(channel.clone());
    // Key generation
    let request = tonic::Request::new(GenerateKeysRequest {});

    let response = client.generate_keys(request).await?.into_inner();

    let sk = response.sk.unwrap();
    // Make sure as well, that the keys are related.
    let sk = SecretSpendKey::from_slice(&sk.payload).expect("Decoding error");

    let vk = response.vk.unwrap();
    let vk = ViewKey::from_slice(&vk.payload).expect("Decoding error");

    let pk = response.pk.unwrap();
    let psk = PublicSpendKey::from_slice(&pk.payload).expect("Decoding error");

    assert_eq!(sk.view_key(), vk);
    assert_eq!(sk.public_spend_key(), psk);

    // Stealth address generation
    let request = tonic::Request::new(pk);

    let _ = client.generate_stealth_address(request).await?;
    Ok(())
}
