// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![allow(non_snake_case)]

/*
use bid_circuits::CorrectnessCircuit;
use dusk_blindbid::{bid::Bid, BlindBidCircuit};
use dusk_pki::{PublicSpendKey, SecretSpendKey};
use dusk_plonk::circuit_builder::Circuit;
use dusk_plonk::jubjub::{
    JubJubAffine, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use poseidon252::tree::PoseidonBranch;
*/

use dusk_plonk::prelude::*;
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

        unsafe {
            PublicParameters::from_slice_unchecked(&buff)
                .expect("CRS not decoded")
        }
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

    /*
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
    */

    // Get the cached keys for transfer contract crate from rusk profile, or
    // recompile and update them if they're outdated
    let transfer_keys = rusk_profile::keys_for("transfer-circuits");
    if transfer_keys.are_outdated() {
        let (id, pk, vk) = transfer::compile_stct_circuit()?;
        transfer_keys.update(id.as_str(), (pk, vk))?;

        let (id, pk, vk) = transfer::compile_stco_circuit()?;
        transfer_keys.update(id.as_str(), (pk, vk))?;

        // The execute circuit has multiple variations,
        // which is dependant upon the number of input
        // and output notes and is denoted in the table below:
        for inputs in 1..5 {
            for outputs in 0..3 {
                let (id, pk, vk) =
                    transfer::compile_execute_circuit(inputs, outputs)?;

                transfer_keys.update(id.as_str(), (pk, vk))?;
            }
        }
    }

    Ok(())
}

/*
mod bid {
    use super::*;

    pub fn compile_circuit() -> Result<(Vec<u8>, Vec<u8>)> {
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

    pub fn compile_circuit() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;

        // Generate a correct Bid
        let secret = JubJubScalar::random(&mut rand::thread_rng());
        let secret_k = BlsScalar::random(&mut rand::thread_rng());
        let bid = random_bid(&secret, secret_k)?;
        let secret: JubJubAffine = (GENERATOR_EXTENDED * secret).into();

        // Generate fields for the Bid & required by the compute_score
        let consensus_round_seed = 50u64;
        let latest_consensus_round = 50u64;
        let latest_consensus_step = 50u64;

        // Extract the branch
        let branch = PoseidonBranch::<17>::default();

        // Generate a `Score` for our Bid with the consensus parameters
        let score = bid
            .compute_score(
                &secret,
                secret_k,
                branch.root(),
                consensus_round_seed,
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

    fn random_bid(secret: &JubJubScalar, secret_k: BlsScalar) -> Result<Bid> {
        let mut rng = rand::thread_rng();
        let pk_r = PublicSpendKey::from(SecretSpendKey::default());
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
        .map_err(|e| anyhow::anyhow!(format!("{:?}", e)))
    }
}
*/

mod transfer {
    use super::PUB_PARAMS;
    use std::convert::TryInto;

    use anyhow::{anyhow, Result};
    use canonical_host::MemStore;
    use dusk_pki::{Ownable, SecretKey, SecretSpendKey};
    use dusk_plonk::jubjub::GENERATOR_EXTENDED;
    use dusk_plonk::prelude::*;
    use phoenix_core::{Message, Note};
    use poseidon252::sponge;
    use schnorr::Signature;
    use transfer_circuits::{
        ExecuteCircuit, SendToContractObfuscatedCircuit,
        SendToContractTransparentCircuit,
    };

