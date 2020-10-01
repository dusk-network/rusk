// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.
use anyhow::Result;
use bid_circuits::CorrectnessCircuit;
use dusk_blindbid::{bid::Bid, BlindBidCircuit};
use dusk_pki::{PublicSpendKey, SecretSpendKey};
use dusk_plonk::jubjub::{
    AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;
use kelvin::Blake2b;
use poseidon252::PoseidonTree;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

/// CRS path.
const PUB_PARAMS_FILE: &'static str = "pub_params_dev.bin";
/// BlindBid Circuit ProverKey path.
const BLINDBID_CIRCUIT_PK_PATH: &'static str = "blindbid_circ.pk";
/// BlindBid Circuit VerifierKey path.
const BLINDBID_CIRCUIT_VK_PATH: &'static str = "blindbid_circ.vk";
/// Bid correctness Circuit ProverKey path.
const BID_CORRECTNESS_CIRCUIT_PK_PATH: &'static str = "bid_correctness_circ.pk";
/// Bid correctness Circuit VerifierKey path.
const BID_CORRECTNESS_CIRCUIT_VK_PATH: &'static str = "bid_correctness_circ.vk";

/// Buildfile for the rusk crate.
///
/// Main goals of the file at the moment are:
/// 1. Compile the `.proto` files for tonic.
/// 2. Get the version of the crate and some extra info to
/// support the `-v` argument properly.
/// 3. Compile the blindbid circuit.
/// 4. Compile the Bid correctness circuit.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile protos for tonic.
    tonic_build::compile_protos("schema/rusk.proto")?;
    // Get crate version + commit + toolchain for `-v` arg support.
    println!(
        "cargo:rustc-env=GIT_HASH={}",
        rustc_tools_util::get_commit_hash().unwrap_or_default()
    );
    println!(
        "cargo:rustc-env=COMMIT_DATE={}",
        rustc_tools_util::get_commit_date().unwrap_or_default()
    );
    println!(
        "cargo:rustc-env=RUSTC_RELEASE_CHANNEL={}",
        rustc_tools_util::get_channel().unwrap_or_default()
    );

    // Compile BlindBid Circuit if it hasn't been already.
    match (
        PathBuf::from(BLINDBID_CIRCUIT_PK_PATH).exists(),
        PathBuf::from(BLINDBID_CIRCUIT_VK_PATH).exists(),
    ) {
        (true, true) => (),
        (_, _) => blindbid::compile_blindbid_circuit()?,
    };

    // Compile Bid correctness Circuit if it hasn't been already.
    match (
        PathBuf::from(BID_CORRECTNESS_CIRCUIT_PK_PATH).exists(),
        PathBuf::from(BID_CORRECTNESS_CIRCUIT_VK_PATH).exists(),
    ) {
        (true, true) => (),
        (_, _) => bid::compile_bid_correctness_circuit()?,
    };
    Ok(())
}

/// Read PublicParameters from the binary file they're stored on.
fn read_pub_params() -> Result<PublicParameters> {
    let mut pub_params_file = File::open(PUB_PARAMS_FILE)?;
    let mut buff = vec![];
    pub_params_file.read_to_end(&mut buff)?;
    let result: PublicParameters = bincode::deserialize(&buff)?;
    Ok(result)
}

mod bid {
    use super::*;

    pub fn compile_bid_correctness_circuit() -> Result<()> {
        let pub_params = read_pub_params()?;
        let value = JubJubScalar::from(100000 as u64);
        let blinder = JubJubScalar::from(50000 as u64);

        let c = AffinePoint::from(
            (GENERATOR_EXTENDED * value) + (GENERATOR_NUMS_EXTENDED * blinder),
        );

        let mut circuit = CorrectnessCircuit {
            commitment: Some(c),
            value: Some(value.into()),
            blinder: Some(blinder.into()),
            size: 0,
            pi_constructor: None,
        };

        let (pk, vk, _) = circuit.compile(&pub_params)?;
        let mut pk_file = File::create(BID_CORRECTNESS_CIRCUIT_PK_PATH)?;
        pk_file.write(&pk.to_bytes())?;

        let mut vk_file = File::create(BID_CORRECTNESS_CIRCUIT_VK_PATH)?;
        vk_file.write(&vk.to_bytes())?;
        Ok(())
    }
}

mod blindbid {
    use super::*;

    pub fn compile_blindbid_circuit() -> Result<()> {
        let pub_params = read_pub_params()?;
        // Generate a PoseidonTree and append the Bid.
        let mut tree: PoseidonTree<Bid, Blake2b> = PoseidonTree::new(17usize);
        // Generate a correct Bid
        let secret = JubJubScalar::random(&mut rand::thread_rng());
        let secret_k = BlsScalar::random(&mut rand::thread_rng());
        let bid = random_bid(&secret, secret_k)?;
        let secret: AffinePoint = (GENERATOR_EXTENDED * &secret).into();
        // Generate fields for the Bid & required by the compute_score
        let consensus_round_seed = BlsScalar::from(50u64);
        let latest_consensus_round = BlsScalar::from(50u64);
        let latest_consensus_step = BlsScalar::from(50u64);

        // Append the StorageBid as an StorageScalar to the tree.
        tree.push(bid)?;

        // Extract the branch
        let branch = tree
            .poseidon_branch(0u64)?
            .expect("Poseidon Branch Extraction");

        // Generate a `Score` for our Bid with the consensus parameters
        let score = bid.compute_score(
            &secret,
            secret_k,
            branch.root,
            consensus_round_seed,
            latest_consensus_round,
            latest_consensus_step,
        )?;

        let mut circuit = BlindBidCircuit {
            bid: Some(bid),
            score: Some(score),
            secret_k: Some(secret_k),
            secret: Some(secret),
            seed: Some(consensus_round_seed),
            latest_consensus_round: Some(latest_consensus_round),
            latest_consensus_step: Some(latest_consensus_step),
            branch: Some(&branch),
            size: 0,
            pi_constructor: None,
        };
        let (pk, vk, _) = circuit.compile(&pub_params)?;
        let mut pk_file = File::create(BLINDBID_CIRCUIT_PK_PATH)?;
        pk_file.write(&pk.to_bytes())?;

        let mut vk_file = File::create(BLINDBID_CIRCUIT_VK_PATH)?;
        vk_file.write(&vk.to_bytes())?;

        Ok(())
    }

    fn random_bid(secret: &JubJubScalar, secret_k: BlsScalar) -> Result<Bid> {
        let mut rng = rand::thread_rng();
        let pk_r = PublicSpendKey::from(SecretSpendKey::default());
        let stealth_addr = pk_r.gen_stealth_address(&secret);
        let secret = GENERATOR_EXTENDED * secret;
        let value = 60_000u64;
        let value = JubJubScalar::from(value);
        // Set the timestamps as the max values so the proofs do not fail for them
        // (never expired or non-elegible).
        let elegibility_ts = -BlsScalar::from(90u64);
        let expiration_ts = -BlsScalar::from(90u64);

        Bid::new(
            &mut rng,
            &stealth_addr,
            &value,
            &secret.into(),
            secret_k,
            elegibility_ts,
            expiration_ts,
        )
    }
}
