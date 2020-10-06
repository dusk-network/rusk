// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use anyhow::Result;
use bid_circuits::CorrectnessCircuit;
use dusk_blindbid::{bid::Bid, BlindBidCircuit};
use dusk_pki::{PublicSpendKey, SecretSpendKey};
use dusk_plonk::circuit_builder::Circuit;
use dusk_plonk::jubjub::{
    AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::PublicParameters;
use dusk_plonk::prelude::*;
use kelvin::Blake2b;
use lazy_static::lazy_static;
use poseidon252::{PoseidonAnnotation, PoseidonTree};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

/// Default proof path.
const DEFAULT_PROOF_FILE: &str = "src/lib/proof.bin";

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

    // Write a default proof if it doesn't exist yet.
    match PathBuf::from(DEFAULT_PROOF_FILE).exists() {
        true => (),
        _ => default_proof::write_default_proof()?,
    };

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
            commitment: c,
            value: value.into(),
            blinder: blinder.into(),
            trim_size: 1 << 10,
            pi_positions: vec![],
        };

        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }
}

mod blindbid {
    use super::*;

    pub fn compile_circuit() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;
        // Generate a PoseidonTree and append the Bid.
        let mut tree: PoseidonTree<Bid, PoseidonAnnotation, Blake2b> =
            PoseidonTree::new(17usize);
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
            branch.root(),
            consensus_round_seed,
            latest_consensus_round,
            latest_consensus_step,
        )?;

        let mut circuit = BlindBidCircuit {
            bid,
            score,
            secret_k,
            secret,
            seed: consensus_round_seed,
            latest_consensus_round,
            latest_consensus_step,
            branch: &branch,
            trim_size: 1 << 15,
            pi_positions: vec![],
        };
        let (pk, vk) = circuit.compile(&pub_params)?;
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

mod default_proof {
    use super::*;

