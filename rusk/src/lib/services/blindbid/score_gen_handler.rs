// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::super::ServiceRequestHandler;
use super::{GenerateScoreRequest, GenerateScoreResponse};
use crate::encoding;
use anyhow::Result;
use blindbid_circuits::BlindBidCircuit;
use dusk_blindbid::{Bid, Score};
use dusk_bls12_381::BlsScalar;
use dusk_bytes::DeserializableSlice;
use dusk_bytes::Serializable;
use dusk_plonk::jubjub::JubJubAffine;
use dusk_plonk::prelude::*;
use dusk_poseidon::tree::PoseidonBranch;
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

    #[allow(unreachable_code)]
    #[allow(unused_variables)]
    fn handle_request(
        &self,
    ) -> Result<Response<GenerateScoreResponse>, Status> {
        // Parse the optional request fields and return an error if
        // any of them is missing since all are required to compute
        // the score and the blindbid proof.
        // FIXME: `seed` should be sent as `u64`? No? What happens here?
        let (k, seed, secret) = parse_score_gen_params(self.request)?;
        // TODO: This should fetch the Bid from the tree once this
        // functionallity is enabled.
        let (bid, branch): (Bid, PoseidonBranch<17>) = unimplemented!();

        // Generate Score for the Bid
        let latest_consensus_round = self.request.get_ref().round as u64;
        let latest_consensus_step = self.request.get_ref().step as u64;
        let score = Score::compute(
            &bid,
            &secret,
            k,
            *branch.root(),
            seed,
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
        };
        let proof = gen_blindbid_proof(&mut circuit)
            .map_err(|e| Status::new(Code::Unknown, format!("{}", e)))?;
        Ok(Response::new(GenerateScoreResponse {
            blindbid_proof: proof.to_bytes().to_vec(),
            score: score.to_bytes().to_vec(),
            prover_identity: prover_id.to_bytes().to_vec(),
        }))
    }
}

// Parses the optional inputs of the GenerateScoreRequest returning an error if
// any of them isn't present (is `None`).
fn parse_score_gen_params(
    request: &Request<GenerateScoreRequest>,
) -> Result<(BlsScalar, BlsScalar, JubJubAffine), Status> {
    let k = encoding::as_status_err(BlsScalar::from_slice(
        &request.get_ref().k[..],
    ))?;
    let seed = encoding::as_status_err(BlsScalar::from_slice(
        &request.get_ref().seed[..],
    ))?;
    let secret = encoding::as_status_err(JubJubAffine::from_slice(
        &request.get_ref().secret[..],
    ))?;
    Ok((k, seed, secret))
}

// Generate a blindbid proof given a circuit instance loaded with the
// desired inputs.
fn gen_blindbid_proof(circuit: &mut BlindBidCircuit) -> Result<Proof> {
    // Read ProverKey of the circuit.
    let pk =
        rusk_profile::keys_for(&BlindBidCircuit::CIRCUIT_ID)?.get_prover()?;

    let prover_key = ProverKey::from_slice(&pk)?;
    // Generate a proof using the circuit
    circuit
        .gen_proof(
            &crate::PUB_PARAMS,
            &prover_key,
            super::BLINDBID_TRANSCRIPT_INIT,
        )
        .map_err(|e| anyhow::anyhow!("{:?}", e))
}
