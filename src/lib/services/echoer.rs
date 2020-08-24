//! Temporary server implementation for rusk

use crate::Rusk;
use tonic::{Request, Response, Status};
use tracing::info;

// Re-export the main types for Echoer Service.
pub use basic_proto::echoer_client::EchoerClient;
pub use basic_proto::echoer_server::{Echoer, EchoerServer};
pub use basic_proto::{EchoRequest, EchoResponse};

pub(self) mod basic_proto {
    tonic::include_proto!("basic_proto");
}

#[tonic::async_trait]
impl Echoer for Rusk {
    async fn echo(
        &self,
        request: Request<EchoRequest>, // Accept request of type EchoRequest
    ) -> Result<Response<EchoResponse>, Status> {
        // Return an instance of type EchoReply
        info!("Got a request: {:?}", request);

        let reply = EchoResponse {
            // We must use .into_inner() as the fields of gRPC requests and responses are private
            message: format!("{}", request.into_inner().message).into(),
        };

        Ok(Response::new(reply))
    }
}
