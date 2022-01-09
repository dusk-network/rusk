// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::TestContext;
use dusk_bytes::DeserializableSlice;
use dusk_pki::{PublicSpendKey, SecretSpendKey, ViewKey};
use dusk_plonk::prelude::*;
use rusk::services::rusk_proto::keys_client::KeysClient;
use rusk::services::rusk_proto::GenerateKeysRequest;
use test_context::test_context;
#[test_context(TestContext)]
#[tokio::test]
pub async fn pki_walkthrough_uds(
    ctx: &mut TestContext,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = KeysClient::new(ctx.channel.clone());
    // Key generation
    let request = tonic::Request::new(GenerateKeysRequest {});

    let response = client.generate_keys(request).await?.into_inner();

    let sk = response.sk.unwrap();
    // Make sure as well, that the keys are related.
    let a = JubJubScalar::from_slice(&sk.a).expect("Decoding error");
    let b = JubJubScalar::from_slice(&sk.b).expect("Decoding error");
    let sk = SecretSpendKey::new(a, b);

    let vk = response.vk.unwrap();
    let a = JubJubScalar::from_slice(&vk.a).expect("Decoding error");
    let b = JubJubExtended::from(
        JubJubAffine::from_slice(&vk.b_g).expect("Decoding error"),
    );
    let vk = ViewKey::new(a, b);

    let pk = response.pk.unwrap();
    let a = JubJubExtended::from(
        JubJubAffine::from_slice(&pk.a_g).expect("Decoding error"),
    );
    let b = JubJubExtended::from(
        JubJubAffine::from_slice(&pk.b_g).expect("Decoding error"),
    );
    let psk = PublicSpendKey::new(a, b);

    assert_eq!(sk.view_key(), vk);
    assert_eq!(sk.public_spend_key(), psk);

    // Stealth address generation
    let request = tonic::Request::new(pk);

    let _ = client.generate_stealth_address(request).await?;
    Ok(())
}
