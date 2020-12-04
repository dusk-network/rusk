// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod encoding;
#[cfg(not(target_os = "windows"))]
mod unix;
use dusk_pki::{jubjub_decode, PublicSpendKey, SecretSpendKey, ViewKey};
use dusk_plonk::jubjub::{JubJubAffine, JubJubExtended, JubJubScalar};
use futures::stream::TryStreamExt;
use rusk::services::pki::{GenerateKeysRequest, KeysClient, KeysServer};
use rusk::Rusk;
use std::convert::TryFrom;
use std::path::Path;
use tokio::net::UnixListener;
use tokio::net::UnixStream;
use tonic::transport::Server;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;
use tracing::{subscriber, Level};
use tracing_subscriber::fmt::Subscriber;

/// Default UDS path that Rusk GRPC-server will connect to.
const SOCKET_PATH: &'static str = "/tmp/rusk_listener_pki";

#[cfg(test)]
mod pki_service_tests {
    use super::*;

    #[tokio::test(threaded_scheduler)]
    async fn pki_works_uds() -> Result<(), Box<dyn std::error::Error>> {
        // Generate a subscriber with the desired log level.
        let subscriber =
            Subscriber::builder().with_max_level(Level::INFO).finish();
        // Set the subscriber as global.
        // so this subscriber will be used as the default in all threads for the
        // remainder of the duration of the program, similar to how
        // `loggers` work in the `log` crate.
        subscriber::set_global_default(subscriber)
            .expect("Failed on subscribe tracing");

        // Create the server binded to the default UDS path.
        tokio::fs::create_dir_all(Path::new(SOCKET_PATH).parent().unwrap())
            .await?;

        let mut uds = UnixListener::bind(SOCKET_PATH)?;
        let rusk = Rusk::default();
        // We can't avoid the unwrap here until the async closure (#62290)
        // lands. And therefore we can force the closure to return a
        // Result. See: https://github.com/rust-lang/rust/issues/62290
        tokio::spawn(async move {
            Server::builder()
                .add_service(KeysServer::new(rusk))
                .serve_with_incoming(uds.incoming().map_ok(unix::UnixStream))
                .await
                .unwrap();
        });

        // Create the client binded to the default testing UDS path.
        let channel = Endpoint::try_from("http://[::]:50051")?
            .connect_with_connector(service_fn(|_: Uri| {
                // Connect to a Uds socket
                UnixStream::connect(SOCKET_PATH)
            }))
            .await?;
        let mut client = KeysClient::new(channel);

        // Actual test case
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
}
