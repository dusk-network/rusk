// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/*
pub mod encoding;
#[cfg(not(target_os = "windows"))]
pub mod unix;

use dusk_plonk::prelude::*;
use futures::prelude::*;
use futures::stream::TryStreamExt;
use rusk::services::blindbid::BlindBidServiceServer;
use rusk::services::echoer::EchoerServer;
use rusk::services::pki::KeysServer;
use rusk::Rusk;
use std::convert::TryFrom;
use std::path::Path;
use tokio::net::UnixListener;
use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint, Server, Uri};
use tower::service_fn;
use tracing::{subscriber, Level};
use tracing_subscriber::fmt::Subscriber;

/// Default UDS path that Rusk GRPC-server will connect to.
const SOCKET_PATH: &'static str = "/tmp/rusk_listener_pki";

pub async fn setup() -> Result<Channel, Box<dyn std::error::Error>> {
    // Generate a subscriber with the desired log level.
    let subscriber = Subscriber::builder().with_max_level(Level::INFO).finish();
    // Set the subscriber as global.
    // so this subscriber will be used as the default in all threads for the
    // remainder of the duration of the program, similar to how
    // `loggers` work in the `log` crate.
    subscriber::set_global_default(subscriber)
        .expect("Failed on subscribe tracing");

    // Create the server binded to the default UDS path.
    tokio::fs::create_dir_all(Path::new(SOCKET_PATH).parent().unwrap()).await?;

    let mut uds = UnixListener::bind(SOCKET_PATH)?;
    let rusk = Rusk::default();
    // We can't avoid the unwrap here until the async closure (#62290)
    // lands. And therefore we can force the closure to return a
    // Result. See: https://github.com/rust-lang/rust/issues/62290
    tokio::spawn(async move {
        Server::builder()
            .add_service(BlindBidServiceServer::new(rusk))
            .add_service(KeysServer::new(rusk))
            .add_service(EchoerServer::new(rusk))
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
    Ok(channel)
}
*/
