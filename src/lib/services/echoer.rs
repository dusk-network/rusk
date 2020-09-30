// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

//! Echo service implementation for the Rusk server.

use super::rusk_proto;
use crate::Rusk;
use tonic::{Request, Response, Status};
use tracing::info;

// Re-export the main types for Echoer Service.
pub use rusk_proto::echoer_client::EchoerClient;
pub use rusk_proto::echoer_server::{Echoer, EchoerServer};
pub use rusk_proto::{EchoRequest, EchoResponse};

#[tonic::async_trait]
impl Echoer for Rusk {
    async fn echo(
        &self,
        request: Request<EchoRequest>, // Accept request of type EchoRequest
    ) -> Result<Response<EchoResponse>, Status> {
        // Return an instance of type EchoReply
        info!("Got an ECHO request: {:?}", request);

        let reply = EchoResponse {
            // We must use .into_inner() as the fields of gRPC requests and responses are private
            message: format!("{}", request.into_inner().message).into(),
        };

        Ok(Response::new(reply))
    }
}
