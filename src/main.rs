mod client;
mod server;

use tonic::transport::Server;

use phoenix::rpc::rusk_server::RuskServer;
use server::Rusk;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:8080".parse().unwrap();
    println!("listening on {}...", addr);
    Server::builder()
        .add_service(RuskServer::new(Rusk::default()))
        .serve(addr)
        .await?;

    Ok(())
}
