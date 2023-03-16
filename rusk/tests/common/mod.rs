// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod encoding;
pub mod keys;
pub mod state;
#[cfg(not(target_os = "windows"))]
pub mod unix;

use futures::future::BoxFuture;
use std::convert::TryFrom;
use std::io;
use std::path::PathBuf;
use std::task::{Context, Poll};
use tempfile::tempdir;

use futures::{Stream, TryFutureExt};
use rusk::Result;
use tokio::net::{UnixListener, UnixStream};
use tokio::runtime::Handle;
use tokio::task::block_in_place;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::Service;
use tracing::info;
use tracing_subscriber::EnvFilter;

pub trait Block {
    fn wait(self) -> <Self as futures::Future>::Output
    where
        Self: Sized,
        Self: futures::Future,
    {
        block_in_place(move || Handle::current().block_on(self))
    }
}

impl<F, T> Block for F where F: futures::Future<Output = T> {}

pub fn logger() {
    // Can't use `with_default_env` since we want to have a default
    // directive, and *then* apply the environment variable on top of it,
    // not the other way around.
    let directive = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "rusk=info,tests=info".to_string());

    let filter = EnvFilter::new(directive);
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

pub async fn setup() -> (
    Channel,
    impl Stream<Item = Result<unix::UnixStream, io::Error>>,
) where {
    logger();
    // Creates a temporary file for the socket
    let tempdir = tempdir().expect("failed to create tmp");
    let socket_path = tempdir.path().join("socket");

    info!("creating socket at {:?}", socket_path);

    let uds =
        UnixListener::bind(&socket_path).expect("Error binding the socket");

    let incoming = async_stream::stream! {
        loop {
            yield uds.accept().map_ok(|(st, _)| unix::UnixStream(st)).await
        }
    };
    // Create the client bound to the default testing UDS path.
    let channel = Endpoint::try_from("http://[::]:50051")
        .expect("Serde error on addr reading")
        .connect_with_connector(UdsConnector::from(socket_path.clone()))
        .await
        .expect("Error generating a Channel");

    (channel, incoming)
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
