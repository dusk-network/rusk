// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk::services::echoer::{EchoRequest, EchoerClient};
use tonic::transport::Channel;

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
