/*use super::*;
use crate::client;
use phoenix::{
  db, rpc::rusk_server::RuskServer, utils, zk, PublicKey, SecretKey,
  Transaction, TransparentNote,
};
use tonic::transport::Server;

#[tokio::test(threaded_scheduler)]
async fn test_transfer() {
  // Set DB_PATH
  let mut db_path = std::env::temp_dir();
  db_path.push("phoenix-db");
  std::env::set_var("PHOENIX_DB", db_path.into_os_string());

  // Mandatory Phoenix setup
  utils::init();
  zk::init();

  let srv = RuskServer::new(Rusk::default());
  let addr = "0.0.0.0:8080";

  tokio::spawn(async move {
    Server::builder()
      .add_service(srv)
      .serve(addr.parse().unwrap())
      .await
  });

  // TODO: maybe find a less hacky way to let the server get up and running
  std::thread::sleep(std::time::Duration::from_millis(1000));

  // First, credit the sender with a note, so that he can create a transaction from it
  let sk = SecretKey::default();
  let pk = sk.public_key();

  let mut tx = Transaction::default();
  let value = 100_000_000;
  let (note, blinding_factor) = TransparentNote::output(&pk, value);
  tx.push_output(note.to_transaction_output(value, blinding_factor, pk))
    .unwrap();
  db::store(
    std::path::Path::new(&std::env::var("PHOENIX_DB").unwrap()),
    &tx,
  )
  .unwrap();

  // Now, let's make a transaction
  let recipient = PublicKey::default();
  let tx = client::create_transaction(
    sk,
    100_000 as u64,
    100 as u64,
    recipient.into(),
  )
  .await
  .unwrap();

  // And execute it on the VM
  let response = client::validate_state_transition(tx).await.unwrap();

  println!("{:?}", response);

  // Clean up DB
  std::fs::remove_dir_all(std::path::Path::new(
    &std::env::var("PHOENIX_DB").unwrap(),
  ))
  .unwrap();
}
*/
