// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::TestContext;
use rusk::services::echoer::{EchoRequest, EchoerClient};
use test_context::test_context;

#[test_context(TestContext)]
#[tokio::test]
pub async fn echo_works_uds(
    ctx: &mut TestContext,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = EchoerClient::new(ctx.channel.clone());

    // Actual test case.
    let message = "Test echo is working!";
    let request = tonic::Request::new(EchoRequest {
        message: message.into(),
    });

    let response = client.echo(request).await?;

    assert_eq!(response.into_inner().message, message);
    Ok(())
}
