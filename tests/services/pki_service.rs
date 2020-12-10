// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::super::common::encoding::*;
use super::super::common::unix::*;
use dusk_pki::{jubjub_decode, PublicSpendKey, SecretSpendKey, ViewKey};
use dusk_plonk::jubjub::{JubJubAffine, JubJubExtended, JubJubScalar};
use futures::stream::TryStreamExt;
use rusk::services::rusk_proto::keys_client::KeysClient;
use rusk::services::rusk_proto::GenerateKeysRequest;
use rusk::Rusk;
use std::convert::TryFrom;
use std::path::Path;
use tokio::net::UnixListener;
use tokio::net::UnixStream;
use tonic::transport::{Channel, Server};
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;
use tracing::{subscriber, Level};
use tracing_subscriber::fmt::Subscriber;

pub async fn pki_walkthrough_uds(
    channel: Channel,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = KeysClient::new(channel);
    // Key generation
    let request = tonic::Request::new(GenerateKeysRequest {});

    let response = client.generate_keys(request).await?.into_inner();

    let sk = response.sk.unwrap();
    // Make sure as well, that the keys are related.
    let a = jubjub_decode::<JubJubScalar>(&sk.a).expect("Decoding error");
    let b = jubjub_decode::<JubJubScalar>(&sk.b).expect("Decoding error");
    let sk = SecretSpendKey::new(a, b);

    let vk = response.vk.unwrap();
    let a = jubjub_decode::<JubJubScalar>(&vk.a).expect("Decoding error");
    let b = JubJubExtended::from(
        jubjub_decode::<JubJubAffine>(&vk.b_g).expect("Decoding error"),
    );
    let vk = ViewKey::new(a, b);

    let pk = response.pk.unwrap();
    let a = JubJubExtended::from(
        jubjub_decode::<JubJubAffine>(&pk.a_g).expect("Decoding error"),
    );
    let b = JubJubExtended::from(
        jubjub_decode::<JubJubAffine>(&pk.b_g).expect("Decoding error"),
    );
    let psk = PublicSpendKey::new(a, b);

    assert_eq!(sk.view_key(), vk);
    assert_eq!(sk.public_key(), psk);

    // Stealth address generation
    let request = tonic::Request::new(pk);

    let _ = client.generate_stealth_address(request).await?;

    Ok(())
}
