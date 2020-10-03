// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::super::ServiceRequestHandler;
use super::get_bid_storage_fields;
use super::{GenerateScoreRequest, GenerateScoreResponse};
use crate::circuit_helpers::*;
use crate::encoding::{decode_request_param, encode_request_param};
use anyhow::Result;
use dusk_blindbid::BlindBidCircuit;
use dusk_plonk::jubjub::AffinePoint as JubJubAffine;
use dusk_plonk::prelude::*;
use tonic::{Code, Request, Response, Status};

/// Implementation of the ScoreGeneration Handler.
pub struct ScoreGenHandler<'a> {
    request: &'a Request<GenerateScoreRequest>,
}

impl<'a, 'b>
    ServiceRequestHandler<'a, 'b, GenerateScoreRequest, GenerateScoreResponse>
    for ScoreGenHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<GenerateScoreRequest>) -> Self {
        Self { request }
    }

    fn handle_request(
        &self,
    ) -> Result<Response<GenerateScoreResponse>, Status> {
        // Parse the optional request fields and return an error if
        // any of them is missing since all are required to compute
        // the score and the blindbid proof.
        let (k, seed, secret) = parse_score_gen_params(self.request)?;
        // XXX: We need to mock the storage for now, so we push to a PoseidonTree the Bid that we
        // should only need to retrieve. To do so, we need all of the parameters, not just the index.
        let (bid, branch) = get_bid_storage_fields(
            self.request.get_ref().index_stored_bid as usize,
            Some(secret),
            Some(k),
        )?;

        // Generate Score for the Bid
        let latest_consensus_round =
            BlsScalar::from(self.request.get_ref().round as u64);
        let latest_consensus_step =
            BlsScalar::from(self.request.get_ref().step as u64);
        let score = bid
            .compute_score(
                &secret,
                k,
                branch.root,
                seed,
                latest_consensus_round,
                latest_consensus_step,
            )
            .map_err(|e| Status::new(Code::Unknown, format!("{}", e)))?;
        // Generate Prover ID
        let prover_id = bid.generate_prover_id(
            k,
            seed,
            latest_consensus_round,
            latest_consensus_step,
        );
        // Generate Blindbid proof proving that the generated `Score` is correct.
        let mut circuit = BlindBidCircuit {
            bid: Some(bid),
            score: Some(score),
            secret_k: Some(k),
            secret: Some(secret),
            seed: Some(seed),
            latest_consensus_round: Some(latest_consensus_round),
            latest_consensus_step: Some(latest_consensus_step),
            branch: Some(&branch),
            size: 0,
            pi_constructor: None,
        };
        let proof = gen_blindbid_proof(&mut circuit)
            .map_err(|e| Status::new(Code::Unknown, format!("{}", e)))?;
        Ok(Response::new(GenerateScoreResponse {
            blindbid_proof: encode_request_param(&proof),
            score: encode_request_param(score.score),
            prover_identity: encode_request_param(prover_id),
        }))
    }
}

// Parses the optional inputs of the GenerateScoreRequest returning an error if
// any of them isn't present (is `None`).
fn parse_score_gen_params(
    request: &Request<GenerateScoreRequest>,
) -> Result<(BlsScalar, BlsScalar, JubJubAffine), Status> {
    let k: BlsScalar =
        decode_request_param(request.get_ref().k.as_ref().as_ref())?;
    let seed: BlsScalar =
        decode_request_param(request.get_ref().seed.as_ref().as_ref())?;
    let secret: JubJubAffine =
        decode_request_param(request.get_ref().secret.as_ref().as_ref())?;
    Ok((k, seed, secret))
}

// Generate a blindbid proof given a circuit instance loaded with the
// desired inputs.verifier_key
fn gen_blindbid_proof(circuit: &mut BlindBidCircuit) -> Result<Proof> {
    // Read ProverKey of the circuit.
    let prover_key = read_blindcid_circuit_pk()?;
    // Generate a proof using the circuit
    circuit.gen_proof(&crate::PUB_PARAMS, &prover_key, b"BlindBid")
}
