// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod tree_assets;
use blindbid_circuits::{BlindBidCircuit, BlindBidCircuitError};
use dusk_blindbid::{Bid, Score, V_RAW_MAX, V_RAW_MIN};
use dusk_pki::{PublicSpendKey, SecretSpendKey};
use dusk_plonk::jubjub::{JubJubAffine, GENERATOR_EXTENDED};
use dusk_plonk::prelude::*;
use rand::Rng;
use tree_assets::BidTree;

fn random_bid(secret: &JubJubScalar, secret_k: BlsScalar) -> Bid {
    let mut rng = rand::thread_rng();
    let pk_r = PublicSpendKey::from(SecretSpendKey::new(
        JubJubScalar::one(),
        -JubJubScalar::one(),
    ));
    let stealth_addr = pk_r.gen_stealth_address(&secret);
    let secret = GENERATOR_EXTENDED * secret;
    let value: u64 = (&mut rand::thread_rng()).gen_range(V_RAW_MIN..V_RAW_MAX);
    let value = JubJubScalar::from(value);
    // Set the timestamps as the max values so the proofs do not fail for them
    // (never expired or non-elegible).
    let elegibility_ts = u64::MAX;
    let expiration_ts = u64::MAX;

    Bid::new(
        &mut rng,
        &stealth_addr,
        &value,
        &secret.into(),
        secret_k,
        elegibility_ts,
        expiration_ts,
    )
    .expect("Bid creation error")
}

#[test]
fn correct_blindbid_proof() -> Result<(), BlindBidCircuitError> {
    // Generate Composer & Public Parameters
    let pub_params = unsafe {
        PublicParameters::from_slice_unchecked(
            rusk_profile::get_common_reference_string()
                .expect("Failed to fetch CRS from rusk_profile")
                .as_slice(),
        )
    };

    // Generate a BidTree and append the Bid.
    let mut tree = BidTree::new();

    // Generate a correct Bid
    let secret = JubJubScalar::random(&mut rand::thread_rng());
    let secret_k = BlsScalar::random(&mut rand::thread_rng());
    let bid = random_bid(&secret, secret_k);
    let secret: JubJubAffine = (GENERATOR_EXTENDED * &secret).into();
    // Generate fields for the Bid & required by the compute_score
    let consensus_round_seed = BlsScalar::random(&mut rand::thread_rng());
    let latest_consensus_round = 50u64;
    let latest_consensus_step = 50u64;

    // Append the Bid to the tree.
    tree.push(bid.into())?;

    // Extract the branch
    let branch = tree.branch(0)?.expect("Poseidon Branch Extraction");

    // Generate a `Score` for our Bid with the consensus parameters
    let score = Score::compute(
        &bid,
        &secret,
        secret_k,
        *branch.root(),
        consensus_round_seed,
        latest_consensus_round,
        latest_consensus_step,
    )?;

    let prover_id = bid.generate_prover_id(
        secret_k,
        BlsScalar::from(consensus_round_seed),
        BlsScalar::from(latest_consensus_round),
        BlsScalar::from(latest_consensus_step),
    );

    let mut circuit = BlindBidCircuit {
        bid,
        score,
        secret_k,
        secret,
        seed: BlsScalar::from(consensus_round_seed),
        latest_consensus_round: BlsScalar::from(latest_consensus_round),
        latest_consensus_step: BlsScalar::from(latest_consensus_step),
        branch: &branch,
    };

    let (pk, vd) = circuit
        .compile(&pub_params)
        .expect("Circuit compilation Error");
    let proof = circuit.gen_proof(&pub_params, &pk, b"CorrectBid")?;
    let storage_bid = bid.hash();
    let pi: Vec<PublicInputValue> = vec![
        (*branch.root()).into(),
        storage_bid.into(),
        (*bid.commitment()).into(),
        (*bid.hashed_secret()).into(),
        prover_id.into(),
        (*score.value()).into(),
    ];

    Ok(circuit::verify_proof(
        &pub_params,
        &vd.key(),
        &proof,
        &pi,
        &vd.pi_pos(),
        b"CorrectBid",
    )?)
}