    const DEFAULT_PROOF_BYTES: [u8; 1040] = [
        170, 161, 69, 162, 186, 114, 128, 191, 233, 75, 200, 123, 129, 208,
        217, 183, 186, 165, 191, 134, 80, 225, 163, 225, 93, 117, 79, 138, 235,
        159, 98, 157, 251, 55, 186, 143, 5, 73, 207, 252, 4, 138, 55, 48, 86,
        43, 79, 106, 164, 33, 201, 127, 177, 218, 94, 184, 168, 63, 232, 149,
        175, 37, 92, 103, 62, 76, 118, 188, 221, 62, 249, 207, 67, 202, 34, 1,
        57, 211, 13, 238, 184, 93, 229, 80, 81, 177, 217, 204, 34, 24, 103, 54,
        173, 92, 15, 167, 169, 166, 8, 92, 28, 129, 97, 0, 217, 87, 30, 74,
        111, 60, 32, 61, 49, 80, 196, 17, 0, 187, 114, 142, 224, 133, 139, 169,
        23, 108, 3, 34, 17, 16, 117, 252, 143, 81, 123, 57, 205, 109, 153, 1,
        124, 218, 139, 122, 166, 86, 122, 244, 102, 8, 15, 144, 223, 74, 173,
        203, 70, 105, 209, 37, 228, 55, 197, 75, 78, 10, 93, 57, 30, 231, 97,
        101, 19, 22, 89, 159, 69, 169, 69, 44, 97, 119, 157, 221, 172, 85, 209,
        159, 232, 47, 25, 79, 175, 254, 179, 124, 106, 216, 89, 186, 20, 114,
        254, 246, 165, 244, 195, 200, 104, 34, 92, 109, 129, 240, 106, 21, 166,
        146, 75, 40, 194, 138, 99, 144, 5, 246, 90, 179, 174, 109, 37, 223,
        134, 196, 221, 66, 155, 206, 64, 23, 141, 1, 22, 238, 184, 73, 125, 98,
        59, 220, 65, 243, 173, 233, 73, 158, 64, 35, 124, 103, 254, 251, 239,
        39, 219, 54, 14, 248, 54, 219, 78, 58, 244, 204, 250, 112, 233, 124,
        34, 181, 78, 226, 13, 97, 136, 14, 151, 254, 128, 253, 168, 11, 142,
        183, 46, 198, 242, 93, 222, 135, 226, 66, 33, 126, 97, 46, 121, 114,
        202, 254, 108, 231, 253, 115, 209, 103, 237, 237, 195, 123, 51, 115,
        230, 118, 23, 95, 146, 190, 138, 203, 173, 32, 59, 35, 185, 208, 139,
        166, 90, 110, 194, 149, 125, 62, 200, 179, 168, 136, 148, 78, 233, 68,
        91, 131, 211, 80, 127, 123, 225, 19, 158, 136, 216, 93, 253, 13, 118,
        52, 225, 71, 127, 240, 179, 231, 248, 215, 21, 224, 28, 113, 65, 196,
        35, 210, 166, 39, 207, 112, 211, 208, 203, 223, 195, 34, 216, 3, 88,
        80, 64, 148, 119, 175, 243, 56, 27, 76, 68, 121, 224, 22, 252, 152,
        220, 64, 38, 72, 24, 216, 239, 74, 81, 101, 32, 243, 53, 162, 176, 191,
        73, 197, 8, 73, 175, 119, 10, 154, 125, 49, 208, 180, 215, 208, 193,
        12, 163, 151, 155, 95, 213, 42, 239, 48, 40, 170, 61, 253, 93, 32, 30,
        63, 50, 29, 120, 197, 160, 121, 80, 65, 228, 23, 248, 227, 82, 187,
        114, 224, 26, 140, 110, 168, 141, 168, 27, 183, 166, 131, 145, 208,
        161, 193, 9, 27, 32, 23, 202, 157, 31, 14, 128, 81, 76, 203, 160, 28,
        169, 138, 90, 45, 75, 196, 56, 31, 157, 11, 217, 136, 71, 88, 102, 226,
        4, 88, 7, 246, 147, 101, 189, 150, 86, 206, 49, 176, 118, 233, 0, 234,
        177, 19, 0, 202, 65, 78, 154, 230, 17, 4, 64, 2, 161, 251, 178, 0, 53,
        7, 87, 243, 162, 156, 171, 65, 11, 231, 136, 131, 150, 251, 136, 143,
        203, 30, 57, 50, 248, 38, 212, 91, 11, 83, 216, 210, 91, 237, 43, 95,
        101, 75, 120, 107, 228, 183, 71, 107, 7, 23, 112, 201, 157, 238, 219,
        214, 237, 160, 141, 214, 38, 215, 70, 123, 135, 167, 56, 5, 44, 87,
        110, 83, 138, 91, 43, 74, 82, 38, 7, 32, 166, 71, 35, 57, 161, 124,
        223, 14, 192, 67, 37, 155, 228, 54, 212, 108, 68, 4, 86, 217, 95, 202,
        47, 249, 238, 2, 170, 148, 52, 169, 196, 9, 37, 137, 3, 203, 206, 28,
        44, 191, 191, 226, 189, 225, 172, 174, 183, 74, 148, 121, 253, 153, 2,
        43, 168, 214, 196, 75, 74, 137, 34, 249, 121, 113, 135, 121, 80, 35,
        69, 61, 235, 28, 7, 201, 94, 174, 60, 248, 198, 46, 83, 10, 44, 92, 23,
        107, 214, 146, 164, 133, 241, 37, 25, 238, 61, 85, 73, 14, 85, 228,
        218, 102, 142, 183, 97, 109, 201, 23, 160, 244, 228, 81, 68, 120, 107,
        17, 178, 77, 251, 82, 103, 224, 220, 222, 234, 40, 100, 59, 113, 133,
        53, 25, 171, 88, 195, 207, 117, 13, 136, 125, 137, 15, 46, 38, 12, 122,
        227, 93, 218, 117, 98, 142, 219, 188, 248, 201, 32, 129, 91, 103, 56,
        75, 56, 207, 85, 118, 156, 116, 4, 216, 230, 151, 14, 156, 81, 119,
        110, 142, 155, 232, 115, 207, 182, 34, 118, 228, 175, 83, 6, 110, 69,
        58, 51, 191, 135, 26, 123, 17, 87, 67, 246, 72, 225, 217, 183, 100, 49,
        124, 9, 117, 18, 101, 143, 163, 125, 134, 5, 61, 206, 43, 104, 234, 22,
        113, 142, 46, 78, 137, 96, 106, 44, 170, 91, 40, 188, 246, 141, 208,
        235, 147, 105, 98, 164, 245, 245, 169, 138, 97, 157, 255, 193, 63, 221,
        165, 133, 88, 29, 217, 28, 78, 64, 3, 45, 241, 55, 156, 136, 224, 67,
        218, 143, 80, 113, 96, 43, 229, 166, 174, 185, 160, 34, 179, 162, 36,
        212, 44, 4, 46, 141, 246, 184, 73, 222, 33, 246, 228, 166, 1, 109, 119,
        20, 41, 176, 57, 169, 141, 10, 149, 25, 26, 244, 207, 176, 54, 223, 98,
        23, 79, 73, 35, 142, 225, 78, 94, 155, 175, 164, 146, 63, 55, 180, 173,
        1, 66, 0, 112, 203, 1, 157, 177, 134, 228, 164, 212, 254, 63, 188, 199,
        25, 172, 248, 13, 170, 6, 212, 158, 244, 18, 24, 93, 214, 54, 148, 71,
        127, 121, 85, 144, 208, 41, 192, 1, 143, 17, 67, 149, 130, 157, 177,
        252, 121, 30, 51, 48, 218, 218, 46, 126, 11, 247, 123, 50, 154, 188,
        101, 71, 93, 82, 52, 38, 198, 57, 123, 2, 252, 179, 132, 24, 45, 203,
        103, 189, 16, 249, 9, 48,
    ];

    pub fn write_default_proof() -> Result<()> {
        let proof = Proof::from_bytes(&DEFAULT_PROOF_BYTES)?;

        let mut proof_file = File::create(DEFAULT_PROOF_FILE)?;
        let _ = proof_file.write(&proof.to_bytes())?;
        Ok(())
    }
}
