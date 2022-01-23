// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg(not(target_os = "windows"))]
use super::unix;

use futures::TryFutureExt;
use std::path::Path;
use tokio::net::UnixListener;

use tonic::body::BoxBody;
use tonic::codegen::http::{Request, Response};
use tonic::codegen::Service;
use tonic::transport::server::Router;
use tonic::transport::{Body, NamedService};
type TonicError = Box<dyn std::error::Error + Send + Sync>;

#[cfg(not(target_os = "windows"))]

pub(crate) async fn startup_with_uds_test<S, A>(
    socket: &str,
    service: Router<S, A>,
) -> Result<(), Box<dyn std::error::Error>>
where
    A: Service<Request<Body>, Response = Response<BoxBody>>
        + Clone
        + Send
        + 'static,
    A::Future: Send + 'static,
    A::Error: Into<TonicError> + Send,
    S: Service<Request<Body>, Response = Response<BoxBody>>
        + NamedService
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    S::Error: Into<TonicError> + Send,
{
    tokio::fs::create_dir_all(Path::new(socket).parent().unwrap()).await?;
    let uds = UnixListener::bind(socket)?;
    let incoming = {
        async_stream::stream! {
            loop {
                yield uds.accept().map_ok(|(st, _)| unix::UnixStream(st)).await
            }
        }
    };
    service.serve_with_incoming(incoming).await?;
    Ok(())
}

pub(crate) async fn startup_with_tcp_ip<S, A>(
    host: &str,
    port: &str,
    service: Router<S, A>,
) -> Result<(), Box<dyn std::error::Error>>
where
    A: Service<Request<Body>, Response = Response<BoxBody>>
        + Clone
        + Send
        + 'static,
    A::Future: Send + 'static,
    A::Error: Into<TonicError> + Send,
    S: Service<Request<Body>, Response = Response<BoxBody>>
        + NamedService
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    S::Error: Into<TonicError> + Send,
{
    let mut full_address = host.to_string();
    full_address.push(':');
    full_address.push_str(port);
    let addr: std::net::SocketAddr = full_address.parse()?;

    Ok(service.serve(addr).await?)
}
