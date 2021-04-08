// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![allow(non_snake_case)]

/*
use bid_circuits::CorrectnessCircuit;
use dusk_blindbid::{Bid, BlindBidCircuit, Score};
use dusk_bls12_381::BlsScalar;
use dusk_jubjub::{JubJubAffine, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED};
use dusk_pki::{PublicSpendKey, SecretSpendKey};
use dusk_poseidon::tree::PoseidonBranch;
*/
use lazy_static::lazy_static;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use dusk_plonk::prelude::*;

lazy_static! {
    static ref PUB_PARAMS: PublicParameters = {
        match rusk_profile::get_common_reference_string() {
            Ok(buff) if rusk_profile::verify_common_reference_string(&buff) => unsafe {
                info!("Got the CRS from cache");

                PublicParameters::from_slice_unchecked(&buff[..])
            },

            Ok(_) | Err(_) => {
                info!("New CRS needs to be generated and cached");

                use rand::rngs::StdRng;
                use rand::SeedableRng;

                let mut rng = StdRng::seed_from_u64(0xbeef);

                let pp = PublicParameters::setup(1 << 17, &mut rng)
                    .expect("Cannot initialize Public Parameters");

                info!("Public Parameters initialized");

                rusk_profile::set_common_reference_string(
                    pp.to_raw_var_bytes(),
                )
                .expect("Unable to write the CRS");

                pp
            }
        }
    };
}
/// Buildfile for the rusk crate.
///
/// Main goals of the file at the moment are:
/// 1. Compile the `.proto` files for tonic.
/// 2. Get the version of the crate and some extra info to
/// support the `-v` argument properly.
/// 3. Compile the contract-related circuits.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Ensure we run the build script again even if we change just the build.rs
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=path/to/Cargo.lock");

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

    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info,
        // warn, etc.) will be written to stdout.
        .with_max_level(Level::TRACE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    // This will enforce the usage and therefore the cache / generation
    // of the CRS even if it's not used to compiles circuits inside the
    // build script.
    lazy_static::initialize(&PUB_PARAMS);

    // Compile protos for tonic
    tonic_build::compile_protos("../schema/rusk.proto")?;

    /*
    if option_env!("RUSK_BUILD_BID_KEYS").unwrap_or("0") != "0" {
        info!("Bulding Bid Keys");
        let bid_keys = rusk_profile::keys_for("bid-circuits");
        bid_keys.clear_all()?;
        bid_keys.update("bid", bid::compile_circuit()?)?;

        let blindbid_keys = rusk_profile::keys_for("dusk-blindbid");
        blindbid_keys.clear_all()?;
        blindbid_keys.update("blindbid", blindbid::compile_circuit()?)?;
    }
    */

    if option_env!("RUSK_BUILD_TRANSFER_KEYS").unwrap_or("0") != "0" {
        info!("Building Transfer Keys");
        let transfer_keys = rusk_profile::keys_for("transfer-circuits");
        info!("Transfer keys prepared!");

        (0..3).into_par_iter().for_each(|i| match i {
            0 => {
               info!("Starting the compilation of the circuit \"transfer-send-to-contract-obfuscated\"");

                let (id, pk, vd) =
                    transfer::compile_stco_circuit().expect("Circuit Compiled");
                transfer_keys.update(id, (pk, vd)).expect("Keys Updated");
            }
            1 => {
                info!("Starting the compilation of the circuit \"transfer-send-to-contract-transparent\"");
                let (id, pk, vd) =
                    transfer::compile_stct_circuit().expect("Circuit Compiled");
                transfer_keys.update(id, (pk, vd)).expect("Keys Updated");
            }
            2 => {
                info!("Starting the compilation of the circuit \"withdraw-from-obfuscated\"");
                let (id, pk, vd) =
                    transfer::compile_wfo_circuit().expect("Circuit Compiled");
                transfer_keys.update(id, (pk, vd)).expect("Keys Updated")
            }
            _ => (),
        });

        use rayon::prelude::*;
        let args = (1..5)
            .flat_map(|i| (0..3).map(move |j| (i, j)))
            .collect::<Vec<(usize, usize)>>();

        args.par_iter().for_each(|(inputs, outputs)| {
            let (id, pk, vd) =
                transfer::compile_execute_circuit(*inputs, *outputs)
                    .expect("Circuit Compiled");

            transfer_keys.update(id, (pk, vd)).expect("Keys Updated");
        });
    }

    Ok(())
}

