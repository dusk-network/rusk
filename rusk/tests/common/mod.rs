// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod encoding;
#[cfg(not(target_os = "windows"))]
pub mod unix;

use super::{new_socket_path, SOCKET_DIR};

use futures::future::BoxFuture;
use std::convert::TryFrom;
use std::path::PathBuf;
use std::task::{Context, Poll};
use std::{fs, io};

use futures::TryFutureExt;
use microkelvin::{BackendCtor, DiskBackend};
use rusk::services::network::NetworkServer;
use rusk::services::network::RuskNetwork;
use rusk::services::pki::KeysServer;
use rusk::services::prover::ProverServer;
use rusk::{Result, Rusk};
use test_context::AsyncTestContext;
use tokio::net::{UnixListener, UnixStream};
use tonic::transport::{Channel, Endpoint, Server, Uri};
use tower::Service;
use tracing::{subscriber, Level};
use tracing_subscriber::fmt::Subscriber;

/// This function creates a temporary backend for testing purposes.
/// Calling `Rusk::with_backend()` with this function, will deploy a fresh
/// state with the genesis contracts in a temporary location.
pub fn testbackend() -> BackendCtor<DiskBackend> {
    BackendCtor::new(|| DiskBackend::ephemeral())
}

pub struct TestContext {
    pub channel: Channel,
    socket_path: PathBuf,
}

#[async_trait::async_trait]
impl AsyncTestContext for TestContext {
    async fn setup() -> TestContext {
        // Remove all unused files and tries to remove the directory - if
        // possible - and recreate it afterwards. This is required due
        // to the tear_down functions not being called when a test
        // panics.
        let _ = fs::remove_dir_all(&*SOCKET_DIR);
        fs::create_dir_all(&*SOCKET_DIR).expect("directory to be created");

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

        let socket_path = new_socket_path();

        let uds =
            UnixListener::bind(&socket_path).expect("Error binding the socket");

        let rusk = Rusk::with_backend(&testbackend())
            .expect("Error creating Rusk Instance");
        let network = RuskNetwork::default();

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
                .add_service(NetworkServer::new(network))
                .add_service(ProverServer::new(rusk))
                .serve_with_incoming(incoming)
                .await
        });

        // Create the client bound to the default testing UDS path.
        let channel = Endpoint::try_from("http://[::]:50051")
            .expect("Serde error on addr reading")
            .connect_with_connector(UdsConnector::from(socket_path.clone()))
            .await
            .expect("Error generating a Channel");

        TestContext {
            channel,
            socket_path,
        }
    }

    // Collection of actions that have to be done before any panics are
    // unwinded.
    async fn teardown(self) {
        std::fs::remove_file(self.socket_path).expect("Socket removal error");
    }
}

/// A connector to a UDS with a particular path.
struct UdsConnector {
    path: PathBuf,
}

impl From<PathBuf> for UdsConnector {
    fn from(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Service<Uri> for UdsConnector {
    type Response = UnixStream;
    type Error = io::Error;
    type Future = BoxFuture<'static, io::Result<UnixStream>>;

    fn poll_ready(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, _: Uri) -> Self::Future {
        Box::pin(UnixStream::connect(self.path.clone()))
    }
}
