// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use anyhow::Result;
use bid_circuits::CorrectnessCircuit;
use dusk_blindbid::{bid::Bid, tree::BidTree, BlindBidCircuit};
use dusk_pki::{PublicSpendKey, SecretSpendKey};
use dusk_plonk::jubjub::{
    AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;

use dusk_plonk::prelude::PublicParameters;
use lazy_static::lazy_static;

lazy_static! {
    static ref PUB_PARAMS: PublicParameters = {
        let buff = match rusk_profile::get_common_reference_string() {
            Ok(buff) => buff,
            Err(_) => {
                rusk_profile::set_common_reference_string("pub_params_dev.bin")
                    .expect("Unable to copy the CRS")
            }
        };

        let result: PublicParameters =
            bincode::deserialize(&buff).expect("CRS not decoded");

        result
    };
}

/// Buildfile for the rusk crate.
///
/// Main goals of the file at the moment are:
/// 1. Compile the `.proto` files for tonic.
/// 2. Get the version of the crate and some extra info to
/// support the `-v` argument properly.
/// 3. Compile the blindbid circuit.
/// 4. Compile the Bid correctness circuit.
fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    // Compile protos for tonic
    tonic_build::compile_protos("schema/rusk.proto")?;

    // Get the cached keys for bid-circuits crate from rusk profile, or
    // recompile and update them if they're outdated
    let bid_keys = rusk_profile::keys_for("bid-circuits");
    if bid_keys.are_outdated() {
        bid_keys.update("bid", bid::compile_circuit()?)?;
    }

    // Get the cached keys for dusk-blindbid crate from rusk profile, or
    // recompile and update them if they're outdated
    let blindbid_keys = rusk_profile::keys_for("dusk-blindbid");
    if blindbid_keys.are_outdated() {
        blindbid_keys.update("blindbid", blindbid::compile_circuit()?)?;
    }

    Ok(())
}

mod bid {
    use super::*;

    pub fn compile_circuit() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;
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
        Ok((pk.to_bytes(), vk.to_bytes()))
    }
}

mod blindbid {
    use super::*;

    pub fn compile_circuit() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;
        // Generate a PoseidonTree and append the Bid.
        let mut tree = BidTree::new(17usize);
        // Generate a correct Bid
        let secret = JubJubScalar::random(&mut rand::thread_rng());
        let secret_k = BlsScalar::random(&mut rand::thread_rng());
        let bid = random_bid(&secret, secret_k)?;
        let secret: AffinePoint = (GENERATOR_EXTENDED * secret).into();

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
        Ok((pk.to_bytes(), vk.to_bytes()))
    }

    fn random_bid(secret: &JubJubScalar, secret_k: BlsScalar) -> Result<Bid> {
        let mut rng = rand::thread_rng();
        let pk_r = PublicSpendKey::from(SecretSpendKey::default());
        let stealth_addr = pk_r.gen_stealth_address(&secret);
        let secret = GENERATOR_EXTENDED * secret;
        let value = 60_000u64;
        let value = JubJubScalar::from(value);
        // Set the timestamps as the max values so the proofs do not fail for
        // them (never expired or non-elegible).
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
