// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod encoding;
#[cfg(not(target_os = "windows"))]
pub mod unix;

use super::SOCKET_PATH;
use futures::TryFutureExt;
use rusk::services::echoer::EchoerServer;
use rusk::services::pki::KeysServer;
use rusk::Rusk;
use std::convert::TryFrom;
use test_context::AsyncTestContext;
use tokio::net::UnixListener;
use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint, Server, Uri};
use tower::service_fn;
use tracing::{subscriber, Level};
use tracing_subscriber::fmt::Subscriber;

pub struct TestContext {
    pub channel: Channel,
}

#[async_trait::async_trait]
impl AsyncTestContext for TestContext {
    async fn setup() -> TestContext {
        // Initialize the subscriber
        // Generate a subscriber with the desired log level.
        let subscriber =
            Subscriber::builder().with_max_level(Level::INFO).finish();

        // Set the subscriber as global.
        // So this subscriber will be used as the default in all tests for the
        // remainder of the duration of the program, similar to how
        // `loggers` work in the `log` crate.
        //
        // NOTE that since we're using a `setup` fn that gets executed after
        // each test execution, we simply ignore the error since only
        // the first call to this fn will succeed.
        let _ = subscriber::set_global_default(subscriber);

        let uds = UnixListener::bind(&*SOCKET_PATH)
            .expect("Error binding the socket");
        let rusk = Rusk::default();

        let incoming = async_stream::stream! {
            loop {
                yield uds.accept().map_ok(|(st, _)| unix::UnixStream(st)).await
            }
        };

        // We can't avoid the unwrap here until the async closure (#62290)
        // lands. And therefore we can force the closure to return a
        // Result. See: https://github.com/rust-lang/rust/issues/62290
        tokio::spawn(async move {
            Server::builder()
                .add_service(KeysServer::new(rusk))
                .add_service(EchoerServer::new(rusk))
                .serve_with_incoming(incoming)
                .await
                .unwrap();
        });

        // Create the client binded to the default testing UDS path.
        let channel = Endpoint::try_from("http://[::]:50051")
            .expect("Serde error on addr reading")
            .connect_with_connector(service_fn(move |_: Uri| {
                // Connect to a Uds socket
                UnixStream::connect(&*SOCKET_PATH)
            }))
            .await
            .expect("Error generating a Channel");

        TestContext { channel }
    }

    // Collection of actions that have to be done before any panics are
    // unwinded.
    async fn teardown(self) {
        std::fs::remove_file(&*SOCKET_PATH).expect("Socket removal error");
    }
}
