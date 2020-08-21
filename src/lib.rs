mod server;

use server::basic_proto::echoer_server::EchoerServer;
use server::Rusk;
use tonic::transport::Server;

pub async fn startup(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let addr = addr.parse()?;
    let rusk = Rusk::default();

    Server::builder()
        .add_service(EchoerServer::new(rusk))
        .serve(addr)
        .await
        .unwrap();
    println!("Server up!");
    Ok(())
}
