// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::super::ServiceRequestHandler;
use super::{GenerateScoreRequest, GenerateScoreResponse};
use crate::encoding::{decode_affine, decode_bls_scalar};
use anyhow::Result;
use dusk_blindbid::{bid::Bid, BlindBidCircuit};
use dusk_plonk::jubjub::JubJubAffine;
use dusk_plonk::prelude::*;
use poseidon252::tree::PoseidonBranch;
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
        // FIXME: Once Bid contract is ready this will be implementable.
        let (bid, branch): (Bid, PoseidonBranch<17>) = unimplemented!();

        // Generate Score for the Bid
        let latest_consensus_round = self.request.get_ref().round as u64;
        let latest_consensus_step = self.request.get_ref().step as u64;
        let score = bid
            .compute_score(
                &secret,
                k,
                branch.root(),
                seed.reduce().0[0],
                latest_consensus_round,
                latest_consensus_step,
            )
            .map_err(|e| Status::new(Code::Unknown, format!("{}", e)))?;
        // Generate Prover ID
        let prover_id = bid.generate_prover_id(
            k,
            seed,
            BlsScalar::from(latest_consensus_round),
            BlsScalar::from(latest_consensus_step),
        );
        // Generate Blindbid proof proving that the generated `Score` is
        // correct.
        let mut circuit = BlindBidCircuit {
            bid,
            score,
            secret_k: k,
            secret,
            seed,
            latest_consensus_round: BlsScalar::from(latest_consensus_round),
            latest_consensus_step: BlsScalar::from(latest_consensus_step),
            branch: &branch,
            trim_size: 1 << 15,
            pi_positions: vec![],
        };
        let proof = gen_blindbid_proof(&mut circuit)
            .map_err(|e| Status::new(Code::Unknown, format!("{}", e)))?;
        Ok(Response::new(GenerateScoreResponse {
            blindbid_proof: proof.to_bytes().to_vec(),
            score: score.score.to_bytes().to_vec(),
            prover_identity: prover_id.to_bytes().to_vec(),
        }))
    }
}

// Parses the optional inputs of the GenerateScoreRequest returning an error if
// any of them isn't present (is `None`).
fn parse_score_gen_params(
    request: &Request<GenerateScoreRequest>,
) -> Result<(BlsScalar, BlsScalar, JubJubAffine), Status> {
    let k = decode_bls_scalar(&request.get_ref().k[..])?;
    let seed = decode_bls_scalar(&request.get_ref().seed[..])?;
    let secret = decode_affine(&request.get_ref().secret[..])?;
    Ok((k, seed, secret))
}

// Generate a blindbid proof given a circuit instance loaded with the
// desired inputs.
fn gen_blindbid_proof(circuit: &mut BlindBidCircuit) -> Result<Proof> {
    // Read ProverKey of the circuit.
    let prover_key = rusk_profile::keys_for("dusk-blindbid")
        .get_prover("blindbid")
        .expect("Failed to get blindbid circuit keys from rusk_profile.");

    let prover_key = ProverKey::from_bytes(&prover_key[..])?;
    // Generate a proof using the circuit
    circuit.gen_proof(&crate::PUB_PARAMS, &prover_key, b"BlindBid")
}
