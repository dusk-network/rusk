//! Temporary server implementation for rusk

use tonic::{Request, Response, Status};

use basic_proto::echoer_server::Echoer;
use basic_proto::{EchoRequest, EchoResponse};

pub mod basic_proto {
    tonic::include_proto!("basic_proto");
}

#[derive(Debug, Default)]
pub struct Rusk {}

#[tonic::async_trait]
impl Echoer for Rusk {
    async fn echo(
        &self,
        request: Request<EchoRequest>, // Accept request of type EchoRequest
    ) -> Result<Response<EchoResponse>, Status> {
        // Return an instance of type EchoReply
        println!("Got a request: {:?}", request);

        let reply = EchoResponse {
            // We must use .into_inner() as the fields of gRPC requests and responses are private
            message: format!("{}", request.into_inner().message).into(),
        };

        Ok(Response::new(reply))
    }
}
