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
    JubJubAffine as AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::PublicParameters;
use dusk_plonk::prelude::*;
use kelvin::Blake2b;
use lazy_static::lazy_static;
use poseidon252::sponge::sponge::{sponge_hash, sponge_hash_gadget};
use poseidon252::tree::{PoseidonAnnotation, PoseidonBranch, PoseidonTree};
use transfer_circuits::dusk_contract::{
    ExecuteCircuit, SendToContractObfuscatedCircuit,
    SendToContractTransparentCircuit, WithdrawFromContractObfuscatedCircuit,
    WithdrawFromObfuscatedToContractCircuitOne,
    WithdrawFromObfuscatedToContractCircuitTwo,
};
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

    // Get the cached keys for transfer contract crate from rusk profile, or
    // recompile and update them if they're outdated
    let transfer_keys = rusk_profile::keys_for("transfer-circuits");
    if transfer_keys.are_outdated() {
        transfer_keys.update(
            "SendToContractTransparent",
            transfer::compile_stct_circuit()?,
        )?;
        transfer_keys.update(
            "SendToContractObfuscated",
            transfer::compile_stco_circuit()?,
        )?;
        transfer_keys.update(
            "WithdrawFromObfuscated",
            transfer::compile_wfo_circuit()?,
        )?;
        transfer_keys.update(
            "WithdrawFromObfuscatedToContractOne",
            transfer::compile_wfotco_circuit()?,
        )?;
        transfer_keys.update(
            "WithdrawFromObfuscatedToContractTwo",
            transfer::compile_wfotct_circuit()?,
        )?;
        for i in 1..13 {
            transfer_keys.update(
                &format!("Execute{}", i),
                transfer::compile_execute_circuit(i)?,
            )?;
        }
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

mod transfer {
    use super::*;
    // This function signs a message with a secret key
    // and produces a public key to allow for the proof
    // of knowledge of a DLP.
    fn single_schnorr_sign(
        sk: JubJubScalar,
        message: BlsScalar,
    ) -> (JubJubScalar, AffinePoint, AffinePoint) {
        let pk = AffinePoint::from(GENERATOR_EXTENDED * sk);
        let r = JubJubScalar::random(&mut rand::thread_rng());
        let R = AffinePoint::from(GENERATOR_EXTENDED * r);
        let h = sponge_hash(&[message]);
        let c_hash = sponge_hash(&[R.get_x(), R.get_y(), h]);
        let c_hash = c_hash & BlsScalar::pow_of_2(250).sub(&BlsScalar::one());
        let c = JubJubScalar::from_bytes(&c_hash.to_bytes()).unwrap();
        let U = r - (c * sk);
        (U, R, pk)
    }
    // This function is used to create a Schnorr signature
    // which are then verified in the circuit. The schnorr
    // signature here, works with a pair of public keys.
    fn double_schnorr_sign(
        sk: JubJubScalar,
        message: BlsScalar,
    ) -> (JubJubScalar, AffinePoint, AffinePoint, AffinePoint) {
        let pk_prime = AffinePoint::from(GENERATOR_NUMS_EXTENDED * sk);
        let r = JubJubScalar::random(&mut rand::thread_rng());
        let R = AffinePoint::from(GENERATOR_EXTENDED * r);
        let R_prime = AffinePoint::from(GENERATOR_NUMS_EXTENDED * r);
        let h = sponge_hash(&[message]);
        let c_hash = sponge_hash(&[
            R.get_x(),
            R.get_y(),
            R_prime.get_x(),
            R_prime.get_y(),
            h,
        ]);
        let c_hash = c_hash & BlsScalar::pow_of_2(250).sub(&BlsScalar::one());
        let c = JubJubScalar::from_bytes(&c_hash.to_bytes()).unwrap();
        let U = r - (c * sk);
        (U, R, R_prime, pk_prime)
    }

    pub fn compile_stct_circuit() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;
        let commitment_value = JubJubScalar::from(319 as u64);
        let commitment_blinder = JubJubScalar::from(157 as u64);
        let c_p = AffinePoint::from(
            (GENERATOR_EXTENDED * commitment_value)
                + (GENERATOR_NUMS_EXTENDED * commitment_blinder),
        );
        let note_value = BlsScalar::from(319);

        let message = BlsScalar::random(&mut rand::thread_rng());
        let sk = JubJubScalar::random(&mut rand::thread_rng());
        let sig = single_schnorr_sign(sk, message);
        let public_key = AffinePoint::from(GENERATOR_EXTENDED * sk);

        let mut circuit = SendToContractTransparentCircuit {
            commitment_value: commitment_value.into(),
            commitment_blinder: commitment_blinder.into(),
            commitment: c_p,
            value: note_value,
            pk: public_key,
            schnorr_sig: sig.0,
            schnorr_r: sig.1,
            schnorr_pk: sig.2,
            schnorr_message: message,
            trim_size: 1 << 13,
            pi_positions: vec![],
        };

        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }

    pub fn compile_stco_circuit() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;
        let crossover_value = JubJubScalar::from(300 as u64);
        let crossover_blinder = JubJubScalar::from(150 as u64);
        let c_p = AffinePoint::from(
            (GENERATOR_EXTENDED * crossover_value)
                + (GENERATOR_NUMS_EXTENDED * crossover_blinder),
        );
        let message_value = JubJubScalar::from(300);
        let message_blinder = JubJubScalar::from(199);
        let m = AffinePoint::from(
            (GENERATOR_EXTENDED * message_value)
                + (GENERATOR_NUMS_EXTENDED * message_blinder),
        );

        let sk = JubJubScalar::random(&mut rand::thread_rng());
        let schnorr_m = BlsScalar::random(&mut rand::thread_rng());
        let sig = single_schnorr_sign(sk, schnorr_m);
        let public_key = AffinePoint::from(GENERATOR_EXTENDED * sk);

        let mut circuit = SendToContractObfuscatedCircuit {
            commitment_crossover_value: crossover_value.into(),
            commitment_crossover_blinder: crossover_blinder.into(),
            commitment_crossover: c_p,
            commitment_message_value: message_value.into(),
            commitment_message_blinder: message_blinder.into(),
            commitment_message: m,
            pk: public_key,
            schnorr_sig: sig.0,
            schnorr_r: sig.1,
            schnorr_pk: sig.2,
            schnorr_message: schnorr_m,
            trim_size: 1 << 13,
            pi_positions: vec![],
        };

        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }

    pub fn compile_wfo_circuit() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;
        let spend_value = JubJubScalar::from(300 as u64);
        let spend_blinder = JubJubScalar::from(150 as u64);
        let s_c = AffinePoint::from(
            (GENERATOR_EXTENDED * spend_value)
                + (GENERATOR_NUMS_EXTENDED * spend_blinder),
        );
        let message_value = JubJubScalar::from(200 as u64);
        let message_blinder = JubJubScalar::from(199);
        let m_c = AffinePoint::from(
            (GENERATOR_EXTENDED * message_value)
                + (GENERATOR_NUMS_EXTENDED * message_blinder),
        );
        let note_value = JubJubScalar::from(100 as u64);
        let note_blinder = JubJubScalar::from(318 as u64);
        let n_c = AffinePoint::from(
            (GENERATOR_EXTENDED * note_value)
                + (GENERATOR_NUMS_EXTENDED * note_blinder),
        );

        let mut circuit = WithdrawFromContractObfuscatedCircuit {
            spend_commitment_value: spend_value.into(),
            spend_commitment_blinder: spend_blinder.into(),
            spend_commitment: s_c,
            message_commitment_value: message_value.into(),
            message_commitment_blinder: message_blinder.into(),
            message_commitment: m_c,
            note_commitment_value: note_value.into(),
            note_commitment_blinder: note_blinder.into(),
            note_commitment: n_c,
            trim_size: 1 << 12,
            pi_positions: vec![],
        };

        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }

    pub fn compile_wfotco_circuit() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;

        let commitment_value = JubJubScalar::from(100 as u64);
        let commitment_blinder = JubJubScalar::from(318 as u64);
        let n_c = AffinePoint::from(
            (GENERATOR_EXTENDED * commitment_value)
                + (GENERATOR_NUMS_EXTENDED * commitment_blinder),
        );

        let spend_value = JubJubScalar::from(300 as u64);
        let spend_blinder = JubJubScalar::from(150 as u64);
        let s_c = AffinePoint::from(
            (GENERATOR_EXTENDED * spend_value)
                + (GENERATOR_NUMS_EXTENDED * spend_blinder),
        );

        let message_value = JubJubScalar::from(200 as u64);
        let message_blinder = JubJubScalar::from(199);
        let m_c = AffinePoint::from(
            (GENERATOR_EXTENDED * message_value)
                + (GENERATOR_NUMS_EXTENDED * message_blinder),
        );

        let mut circuit = WithdrawFromObfuscatedCircuitToContractOne {
            commitment_value: commitment_value.into(),
            commitment_blinder: commitment_blinder.into(),
            commitment_point: s_c,
            spend_commitment_value: spend_value.into(),
            spend_commitment_blinder: spend_blinder.into(),
            spend_commitment: s_c,
            change_commitment_value: message_value,
            change_commitment_blinder: message_blinder.into(),
            change_commitment: m_c,
            trim_size: 1 << 10,
            pi_positions: vec![],
        };

        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }

    pub fn compile_wfotct_circuit() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;

        let commitment_value = JubJubScalar::from(100 as u64);
        let commitment_blinder = JubJubScalar::from(318 as u64);
        let n_c = AffinePoint::from(
            (GENERATOR_EXTENDED * commitment_value)
                + (GENERATOR_NUMS_EXTENDED * commitment_blinder),
        );

        let spend_value = JubJubScalar::from(300 as u64);
        let spend_blinder = JubJubScalar::from(150 as u64);
        let s_c = AffinePoint::from(
            (GENERATOR_EXTENDED * spend_value)
                + (GENERATOR_NUMS_EXTENDED * spend_blinder),
        );

        let message_value = JubJubScalar::from(200 as u64);
        let message_blinder = JubJubScalar::from(199);
        let m_c = AffinePoint::from(
            (GENERATOR_EXTENDED * message_value)
                + (GENERATOR_NUMS_EXTENDED * message_blinder),
        );

        let mut circuit = WithdrawFromObfuscatedCircuitToContractTwo {
            // commitment_value: commitment_value.into(),
            // commitment_blinder: commitment_blinder.into(),
            // commitment_point: s_c,
            // spend_commitment_value: spend_value.into(),
            // spend_commitment_blinder: spend_blinder.into(),
            // spend_commitment: s_c,
            // change_commitment_value: message_value,
            // change_commitment_blinder: message_blinder.into(),
            // change_commitment: m_c,
            // trim_size: 1 << 10,
            // pi_positions: vec![],
        };

        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }

    // pub fn compile_execute_circuit(1) -> Result<(Vec<u8>, Vec<u8>)> {
    //     let pub_params = &PUB_PARAMS;
    //     let commitment_value = JubJubScalar::from(319 as u64);
    //     let commitment_blinder = JubJubScalar::from(157 as u64);
    //     let c = AffinePoint::from(
    //         (GENERATOR_EXTENDED * commitment_value) +
    // (GENERATOR_NUMS_EXTENDED * commitment_blinder),     );
    //     let note_value = BlsScalar::from(319);
    //
    //     let mut circuit = ExecuteCircuit {
    //         commitment_value: commitment_value.into(),
    //         commitment_blinder: commitment_blinder.into(),
    //         commitment: c,
    //         value: note_value,
    //         trim_size: 1 << 10,
    //         pi_positions: vec![],
    //     };
    //
    //     let (pk, vk) = circuit.compile(&pub_params)?;
    //     Ok((pk.to_bytes(), vk.to_bytes()))
    // }
}
