// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Echo service implementation for the Rusk server.
mod score_gen_handler;
mod verify_score_handler;

use super::rusk_proto;
use crate::services::ServiceRequestHandler;
use crate::Rusk;
use dusk_blindbid::tree::BidTree;
use score_gen_handler::ScoreGenHandler;
use std::fs::{read, write};
use tonic::{Request, Response, Status};
use tracing::{info, warn};
use verify_score_handler::VerifyScoreHandler;

pub use super::rusk_proto::{
    GenerateScoreRequest, GenerateScoreResponse, VerifyScoreRequest,
    VerifyScoreResponse,
};

// Re-export the main types for BlindBid Service.
pub use rusk_proto::blind_bid_service_client::BlindBidServiceClient;
pub use rusk_proto::blind_bid_service_server::{
    BlindBidService, BlindBidServiceServer,
};

#[tonic::async_trait]
impl BlindBidService for Rusk {
    async fn generate_score(
        &self,
        request: Request<GenerateScoreRequest>,
    ) -> Result<Response<GenerateScoreResponse>, Status> {
        let handler = ScoreGenHandler::load_request(&request);
        info!("Recieved Score generation request");
        match handler.handle_request() {
            Ok(response) => {
                info!("Score generation request was successfully processed. Sending response..");
                Ok(response)
            }
            Err(e) => {
                warn!("An error ocurred during the Score generation request processing: {:?}", e);
                Err(e)
            }
        }
    }

    async fn verify_score(
        &self,
        request: Request<VerifyScoreRequest>,
    ) -> Result<Response<VerifyScoreResponse>, Status> {
        let handler = VerifyScoreHandler::load_request(&request);
        info!("Recieved Score Verification request");
        match handler.handle_request() {
            Ok(response) => {
                info!("Score verification request was successfully processed. Sending response..");
                Ok(response)
            }
            Err(e) => {
                warn!("An error ocurred during the Score verification processing: {:?}", e);
                Err(e)
            }
        }
    }
}

use canonical_host::MemStore;
use dusk_blindbid::bid::Bid;
use dusk_pki::{PublicSpendKey, SecretSpendKey};
use dusk_plonk::{
    bls12_381::BlsScalar,
    jubjub::{JubJubAffine, JubJubScalar},
};
use poseidon252::tree::PoseidonBranch;
// This function simulates the obtention of a Bid from the
// Bid contract storage and a PoseidonBranch that references it.
// For this function to work as a correct mocker, it always needs
// to recieve the idx 0.
//
// When we use this fn to generate a score, a bid gets saved so that
// we can successfully call verification rpc methods.
pub(crate) fn get_bid_storage_fields(
    idx: usize,
    secret: Option<JubJubAffine>,
    k: Option<BlsScalar>,
) -> Result<(Bid, PoseidonBranch<17>), std::io::Error> {
    const BID_FILE_PATH: &str = "bid.bin";
    let bid = match (secret.as_ref(), k) {
        (Some(secret), Some(k)) => {
            let pk_r = PublicSpendKey::from(SecretSpendKey::default());
            let stealth_addr = pk_r.gen_stealth_address(&JubJubScalar::random(
                &mut rand::thread_rng(),
            ));
            let bid = Bid::new(
                &mut rand::thread_rng(),
                &stealth_addr,
                &JubJubScalar::from(60_000u64),
                secret,
                k,
                -BlsScalar::from(99),
                -BlsScalar::from(99),
            )
            .expect("This should not fail");
            // Write bid to disk to "mock it" since storage is not persistent.
            write(BID_FILE_PATH, &bid.to_bytes()[..])?;
            bid
        }
        (_, _) => {
            // Read the bid from disk to have the same as the original one
            // since storage is not persistent atm.
            let bytes = read(BID_FILE_PATH)?;
            let mut buff: [u8; 328] = [0u8; 328];
            buff.copy_from_slice(&bytes[..]);
            Bid::from_bytes(buff)?
        }
    };

    let mut tree = BidTree::<MemStore>::new();
    let obtained_idx = tree.push(bid).ok().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "tree index obtention failed",
        )
    })?;
    assert_eq!(idx, obtained_idx as usize);
    let branch = tree.poseidon_branch(idx as usize).ok().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "missing branch in the extraction process.",
        )
    })?;
    let branch = branch.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "missing branch in the extraction process.",
        )
    })?;
    let extracted_bid = tree.get(idx as u64).ok().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Bid not found in the tree",
        )
    })?;
    let extracted_bid = extracted_bid.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "missing branch in the extraction process.",
        )
    })?;
    Ok((extracted_bid, branch))
}
