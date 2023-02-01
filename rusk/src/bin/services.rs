// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg(not(target_os = "windows"))]
use super::unix;

use futures::TryFutureExt;
use rusk::services::version::RuskVersionLayer;
use std::path::Path;
use tokio::net::UnixListener;

use tonic::transport::server::Router;
use tower::layer::util::{Identity, Stack};

#[cfg(not(target_os = "windows"))]
pub(crate) async fn startup_with_uds(
    router: Router<Stack<RuskVersionLayer, Identity>>,
    socket: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::fs::create_dir_all(Path::new(socket).parent().unwrap()).await?;
    let uds = UnixListener::bind(socket)?;
    let incoming = {
        async_stream::stream! {
            loop {
                yield uds.accept().map_ok(|(st, _)| unix::UnixStream(st)).await
            }
        }
    };
    router.serve_with_incoming(incoming).await?;
    Ok(())
}

pub(crate) async fn startup_with_tcp_ip(
    router: Router<Stack<RuskVersionLayer, Identity>>,
    host: &str,
    port: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut full_address = host.to_string();
    full_address.push(':');
    full_address.push_str(port);
    let addr: std::net::SocketAddr = full_address.parse()?;

    Ok(router.serve(addr).await?)
}