#[test]
fn edited_score_blindbid_proof() -> Result<(), BlindBidCircuitError> {
    // Generate Composer & Public Parameters
    let pub_params = unsafe {
        PublicParameters::from_slice_unchecked(
            rusk_profile::get_common_reference_string()
                .expect("Failed to fetch CRS from rusk_profile")
                .as_slice(),
        )
    };

    // Generate a BidTree and append the Bid.
    let mut tree = BidTree::new();

    // Generate a correct Bid
    let secret = JubJubScalar::random(&mut rand::thread_rng());
    let secret_k = BlsScalar::random(&mut rand::thread_rng());
    let bid = random_bid(&secret, secret_k);
    let secret: JubJubAffine = (GENERATOR_EXTENDED * &secret).into();
    // Generate fields for the Bid & required by the compute_score
    let consensus_round_seed = BlsScalar::random(&mut rand::thread_rng());
    let latest_consensus_round = 50u64;
    let latest_consensus_step = 50u64;

    // Append the Bid to the tree.
    tree.push(bid.into())?;

    // Extract the branch
    let branch = tree.branch(0)?.expect("Poseidon Branch Extraction");

    // Generate a `Score` for our Bid with the consensus parameters
    let mut score = Score::compute(
        &bid,
        &secret,
        secret_k,
        *branch.root(),
        consensus_round_seed,
        latest_consensus_round,
        latest_consensus_step,
    )?;

    // The only way to edit an Score field is to unsafely do so.
    score = unsafe {
        std::mem::transmute_copy(&[
            // We add 100 to the original Score value trying to cheat on the bid.
            score.value() + BlsScalar::from(100u64),
            *score.y(),
            *score.y_prime(),
            *score.r1(),
            *score.r2(),
        ])
    };

    let prover_id = bid.generate_prover_id(
        secret_k,
        BlsScalar::from(consensus_round_seed),
        BlsScalar::from(latest_consensus_round),
        BlsScalar::from(latest_consensus_step),
    );

    let mut circuit = BlindBidCircuit {
        bid,
        score,
        secret_k,
        secret,
        seed: BlsScalar::from(consensus_round_seed),
        latest_consensus_round: BlsScalar::from(latest_consensus_round),
        latest_consensus_step: BlsScalar::from(latest_consensus_step),
        branch: &branch,
    };

    let (pk, vd) = circuit
        .compile(&pub_params)
        .expect("Circuit compilation Error");
    let proof = circuit.gen_proof(&pub_params, &pk, b"BidWithEditedScore")?;
    let storage_bid = bid.hash();
    let pi: Vec<PublicInputValue> = vec![
        (*branch.root()).into(),
        storage_bid.into(),
        (*bid.commitment()).into(),
        (*bid.hashed_secret()).into(),
        prover_id.into(),
        (*score.value()).into(),
    ];
    assert!(circuit::verify_proof(
        &pub_params,
        &vd.key(),
        &proof,
        &pi,
        &vd.pi_pos(),
        b"BidWithEditedScore"
    )
    .is_err());
    Ok(())
}

#[test]
fn expired_bid_proof() -> Result<(), BlindBidCircuitError> {
    // Generate Composer & Public Parameters
    let pub_params = unsafe {
        PublicParameters::from_slice_unchecked(
            rusk_profile::get_common_reference_string()
                .expect("Failed to fetch CRS from rusk_profile")
                .as_slice(),
        )
    };

    // Generate a BidTree and append the Bid.
    let mut tree = BidTree::new();

    // Create an expired bid.
    let mut rng = rand::thread_rng();
    let secret = JubJubScalar::random(&mut rng);
    let pk_r = PublicSpendKey::from(SecretSpendKey::random(&mut rng));
    let stealth_addr = pk_r.gen_stealth_address(&secret);
    let secret = JubJubAffine::from(GENERATOR_EXTENDED * secret);
    let secret_k = BlsScalar::random(&mut rng);
    let value: u64 = (&mut rand::thread_rng()).gen_range(V_RAW_MIN..V_RAW_MAX);
    let value = JubJubScalar::from(value);
    let expiration_ts = 100u64;
    let elegibility_ts = 1000u64;
    let bid = Bid::new(
        &mut rng,
        &stealth_addr,
        &value,
        &secret.into(),
        secret_k,
        elegibility_ts,
        expiration_ts,
    )
    .expect("Bid creation error");

    // Append the Bid to the tree.
    tree.push(bid.into())?;

    // Extract the branch
    let branch = tree.branch(0)?.expect("Poseidon Branch Extraction");

    // We first generate the score as if the bid wasn't expired. Otherways
    // the score generation would fail since the Bid would be expired.
    let latest_consensus_round = 3u64;
    let latest_consensus_step = 1u64;
    let consensus_round_seed = BlsScalar::random(&mut rand::thread_rng());

    // Generate a `Score` for our Bid with the consensus parameters
    let score = Score::compute(
        &bid,
        &secret,
        secret_k,
        *branch.root(),
        consensus_round_seed,
        latest_consensus_round,
        latest_consensus_step,
    )?;

    // Latest consensus step should be lower than the expiration_ts, in this
    // case is not so the proof should fail since the Bid is expired
    // at this round.
    let latest_consensus_round = 200u64;

    let prover_id = bid.generate_prover_id(
        secret_k,
        BlsScalar::from(consensus_round_seed),
        BlsScalar::from(latest_consensus_round),
        BlsScalar::from(latest_consensus_step),
    );

    let mut circuit = BlindBidCircuit {
        bid,
        score,
        secret_k,
        secret,
        seed: BlsScalar::from(consensus_round_seed),
        latest_consensus_round: BlsScalar::from(latest_consensus_round),
        latest_consensus_step: BlsScalar::from(latest_consensus_step),
        branch: &branch,
    };

    let (pk, vd) = circuit
        .compile(&pub_params)
        .expect("Circuit compilation Error");
    let proof = circuit.gen_proof(&pub_params, &pk, b"ExpiredBid")?;
    let storage_bid = bid.hash();
    let pi: Vec<PublicInputValue> = vec![
        (*branch.root()).into(),
        storage_bid.into(),
        (*bid.commitment()).into(),
        (*bid.hashed_secret()).into(),
        prover_id.into(),
        (*score.value()).into(),
    ];
    assert!(circuit::verify_proof(
        &pub_params,
        &vd.key(),
        &proof,
        &pi,
        &vd.pi_pos(),
        b"ExpiredBid"
    )
    .is_err());
    Ok(())
}