/*
mod bid {
    use super::*;

    pub fn compile_circuit(
    ) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
        let pub_params = &PUB_PARAMS;
        let value = JubJubScalar::from(100000 as u64);
        let blinder = JubJubScalar::from(50000 as u64);

        let c = JubJubAffine::from(
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

    pub fn compile_circuit(
    ) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
        let pub_params = &PUB_PARAMS;

        // Generate a correct Bid
        let secret = JubJubScalar::random(&mut rand::thread_rng());
        let secret_k = BlsScalar::random(&mut rand::thread_rng());
        let bid = random_bid(&secret, secret_k);
        let secret: JubJubAffine = (GENERATOR_EXTENDED * secret).into();

        // Generate fields for the Bid & required by the compute_score
        let consensus_round_seed = 50u64;
        let latest_consensus_round = 50u64;
        let latest_consensus_step = 50u64;

        // Extract the branch
        let branch = PoseidonBranch::<17>::default();

        // Generate a `Score` for our Bid with the consensus parameters
        let score = Score::compute(
            &bid,
            &secret,
            secret_k,
            *branch.root(),
            BlsScalar::from(consensus_round_seed),
            latest_consensus_round,
            latest_consensus_step,
        )
        .expect("Score gen error");

        let mut circuit = BlindBidCircuit {
            bid,
            score,
            secret_k,
            secret,
            seed: BlsScalar::from(consensus_round_seed),
            latest_consensus_round: BlsScalar::from(latest_consensus_round),
            latest_consensus_step: BlsScalar::from(latest_consensus_step),
            branch: &branch,
            trim_size: 1 << 15,
            pi_positions: vec![],
        };
        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }

    fn random_bid(secret: &JubJubScalar, secret_k: BlsScalar) -> Bid {
        let mut rng = rand::thread_rng();
        let pk_r = PublicSpendKey::from(SecretSpendKey::random(&mut rng));
        let stealth_addr = pk_r.gen_stealth_address(&secret);
        let secret = GENERATOR_EXTENDED * secret;
        let value = 60_000u64;
        let value = JubJubScalar::from(value);
        // Set the timestamps as the max values so the proofs do not fail for
        // them (never expired or non-elegible).
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
        .expect("Error generating a Bid")
    }
}
*/

mod transfer {
    use super::PUB_PARAMS;
    use std::convert::TryInto;

    use anyhow::{anyhow, Result};
    use canonical_host::MemStore;
    use dusk_bytes::Serializable;
    use dusk_pki::SecretSpendKey;
    use dusk_plonk::circuit;
    use dusk_plonk::circuit::VerifierData;
    use phoenix_core::{Message, Note};
    use sha2::{Digest, Sha256};
    use tracing::info;
    use transfer_circuits::{
        ExecuteCircuit, SendToContractObfuscatedCircuit,
        SendToContractTransparentCircuit, WithdrawFromObfuscatedCircuit,
    };

    use dusk_plonk::prelude::*;

    pub fn compile_stco_circuit() -> Result<(&'static str, Vec<u8>, Vec<u8>)> {
        let mut rng = rand::thread_rng();

        let ssk = SecretSpendKey::random(&mut rng);
        let vk = ssk.view_key();
        let psk = ssk.public_spend_key();

        let c_value = 100;
        let c_blinding_factor = JubJubScalar::random(&mut rng);
        let c_note =
            Note::obfuscated(&mut rng, &psk, c_value, c_blinding_factor);
        let (fee, crossover) = c_note.try_into().map_err(|e| {
            anyhow!("Failed to convert phoenix note into crossover: {:?}", e)
        })?;
        let c_signature = SendToContractObfuscatedCircuit::sign(
            &mut rng, &ssk, &fee, &crossover,
        );

        let message_r = JubJubScalar::random(&mut rng);
        let message_value = 100;
        let message = Message::new(&mut rng, &message_r, &psk, message_value);

        let mut circuit = SendToContractObfuscatedCircuit::new(
            &crossover,
            &fee,
            &vk,
            c_signature,
            &message,
            &psk,
            message_r,
        )
        .map_err(|e| anyhow!("Error generating circuit: {:?}", e))?;

        let (pk, vd) = circuit.compile(&PUB_PARAMS)?;

        let id = SendToContractObfuscatedCircuit::rusk_keys_id();
        let pk = pk.to_var_bytes();
        let vd = vd.to_var_bytes();

        Ok((id, pk, vd))
    }

