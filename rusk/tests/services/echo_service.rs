// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/*
use futures::stream::TryStreamExt;
use rusk::services::echoer::{EchoRequest, EchoerClient};
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

pub async fn echo_works_uds(
    channel: Channel,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = EchoerClient::new(channel);

    // Actual test case.
    let message = "Test echo is working!";
    let request = tonic::Request::new(EchoRequest {
        message: message.into(),
    });

    let response = client.echo(request).await?;

    assert_eq!(response.into_inner().message, message);

    Ok(())
}

#[tokio::test(threaded_scheduler)]
async fn echo_works_tcp_ip() -> Result<(), Box<dyn std::error::Error>> {
    let addr = SERVER_ADDRESS.parse()?;
    let rusk = Rusk::default();
    tokio::spawn(async move {
        Server::builder()
            .add_service(EchoerServer::new(rusk))
            .serve(addr)
            .await
            .unwrap()
    });
    let mut client = EchoerClient::connect(CLIENT_ADDRESS).await?;

    let message = "Test echo is working!";
    let request = tonic::Request::new(EchoRequest {
        message: message.into(),
    });

    let response = client.echo(request).await?;

    assert_eq!(response.into_inner().message, message);

    Ok(())
}
*/
