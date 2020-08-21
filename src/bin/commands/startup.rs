use super::super::config::Config;
use rusk::basic_proto::echoer_server::EchoerServer;
use rusk::Rusk;
use tonic::transport::Server;

pub async fn startup(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let mut full_address = config.host_address.clone();
    full_address.extend(":".chars());
    full_address.extend(config.port.chars());
    let addr: std::net::SocketAddr = full_address.parse()?;
    let rusk = Rusk::default();

    Server::builder()
        .add_service(EchoerServer::new(rusk))
        .serve(addr)
        .await
        .unwrap();
    Ok(())
}