    pub fn compile_stct_circuit() -> Result<(&'static str, Vec<u8>, Vec<u8>)> {
        let mut rng = rand::thread_rng();

        let c_ssk = SecretSpendKey::random(&mut rng);
        let c_vk = c_ssk.view_key();
        let c_psk = c_ssk.public_spend_key();

        let c_value = 100;
        let c_blinding_factor = JubJubScalar::random(&mut rng);

        let c_note =
            Note::obfuscated(&mut rng, &c_psk, c_value, c_blinding_factor);
        let (fee, crossover) = c_note.try_into().map_err(|e| {
            anyhow!("Failed to convert phoenix note into crossover: {:?}", e)
        })?;

        let c_signature = SendToContractTransparentCircuit::sign(
            &mut rng, &c_ssk, &fee, &crossover,
        );

        let mut circuit = SendToContractTransparentCircuit::new(
            &fee,
            &crossover,
            &c_vk,
            c_signature,
        )
        .map_err(|e| anyhow!("Error generating circuit: {:?}", e))?;

        let (pk, vd) = circuit.compile(&PUB_PARAMS)?;

        let id = SendToContractTransparentCircuit::rusk_keys_id();
        let pk = pk.to_var_bytes();
        let vd = vd.to_var_bytes();

        Ok((id, pk, vd))
    }

    pub fn compile_wfo_circuit() -> Result<(&'static str, Vec<u8>, Vec<u8>)> {
        let mut rng = rand::thread_rng();

        let i_ssk = SecretSpendKey::random(&mut rng);
        let i_vk = i_ssk.view_key();
        let i_psk = i_ssk.public_spend_key();
        let i_value = 100;
        let i_blinding_factor = JubJubScalar::random(&mut rng);
        let i_note =
            Note::obfuscated(&mut rng, &i_psk, i_value, i_blinding_factor);

        let c_ssk = SecretSpendKey::random(&mut rng);
        let c_psk = c_ssk.public_spend_key();
        let c_r = JubJubScalar::random(&mut rng);
        let c_value = 25;
        let c = Message::new(&mut rng, &c_r, &c_psk, c_value);

        let o_ssk = SecretSpendKey::random(&mut rng);
        let o_vk = o_ssk.view_key();
        let o_psk = o_ssk.public_spend_key();
        let o_value = 75;
        let o_blinding_factor = JubJubScalar::random(&mut rng);
        let o_note =
            Note::obfuscated(&mut rng, &o_psk, o_value, o_blinding_factor);

        let mut circuit = WithdrawFromObfuscatedCircuit::new(
            &i_note,
            Some(&i_vk),
            &c,
            c_r,
            &c_psk,
            &o_note,
            Some(&o_vk),
        )
        .map_err(|e| anyhow!("Error generating circuit: {:?}", e))?;

        let (pk, vd) = circuit.compile(&PUB_PARAMS)?;

        let id = WithdrawFromObfuscatedCircuit::rusk_keys_id();
        let pk = pk.to_var_bytes();
        let vd = vd.to_var_bytes();

        Ok((id, pk, vd))
    }

    pub fn compile_execute_circuit(
        inputs: usize,
        outputs: usize,
    ) -> Result<(&'static str, Vec<u8>, Vec<u8>)> {
        info!(
            "Starting the compilation of the circuit for {}/{}",
            inputs, outputs
        );

        let (ci, _, pk, vd, proof, pi) =
            ExecuteCircuit::create_dummy_proof::<_, MemStore>(
                &mut rand::thread_rng(),
                Some(<&PublicParameters>::from(&PUB_PARAMS).clone()),
                inputs,
                outputs,
                true,
                false,
            )?;

        info!(
            "Circuit generated with {}/{}",
            ci.inputs().len(),
            ci.outputs().len()
        );

        let id = ci.rusk_keys_id();

        // Sanity check
        circuit::verify_proof(
            &*PUB_PARAMS,
            vd.key(),
            &proof,
            pi.as_slice(),
            vd.pi_pos(),
            b"dusk-network",
        )
        .map_err(|_| anyhow!("Proof verification failed for {}", id))?;

        let pk = pk.to_var_bytes();
        let vd = vd.to_var_bytes();

        let mut hasher = Sha256::new();
        hasher.update(PUB_PARAMS.to_raw_var_bytes().as_slice());
        let contents = hasher.finalize();
        info!("Using PP {:x}", contents);

        let mut hasher = Sha256::new();
        hasher.update(vd.as_slice());
        let contents = hasher.finalize();

        let mut hasher = Sha256::new();
        let vk_p = VerifierData::from_slice(vd.as_slice()).expect("Data");
        hasher.update(&vk_p.key().to_bytes());
        let contents_key = hasher.finalize();

        info!(
            "Execute circuit data generated for {} with verifier data {:x} and key {:x}",
            id, contents, contents_key
        );

        Ok((id, pk, vd))
    }
}
