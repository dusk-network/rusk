use tonic::{transport::Server, Request, Response, Status};

use rusk::Rusk;
use phoenix::rpc::rusk_server::RuskServer;

const DB_PATH: &'static str = "/tmp/rusk-db";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("listening!");
    Server::builder()
        .add_service(RuskServer::new(Rusk::new(DB_PATH)))
        .serve("127.0.0.1:8080".parse().unwrap())
        .await?;

    println!("done!");

    Ok(())
}
