use crate::config::default_ctants::{HOST_ADDRESS, PORT};
use rusk::services::echoer::basic_proto::echoer_server::EchoerServer;
use rusk::services::echoer::Rusk;
use tonic::transport::Server;

pub async fn startup(
    host: Option<&str>,
    port: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut full_address = host.unwrap_or_else(|| HOST_ADDRESS).to_string();
    full_address.extend(":".chars());
    full_address.extend(port.unwrap_or_else(|| PORT).to_string().chars());
    let addr: std::net::SocketAddr = full_address.parse()?;
    let rusk = Rusk::default();

    Server::builder()
        .add_service(EchoerServer::new(rusk))
        .serve(addr)
        .await
        .unwrap();
    Ok(())
}
