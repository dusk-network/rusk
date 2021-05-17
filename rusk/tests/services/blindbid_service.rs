// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.


mod common;
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
use tonic::transport::{Channel, Server};
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;
use tracing::{subscriber, Level};
use tracing_subscriber::fmt::Subscriber;

pub async fn gen_and_verify_blindbid(
    channel: Channel,
) -> Result<(), Box<dyn std::error::Error>> {
    // ------------------------------------------------------------ //
    //                                                              //
    //                                                              //
    //                      Actual Testcase                         //
    //                                                              //
    // ------------------------------------------------------------ //
    
    let mut  client = BlindBidServiceClient::new(channel);
    // Declare the parameters needed to generate a blindbid proof which
    // were the ones used to generate the Bid that is now stored in the
    // Bid Tree.

    // FIXME: Once Bid contract is implemented
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
