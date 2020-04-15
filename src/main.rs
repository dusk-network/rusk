mod client;
mod server;

use tonic::transport::Server;

use phoenix::{rpc::rusk_server::RuskServer, utils, zk};
use server::Rusk;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set DB_PATH
    // TODO: this should not be a temp dir
    let mut db_path = std::env::temp_dir();
    db_path.push("phoenix-db");
    std::env::set_var("PHOENIX_DB", db_path.into_os_string());

    // Mandatory Phoenix setup
    utils::init();
    zk::init();

    let addr = "127.0.0.1:8080".parse().unwrap();
    println!("listening on {}...", addr);
    Server::builder()
        .add_service(RuskServer::new(Rusk::default()))
        .serve(addr)
        .await?;

    Ok(())
}
