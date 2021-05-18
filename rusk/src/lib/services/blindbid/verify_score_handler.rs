// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::super::ServiceRequestHandler;
use super::{VerifyScoreRequest, VerifyScoreResponse};
use crate::encoding;
use anyhow::Result;
use blindbid_circuits::BlindBidCircuit;
use dusk_blindbid::Score;
use dusk_bls12_381::BlsScalar;
use dusk_bytes::DeserializableSlice;
use dusk_plonk::jubjub::JubJubAffine;
use dusk_plonk::prelude::*;
use tonic::{Request, Response, Status};

/// Implementation of the VerifyScore Handler.
pub struct VerifyScoreHandler<'a> {
    request: &'a Request<VerifyScoreRequest>,
}

impl<'a, 'b>
    ServiceRequestHandler<'a, 'b, VerifyScoreRequest, VerifyScoreResponse>
    for VerifyScoreHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<VerifyScoreRequest>) -> Self {
        Self { request }
    }

    #[allow(unreachable_code)]
    #[allow(unused_variables)]
    fn handle_request(&self) -> Result<Response<VerifyScoreResponse>, Status> {
        // Get the optional parameters from the request.
        let (proof, score, seed, prover_id) =
            parse_score_verify_params(self.request)?;
        // Get the non-optional parameters from the request.
        let latest_consensus_round =
            BlsScalar::from(self.request.get_ref().round as u64);
        let latest_consensus_step =
            BlsScalar::from(self.request.get_ref().step as u64);
        // Get bid from storage

        // FIXME: Once Bid contract is done and this
        // functionallity provided via spurious-functions.
        let (bid, branch) = unimplemented!();

        // Create a BlindBidCircuit instance
        let mut circuit = BlindBidCircuit {
            bid,
            score: Score::default(),
            secret_k: BlsScalar::default(),
            secret: JubJubAffine::default(),
            seed,
            latest_consensus_round,
            latest_consensus_step,
            branch: &branch,
        };

        Ok(Response::new(VerifyScoreResponse {
            success: verify_blindbid_proof(
                &mut circuit,
                &proof,
                prover_id,
                score,
            )
            .is_ok(),
        }))
    }
}

// Parses the optional inputs of the VerifyScoreRequest returning an error if
// any of them isn't present (is `None`).
fn parse_score_verify_params(
    request: &Request<VerifyScoreRequest>,
) -> Result<(Proof, BlsScalar, BlsScalar, BlsScalar), Status> {
    let proof = Proof::from_slice(&request.get_ref().proof)
        .map_err(|e| Status::failed_precondition(format!("{:?}", e)))?;
    let score = encoding::as_status_err(BlsScalar::from_slice(
        &request.get_ref().score,
    ))?;
    let seed = encoding::as_status_err(BlsScalar::from_slice(
        &request.get_ref().seed,
    ))?;
    let prover_id = encoding::as_status_err(BlsScalar::from_slice(
        &request.get_ref().prover_id,
    ))?;
    Ok((proof, score, seed, prover_id))
}

/// Given a circuit instance loaded with the dummy inputs and a
/// blindbid proof, verify whether the proof is correct or not.
fn verify_blindbid_proof(
    circuit: &mut BlindBidCircuit,
    proof: &Proof,
    prover_id: BlsScalar,
    score: BlsScalar,
) -> Result<()> {
    // Read ProverKey of the circuit.
    let vd =
        rusk_profile::keys_for(&BlindBidCircuit::CIRCUIT_ID)?.get_verifier()?;

    let vd = VerifierData::from_slice(&vd)?;

    let pi: Vec<PublicInputValue> = vec![
        (*circuit.branch.root()).into(),
        circuit.bid.hash().into(),
        (*circuit.bid.commitment()).into(),
        (*circuit.bid.hashed_secret()).into(),
        prover_id.into(),
        score.into(),
    ];

    // Verify the proof.
    circuit::verify_proof(
        &crate::PUB_PARAMS,
        &vd.key(),
        proof,
        &pi,
        &vd.pi_pos(),
        super::BLINDBID_TRANSCRIPT_INIT,
    )
    .map_err(|e| anyhow::anyhow!("{:?}", e))
}
