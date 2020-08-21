mod server;

use server::basic_proto::echoer_server::EchoerServer;
use server::Rusk;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let rusk = Rusk::default();

    Server::builder()
        .add_service(EchoerServer::new(rusk))
        .serve(addr)
        .await?;

    Ok(())
}