#[test]
fn non_elegible_bid() -> Result<(), BlindBidCircuitError> {
    // Generate Composer & Public Parameters
    let pub_params = unsafe {
        PublicParameters::from_slice_unchecked(
            rusk_profile::get_common_reference_string()
                .expect("Failed to fetch CRS from rusk_profile")
                .as_slice(),
        )
    };

    // Generate a BidTree and append the Bid.
    let mut tree = BidTree::new();

    // Create a non-elegible Bid.
    let mut rng = rand::thread_rng();
    let secret = JubJubScalar::random(&mut rng);
    let pk_r = PublicSpendKey::from(SecretSpendKey::random(&mut rng));
    let stealth_addr = pk_r.gen_stealth_address(&secret);
    let secret = JubJubAffine::from(GENERATOR_EXTENDED * secret);
    let secret_k = BlsScalar::random(&mut rng);
    let value: u64 = (&mut rand::thread_rng()).gen_range(V_RAW_MIN..V_RAW_MAX);
    let value = JubJubScalar::from(value);
    let expiration_ts = 100u64;
    let elegibility_ts = 1000u64;
    let bid = Bid::new(
        &mut rng,
        &stealth_addr,
        &value,
        &secret.into(),
        secret_k,
        elegibility_ts,
        expiration_ts,
    )
    .expect("Bid creation error");

    // Append the Bid to the tree.
    tree.push(bid.into())?;

    // Extract the branch
    let branch = tree.branch(0)?.expect("Poseidon Branch Extraction");

    // We first generate the score as if the bid was still eligible.
    // Otherways the score generation would fail since the Bid
    // wouldn't be elegible.
    let latest_consensus_round = 3u64;
    let latest_consensus_step = 1u64;
    let consensus_round_seed = BlsScalar::random(&mut rand::thread_rng());

    // Generate a `Score` for our Bid with the consensus parameters
    let score = Score::compute(
        &bid,
        &secret,
        secret_k,
        *branch.root(),
        consensus_round_seed,
        latest_consensus_round,
        latest_consensus_step,
    )?;

    let prover_id = bid.generate_prover_id(
        secret_k,
        BlsScalar::from(consensus_round_seed),
        BlsScalar::from(latest_consensus_round),
        BlsScalar::from(latest_consensus_step),
    );

    // Latest consensus step should be lower than the elegibility_ts, in
    // this case is not so the proof should fail since the Bid is
    // non elegible anymore.
    let latest_consensus_round = 200u64;

    let mut circuit = BlindBidCircuit {
        bid,
        score,
        secret_k,
        secret,
        seed: BlsScalar::from(consensus_round_seed),
        latest_consensus_round: BlsScalar::from(latest_consensus_round),
        latest_consensus_step: BlsScalar::from(latest_consensus_step),
        branch: &branch,
    };

    let (pk, vd) = circuit
        .compile(&pub_params)
        .expect("Circuit compilation Error");
    let proof = circuit.gen_proof(&pub_params, &pk, b"NonElegibleBid")?;
    let storage_bid = bid.hash();
    let pi: Vec<PublicInputValue> = vec![
        (*branch.root()).into(),
        storage_bid.into(),
        (*bid.commitment()).into(),
        (*bid.hashed_secret()).into(),
        prover_id.into(),
        (*score.value()).into(),
    ];
    assert!(circuit::verify_proof(
        &pub_params,
        &vd.key(),
        &proof,
        &pi,
        &vd.pi_pos(),
        b"NonElegibleBid"
    )
    .is_err());
    Ok(())
}
