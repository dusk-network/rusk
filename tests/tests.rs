pub mod common;
pub mod services;

use futures::executor::block_on;
use tonic::transport::Channel;
#[tokio::test(threaded_scheduler)]
async fn rusk_integration_tests() {
    let channel = block_on(common::setup()).expect("Error on the test setup");
    // Blindbid walkthrough tests
    //blindbid_service::walkthrough_works(channel)?;
    // Pki walkthrough tests
    assert!(services::pki_service::pki_walkthrough_uds(channel.clone())
        .await
        .is_ok());
    // Echo ping test
    assert!(services::echo_service::echo_works_uds(channel)
        .await
        .is_ok());
}
