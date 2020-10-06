// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod encoding;
#[cfg(not(target_os = "windows"))]
mod unix;
use dusk_plonk::prelude::*;
use futures::stream::TryStreamExt;
use rusk::services::blindbid::{
    BlindBidServiceClient, BlindBidServiceServer, GenerateScoreRequest,
    GenerateScoreResponse, VerifyScoreRequest, VerifyScoreResponse,
};
use rusk::Rusk;
use std::convert::TryFrom;
use std::path::Path;
use tokio::net::UnixListener;
use tokio::net::UnixStream;
use tonic::transport::Server;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;
use tracing::{subscriber, Level};
use tracing_subscriber::fmt::Subscriber;

/// Default UDS path that Rusk GRPC-server will connect to.
const SOCKET_PATH: &'static str = "/tmp/rusk_listener_blindbid";
const SERVER_ADDRESS: &'static str = "127.0.0.1:50051";
const CLIENT_ADDRESS: &'static str = "http://127.0.0.1:50051";

#[cfg(test)]
mod blindbid_service_tests {
    use super::*;

    #[tokio::test(threaded_scheduler)]
    async fn walkthrough_works() -> Result<(), Box<dyn std::error::Error>> {
        // Generate a subscriber with the desired log level.
        let subscriber =
            Subscriber::builder().with_max_level(Level::INFO).finish();
        // Set the subscriber as global.
        // so this subscriber will be used as the default in all threads for the
        // remainder of the duration of the program, similar to how
        // `loggers` work in the `log` crate.
        subscriber::set_global_default(subscriber)
            .expect("Failed on subscribe tracing");

        // Create the server binded to the default UDS path.
        tokio::fs::create_dir_all(Path::new(SOCKET_PATH).parent().unwrap())
            .await?;

        let mut uds = UnixListener::bind(SOCKET_PATH)?;
        let rusk = Rusk::default();
        // We can't avoid the unwrap here until the async closure (#62290)
        // lands. And therefore we can force the closure to return a
        // Result. See: https://github.com/rust-lang/rust/issues/62290
        tokio::spawn(async move {
            Server::builder()
                .add_service(BlindBidServiceServer::new(rusk))
                .serve_with_incoming(uds.incoming().map_ok(unix::UnixStream))
                .await
                .unwrap();
        });

        // Create the client binded to the default testing UDS path.
        let channel = Endpoint::try_from("http://[::]:50051")?
            .connect_with_connector(service_fn(|_: Uri| {
                // Connect to a Uds socket
                UnixStream::connect(SOCKET_PATH)
            }))
            .await?;

        // ------------------------------------------------------------ //
        //                                                              //
        //                                                              //
        //                      Actual Testcase                         //
        //                                                              //
        // ------------------------------------------------------------ //
        let mut client = BlindBidServiceClient::new(channel);
        // Declare the parameters needed to generate a blindbid proof which
        // were the ones used to generate the Bid that is now stored in the
        // Bid Tree.
        let request = tonic::Request::new(GenerateScoreRequest {
            k: BlsScalar::one().to_bytes().to_vec(),
            seed: BlsScalar::one().to_bytes().to_vec(),
            secret: dusk_plonk::jubjub::GENERATOR.to_bytes().to_vec(),
            round: 1000u32,
            step: 1000u32,
            index_stored_bid: 0u64,
        });
        let response = client.generate_score(request).await?;
        let proof = &response.get_ref().blindbid_proof;
        let score = &response.get_ref().score;
        let prover_id = &response.get_ref().prover_identity;

        let verify_request = tonic::Request::new(VerifyScoreRequest {
            proof: proof.clone(),
            score: score.clone(),
            seed: BlsScalar::one().to_bytes().to_vec(),
            prover_id: prover_id.clone(),
            round: 1000u64,
            step: 1000u32,
            index_stored_bid: 0u64,
        });
        let verify_response = client.verify_score(verify_request).await?;
        assert_eq!(verify_response.get_ref().success, true);
        Ok(())
    }
}