    pub fn compile_stco_circuit() -> Result<(String, Vec<u8>, Vec<u8>)> {
        let mut rng = rand::thread_rng();

        let ssk = SecretSpendKey::random(&mut rng);
        let psk = ssk.public_spend_key();

        let c_value = 100;
        let c_blinding_factor = JubJubScalar::random(&mut rng);

        let c_note =
            Note::obfuscated(&mut rng, &psk, c_value, c_blinding_factor);
        let c_sk_r = ssk.sk_r(c_note.stealth_address()).as_ref().clone();
        let c_pk_r = GENERATOR_EXTENDED * c_sk_r;

        let (_, crossover) = c_note.try_into().map_err(|e| {
            anyhow!("Failed to convert phoenix note into crossover: {:?}", e)
        })?;
        let c_value_commitment = *crossover.value_commitment();

        let c_schnorr_secret = SecretKey::from(c_sk_r);
        let c_commitment_hash =
            sponge::hash(&c_value_commitment.to_hash_inputs());
        let c_signature =
            Signature::new(&c_schnorr_secret, &mut rng, c_commitment_hash);

        let message_r = JubJubScalar::random(&mut rng);
        let message_value = 100;
        let message = Message::new(&mut rng, &message_r, &psk, message_value);
        let (_, message_blinding_factor) = message
            .decrypt(&message_r, &psk)
            .map_err(|e| anyhow!("Error decrypting the message: {:?}", e))?;

        let mut circuit = SendToContractObfuscatedCircuit::new(
            c_value_commitment,
            c_pk_r,
            c_value,
            c_blinding_factor,
            c_signature,
            message_value,
            message_blinding_factor,
            message_r,
            *psk.A(),
            *message.value_commitment(),
            *message.nonce(),
            *message.cipher(),
        );

        let (pk, vk) = circuit.compile(&PUB_PARAMS)?;

        let id = circuit.rusk_label();
        let pk = pk.to_bytes();
        let vk = vk.to_bytes();

        Ok((id, pk, vk))
    }

    pub fn compile_stct_circuit() -> Result<(String, Vec<u8>, Vec<u8>)> {
        let mut rng = rand::thread_rng();

        let c_ssk = SecretSpendKey::random(&mut rng);
        let c_psk = c_ssk.public_spend_key();

        let c_value = 100;
        let c_blinding_factor = JubJubScalar::random(&mut rng);

        let c_note =
            Note::obfuscated(&mut rng, &c_psk, c_value, c_blinding_factor);
        let c_sk_r = c_ssk.sk_r(c_note.stealth_address());
        let c_pk_r = GENERATOR_EXTENDED * c_sk_r.as_ref();

        let (_, crossover) = c_note.try_into().map_err(|e| {
            anyhow!("Failed to convert note to crossover: {:?}", e)
        })?;
        let c_value_commitment = *crossover.value_commitment();

        let c_schnorr_secret = SecretKey::from(c_sk_r);
        let c_commitment_hash =
            sponge::hash(&c_value_commitment.to_hash_inputs());
        let c_signature =
            Signature::new(&c_schnorr_secret, &mut rng, c_commitment_hash);

        let mut circuit = SendToContractTransparentCircuit::new(
            c_value_commitment,
            c_pk_r,
            c_value,
            c_blinding_factor,
            c_signature,
        );

        let (pk, vk) = circuit.compile(&PUB_PARAMS)?;

        let id = circuit.rusk_label();
        let pk = pk.to_bytes();
        let vk = vk.to_bytes();

        Ok((id, pk, vk))
    }

    pub fn compile_execute_circuit(
        inputs: usize,
        outputs: usize,
    ) -> Result<(String, Vec<u8>, Vec<u8>)> {
        let (id, pk, vk) = match inputs {
            1 => get_id_pk_vk::<15>(inputs, outputs)?,
            2 => get_id_pk_vk::<16>(inputs, outputs)?,
            3 | 4 => get_id_pk_vk::<17>(inputs, outputs)?,
            _ => unimplemented!(),
        };

        let pk = pk.to_bytes();
        let vk = vk.to_bytes();

        Ok((id, pk, vk))
    }

    fn get_id_pk_vk<const CAPACITY: usize>(
        inputs: usize,
        outputs: usize,
    ) -> Result<(String, ProverKey, VerifierKey)> {
        let (ci, _, pk, vk, _, _) =
            ExecuteCircuit::<17, CAPACITY>::create_dummy_proof::<_, MemStore>(
                &mut rand::thread_rng(),
                false,
                Some(<&PublicParameters>::from(&PUB_PARAMS).clone()),
                inputs,
                outputs,
            )?;

        let id = ci.rusk_label();

        Ok((id, pk, vk))
    }
}
