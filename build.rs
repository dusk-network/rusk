// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![allow(non_snake_case)]
use anyhow::Result;
use bid_circuits::CorrectnessCircuit;
use dusk_blindbid::{bid::Bid, BlindBidCircuit};
use dusk_pki::{Ownable, PublicSpendKey, SecretSpendKey};
use dusk_plonk::circuit_builder::Circuit;
use dusk_plonk::jubjub::{
    JubJubAffine, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::PublicParameters;
use dusk_plonk::prelude::*;
use lazy_static::lazy_static;
use phoenix_core::{Note, NoteType};
use poseidon252::sponge::hash;
use poseidon252::tree::PoseidonBranch;
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
            let fnctn = match i {
                1 => transfer::compile_execute_circuit_1,
                2 => transfer::compile_execute_circuit_2,
                3 => transfer::compile_execute_circuit_3,
                4 => transfer::compile_execute_circuit_4,
                5 => transfer::compile_execute_circuit_5,
                6 => transfer::compile_execute_circuit_6,
                7 => transfer::compile_execute_circuit_7,
                8 => transfer::compile_execute_circuit_8,
                9 => transfer::compile_execute_circuit_9,
                10 => transfer::compile_execute_circuit_10,
                11 => transfer::compile_execute_circuit_11,
                12 => transfer::compile_execute_circuit_12,
                _ => transfer::compile_execute_circuit_1,
            };
            transfer_keys.update(&format!("Execute{}", i), fnctn()?)?;
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

mod transfer {
    use super::*;

    // This function signs a message with a secret key
    // and produces a public key to allow for the proof
    // of knowledge of a DLP.
    fn single_schnorr_sign(
        sk: JubJubScalar,
        message: BlsScalar,
    ) -> (JubJubScalar, JubJubAffine, JubJubAffine) {
        let pk = JubJubAffine::from(GENERATOR_EXTENDED * sk);
        let r = JubJubScalar::random(&mut rand::thread_rng());
        let R = JubJubAffine::from(GENERATOR_EXTENDED * r);
        let h = hash(&[message]);
        let c_hash = hash(&[R.get_x(), R.get_y(), h]);
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
    ) -> (JubJubScalar, JubJubAffine, JubJubAffine, JubJubAffine) {
        let pk_prime = JubJubAffine::from(GENERATOR_NUMS_EXTENDED * sk);
        let r = JubJubScalar::random(&mut rand::thread_rng());
        let R = JubJubAffine::from(GENERATOR_EXTENDED * r);
        let R_prime = JubJubAffine::from(GENERATOR_NUMS_EXTENDED * r);
        let h = hash(&[message]);
        let c_hash =
            hash(&[R.get_x(), R.get_y(), R_prime.get_x(), R_prime.get_y(), h]);
        let c_hash = c_hash & BlsScalar::pow_of_2(250).sub(&BlsScalar::one());
        let c = JubJubScalar::from_bytes(&c_hash.to_bytes()).unwrap();
        let U = r - (c * sk);
        (U, R, R_prime, pk_prime)
    }
    // Function to create deterministic note from chosen instantiated parameters
    fn circuit_note(
        ssk: SecretSpendKey,
        value: u64,
        pos: u64,
        input_note_blinder: JubJubScalar,
    ) -> Note {
        let r = JubJubScalar::from(150 as u64);
        let nonce = JubJubScalar::from(350 as u64);
        let psk = PublicSpendKey::from(&ssk);
        let mut note = Note::deterministic(
            NoteType::Transparent,
            &r,
            nonce,
            &psk,
            value,
            input_note_blinder,
        );
        note.set_pos(pos);
        note
    }
    // Function to generate value commitment from value and blinder. This is a
    // pedersen commitment.
    fn compute_value_commitment(
        value: JubJubScalar,
        blinder: JubJubScalar,
    ) -> JubJubAffine {
        let commitment = JubJubAffine::from(
            &(GENERATOR_EXTENDED * value)
                + &(GENERATOR_NUMS_EXTENDED * blinder),
        );

        commitment
    }

    pub fn compile_stct_circuit() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;
        let commitment_value = JubJubScalar::from(319 as u64);
        let commitment_blinder = JubJubScalar::from(157 as u64);
        let c_p =
            compute_value_commitment(commitment_value, commitment_blinder);

        let note_value = BlsScalar::from(319);

        let message = BlsScalar::random(&mut rand::thread_rng());
        let sk = JubJubScalar::random(&mut rand::thread_rng());
        let sig = single_schnorr_sign(sk, message);
        let public_key = JubJubAffine::from(GENERATOR_EXTENDED * sk);

        let mut circuit = SendToContractTransparentCircuit {
            commitment_value: commitment_value.into(),
            blinder: commitment_blinder.into(),
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
        let c_p = compute_value_commitment(crossover_value, crossover_blinder);

        let message_value = JubJubScalar::from(300 as u64);
        let message_blinder = JubJubScalar::from(199 as u64);
        let m = compute_value_commitment(message_value, message_blinder);

        let sk = JubJubScalar::random(&mut rand::thread_rng());
        let schnorr_m = BlsScalar::random(&mut rand::thread_rng());
        let sig = single_schnorr_sign(sk, schnorr_m);
        let public_key = JubJubAffine::from(GENERATOR_EXTENDED * sk);

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
        let s_c = compute_value_commitment(spend_value, spend_blinder);
        let message_value = JubJubScalar::from(200 as u64);
        let message_blinder = JubJubScalar::from(199u64);
        let m_c = compute_value_commitment(message_value, message_blinder);
        let note_value = JubJubScalar::from(100 as u64);
        let note_blinder = JubJubScalar::from(318 as u64);
        let n_c = compute_value_commitment(note_value, note_blinder);

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
        let n_c =
            compute_value_commitment(commitment_value, commitment_blinder);

        let spend_value = JubJubScalar::from(300 as u64);
        let spend_blinder = JubJubScalar::from(150 as u64);
        let s_c = compute_value_commitment(spend_value, spend_blinder);

        let change_value = JubJubScalar::from(200 as u64);
        let change_blinder = JubJubScalar::from(199 as u64);
        let m_c = compute_value_commitment(change_value, change_blinder);

        let mut circuit = WithdrawFromObfuscatedToContractCircuitOne {
            commitment_value: commitment_value.into(),
            commitment_blinder: commitment_blinder.into(),
            commitment_point: n_c,
            spend_commitment_value: spend_value.into(),
            spend_commitment_blinder: spend_blinder.into(),
            spend_commitment: s_c,
            change_commitment_value: change_value.into(),
            change_commitment_blinder: change_blinder.into(),
            change_commitment: m_c,
            trim_size: 1 << 12,
            pi_positions: vec![],
        };

        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }

    pub fn compile_wfotct_circuit() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;

        let commitment_value = JubJubScalar::from(100 as u64);
        let commitment_blinder = JubJubScalar::from(318 as u64);
        let n_c =
            compute_value_commitment(commitment_value, commitment_blinder);

        let value = BlsScalar::from(300 as u64);

        let change_message_value = JubJubScalar::from(200 as u64);
        let change_message_blinder = JubJubScalar::from(199 as u64);
        let m_c = compute_value_commitment(
            change_message_value,
            change_message_blinder,
        );

        let mut circuit = WithdrawFromObfuscatedToContractCircuitTwo {
            commitment_value: commitment_value.into(),
            commitment_blinder: commitment_blinder.into(),
            commitment_point: n_c,
            change_commitment_value: change_message_value.into(),
            change_commitment_blinder: change_message_blinder.into(),
            change_commitment: m_c,
            value,
            trim_size: 1 << 12,
            pi_positions: vec![],
        };

        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }

    // The execute circuit has multiple variations,
    // which is dependant upon the number of input
    // and output notes and is denoted in the table below:
    // __________________________________________
    // |Variation_|_Inputs_notes_|_Output_Notes_|
    // |1         |______1_______|______0_______|
    // |2         |______1_______|______1_______|
    // |3         |______1_______|______2_______|
    // |4         |______2_______|______0_______|
    // |5         |______2_______|______1_______|
    // |6         |______2_______|______2_______|
    // |7         |______3_______|______0_______|
    // |8         |______3_______|______1_______|
    // |9         |______3_______|______2_______|
    // |10        |______4_______|______0_______|
    // |12        |______4_______|______1_______|
    // |13________|______4_______|______2_______|

    pub fn compile_execute_circuit_1() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let value1 = 400u64;
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = circuit_note(ssk1, value1, 0, input_note_blinder_one);
        note1.set_pos(0);
        let input_note_value_one = JubJubScalar::from(value1);
        let input_commitment_one = compute_value_commitment(
            input_note_value_one,
            input_note_blinder_one,
        );

        let crossover_commitment_value = JubJubScalar::from(200 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = compute_value_commitment(
            crossover_commitment_value,
            crossover_commitment_blinder,
        );

        let sig1 = double_schnorr_sign(
            ssk1.sk_r(note1.stealth_address()),
            BlsScalar::one(),
        );

        let fee = BlsScalar::from(200);

        let mut circuit = ExecuteCircuit {
            nullifiers: vec![note1.gen_nullifier(&ssk1)],
            note_hashes: vec![note1.hash()],
            position_of_notes: vec![BlsScalar::from(note1.pos())],
            input_note_types: vec![BlsScalar::from(note1.note() as u64)],
            input_poseidon_branches: vec![PoseidonBranch::<17>::default()],
            input_notes_sk: vec![ssk1.sk_r(note1.stealth_address())],
            input_notes_pk: vec![JubJubAffine::from(
                note1.stealth_address().pk_r(),
            )],
            input_notes_pk_prime: vec![sig1.3],
            input_commitments: vec![input_commitment_one],
            input_nonces: vec![*note1.nonce()],
            input_values: vec![input_note_value_one.into()],
            input_blinders: vec![input_note_blinder_one.into()],
            input_randomness: vec![note1.stealth_address().R().into()],
            input_ciphers_one: vec![note1.cipher()[0]],
            input_ciphers_two: vec![note1.cipher()[1]],
            input_ciphers_three: vec![note1.cipher()[2]],
            schnorr_sigs: vec![sig1.0],
            schnorr_r: vec![sig1.1],
            schnorr_r_prime: vec![sig1.2],
            schnorr_messages: vec![BlsScalar::one()],
            crossover_commitment,
            crossover_commitment_value: crossover_commitment_value.into(),
            crossover_commitment_blinder: crossover_commitment_blinder.into(),
            obfuscated_commitment_points: vec![],
            obfuscated_note_values: vec![],
            obfuscated_note_blinders: vec![],
            fee,
            trim_size: 1 << 16,
            pi_positions: vec![],
        };

        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }

    pub fn compile_execute_circuit_2() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let value1 = 500u64;
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = circuit_note(ssk1, value1, 0, input_note_blinder_one);
        note1.set_pos(0);
        let input_note_value_one = JubJubScalar::from(value1);
        let input_commitment_one = compute_value_commitment(
            input_note_value_one,
            input_note_blinder_one,
        );

        let crossover_commitment_value = JubJubScalar::from(200 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = compute_value_commitment(
            crossover_commitment_value,
            crossover_commitment_blinder,
        );
        let obfuscated_note_value_one = JubJubScalar::from(100 as u64);
        let obfuscated_note_blinder_one = JubJubScalar::from(100 as u64);
        let obfuscated_commitment_one = compute_value_commitment(
            obfuscated_note_value_one,
            obfuscated_note_blinder_one,
        );

        let sig1 = double_schnorr_sign(
            ssk1.sk_r(note1.stealth_address()),
            BlsScalar::one(),
        );

        let fee = BlsScalar::from(200);

        let mut circuit = ExecuteCircuit {
            nullifiers: vec![note1.gen_nullifier(&ssk1)],
            note_hashes: vec![note1.hash()],
            position_of_notes: vec![BlsScalar::from(note1.pos())],
            input_note_types: vec![BlsScalar::from(note1.note() as u64)],
            input_poseidon_branches: vec![PoseidonBranch::<17>::default()],
            input_notes_sk: vec![ssk1.sk_r(note1.stealth_address())],
            input_notes_pk: vec![JubJubAffine::from(
                note1.stealth_address().pk_r(),
            )],
            input_notes_pk_prime: vec![sig1.3],
            input_commitments: vec![input_commitment_one],
            input_nonces: vec![*note1.nonce()],
            input_values: vec![input_note_value_one.into()],
            input_blinders: vec![input_note_blinder_one.into()],
            input_randomness: vec![note1.stealth_address().R().into()],
            input_ciphers_one: vec![note1.cipher()[0]],
            input_ciphers_two: vec![note1.cipher()[1]],
            input_ciphers_three: vec![note1.cipher()[2]],
            schnorr_sigs: vec![sig1.0],
            schnorr_r: vec![sig1.1],
            schnorr_r_prime: vec![sig1.2],
            schnorr_messages: vec![BlsScalar::one()],
            crossover_commitment,
            crossover_commitment_value: crossover_commitment_value.into(),
            crossover_commitment_blinder: crossover_commitment_blinder.into(),
            obfuscated_commitment_points: vec![obfuscated_commitment_one],
            obfuscated_note_values: vec![obfuscated_note_value_one.into()],
            obfuscated_note_blinders: vec![obfuscated_note_blinder_one.into()],
            fee,
            trim_size: 1 << 16,
            pi_positions: vec![],
        };

        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }

    pub fn compile_execute_circuit_3() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let value1 = 600u64;
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = circuit_note(ssk1, value1, 0, input_note_blinder_one);
        note1.set_pos(0);
        let input_note_value_one = JubJubScalar::from(value1);
        let input_commitment_one = compute_value_commitment(
            input_note_value_one,
            input_note_blinder_one,
        );

        let crossover_commitment_value = JubJubScalar::from(200 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = compute_value_commitment(
            crossover_commitment_value,
            crossover_commitment_blinder,
        );
        let obfuscated_note_value_one = JubJubScalar::from(100 as u64);
        let obfuscated_note_blinder_one = JubJubScalar::from(100 as u64);
        let obfuscated_commitment_one = compute_value_commitment(
            obfuscated_note_value_one,
            obfuscated_note_blinder_one,
        );
        let obfuscated_note_value_two = JubJubScalar::from(100 as u64);
        let obfuscated_note_blinder_two = JubJubScalar::from(200 as u64);
        let obfuscated_commitment_two = compute_value_commitment(
            obfuscated_note_value_two,
            obfuscated_note_blinder_two,
        );
        let sig1 = double_schnorr_sign(
            ssk1.sk_r(note1.stealth_address()),
            BlsScalar::one(),
        );

        let fee = BlsScalar::from(200);

        let mut circuit = ExecuteCircuit {
            nullifiers: vec![note1.gen_nullifier(&ssk1)],
            note_hashes: vec![note1.hash()],
            position_of_notes: vec![BlsScalar::from(note1.pos())],
            input_note_types: vec![BlsScalar::from(note1.note() as u64)],
            input_poseidon_branches: vec![PoseidonBranch::<17>::default()],
            input_notes_sk: vec![ssk1.sk_r(note1.stealth_address())],
            input_notes_pk: vec![JubJubAffine::from(
                note1.stealth_address().pk_r(),
            )],
            input_notes_pk_prime: vec![sig1.3],
            input_commitments: vec![input_commitment_one],
            input_nonces: vec![*note1.nonce()],
            input_values: vec![input_note_value_one.into()],
            input_blinders: vec![input_note_blinder_one.into()],
            input_randomness: vec![note1.stealth_address().R().into()],
            input_ciphers_one: vec![note1.cipher()[0]],
            input_ciphers_two: vec![note1.cipher()[1]],
            input_ciphers_three: vec![note1.cipher()[2]],
            schnorr_sigs: vec![sig1.0],
            schnorr_r: vec![sig1.1],
            schnorr_r_prime: vec![sig1.2],
            schnorr_messages: vec![BlsScalar::one()],
            crossover_commitment,
            crossover_commitment_value: crossover_commitment_value.into(),
            crossover_commitment_blinder: crossover_commitment_blinder.into(),
            obfuscated_commitment_points: vec![
                obfuscated_commitment_one,
                obfuscated_commitment_two,
            ],
            obfuscated_note_values: vec![
                obfuscated_note_value_one.into(),
                obfuscated_note_value_two.into(),
            ],
            obfuscated_note_blinders: vec![
                obfuscated_note_blinder_one.into(),
                obfuscated_note_blinder_two.into(),
            ],
            fee,
            trim_size: 1 << 16,
            pi_positions: vec![],
        };

        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }

    pub fn compile_execute_circuit_4() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let value1 = 200u64;
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = circuit_note(ssk1, value1, 0, input_note_blinder_one);
        note1.set_pos(0);
        let input_note_value_one = JubJubScalar::from(value1);
        let input_commitment_one = compute_value_commitment(
            input_note_value_one,
            input_note_blinder_one,
        );
        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        let value2 = 200u64;
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        let mut note2 = circuit_note(ssk2, value2, 0, input_note_blinder_two);
        note2.set_pos(1);
        let input_note_value_two = JubJubScalar::from(value2);
        let input_commitment_two = compute_value_commitment(
            input_note_value_two,
            input_note_blinder_two,
        );

        let crossover_commitment_value = JubJubScalar::from(200 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = compute_value_commitment(
            crossover_commitment_value,
            crossover_commitment_blinder,
        );

        let sig1 = double_schnorr_sign(
            ssk1.sk_r(note1.stealth_address()),
            BlsScalar::one(),
        );
        let sig2 = double_schnorr_sign(
            ssk2.sk_r(note2.stealth_address()),
            BlsScalar::one(),
        );
        let fee = BlsScalar::from(200);

        let mut circuit = ExecuteCircuit {
            nullifiers: vec![
                note1.gen_nullifier(&ssk1),
                note2.gen_nullifier(&ssk2),
            ],
            note_hashes: vec![note1.hash(), note2.hash()],
            position_of_notes: vec![
                BlsScalar::from(note1.pos()),
                BlsScalar::from(note2.pos()),
            ],
            input_note_types: vec![
                BlsScalar::from(note1.note() as u64),
                BlsScalar::from(note2.note() as u64),
            ],
            input_poseidon_branches: vec![
                PoseidonBranch::<17>::default(),
                PoseidonBranch::<17>::default(),
            ],
            input_notes_sk: vec![
                ssk1.sk_r(note1.stealth_address()),
                ssk2.sk_r(note2.stealth_address()),
            ],
            input_notes_pk: vec![
                JubJubAffine::from(note1.stealth_address().pk_r()),
                JubJubAffine::from(note2.stealth_address().pk_r()),
            ],
            input_notes_pk_prime: vec![sig1.3, sig2.3],
            input_commitments: vec![input_commitment_one, input_commitment_two],
            input_nonces: vec![*note1.nonce(), *note2.nonce()],
            input_values: vec![
                input_note_value_one.into(),
                input_note_value_two.into(),
            ],
            input_blinders: vec![
                input_note_blinder_one.into(),
                input_note_blinder_two.into(),
            ],
            input_randomness: vec![
                note1.stealth_address().R().into(),
                note2.stealth_address().R().into(),
            ],
            input_ciphers_one: vec![note1.cipher()[0], note2.cipher()[0]],
            input_ciphers_two: vec![note1.cipher()[1], note2.cipher()[1]],
            input_ciphers_three: vec![note1.cipher()[2], note2.cipher()[2]],
            schnorr_sigs: vec![sig1.0, sig2.0],
            schnorr_r: vec![sig1.1, sig2.1],
            schnorr_r_prime: vec![sig1.2, sig2.2],
            schnorr_messages: vec![BlsScalar::one(), BlsScalar::one()],
            crossover_commitment,
            crossover_commitment_value: crossover_commitment_value.into(),
            crossover_commitment_blinder: crossover_commitment_blinder.into(),
            obfuscated_commitment_points: vec![],
            obfuscated_note_values: vec![],
            obfuscated_note_blinders: vec![],
            fee,
            trim_size: 1 << 16,
            pi_positions: vec![],
        };

        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }
    pub fn compile_execute_circuit_5() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let value1 = 400u64;
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = circuit_note(ssk1, value1, 0, input_note_blinder_one);
        note1.set_pos(0);
        let input_note_value_one = JubJubScalar::from(value1);
        let input_commitment_one = compute_value_commitment(
            input_note_value_one,
            input_note_blinder_one,
        );
        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        let value2 = 200u64;
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        let mut note2 = circuit_note(ssk2, value2, 0, input_note_blinder_two);
        note2.set_pos(1);
        let input_note_value_two = JubJubScalar::from(value2);
        let input_commitment_two = compute_value_commitment(
            input_note_value_two,
            input_note_blinder_two,
        );

        let crossover_commitment_value = JubJubScalar::from(200 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = compute_value_commitment(
            crossover_commitment_value,
            crossover_commitment_blinder,
        );
        let obfuscated_note_value_one = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_one = JubJubScalar::from(100 as u64);
        let obfuscated_commitment_one = compute_value_commitment(
            obfuscated_note_value_one,
            obfuscated_note_blinder_one,
        );

        let sig1 = double_schnorr_sign(
            ssk1.sk_r(note1.stealth_address()),
            BlsScalar::one(),
        );
        let sig2 = double_schnorr_sign(
            ssk2.sk_r(note2.stealth_address()),
            BlsScalar::one(),
        );
        let fee = BlsScalar::from(200);

        let mut circuit = ExecuteCircuit {
            nullifiers: vec![
                note1.gen_nullifier(&ssk1),
                note2.gen_nullifier(&ssk2),
            ],
            note_hashes: vec![note1.hash(), note2.hash()],
            position_of_notes: vec![
                BlsScalar::from(note1.pos()),
                BlsScalar::from(note2.pos()),
            ],
            input_note_types: vec![
                BlsScalar::from(note1.note() as u64),
                BlsScalar::from(note2.note() as u64),
            ],
            input_poseidon_branches: vec![
                PoseidonBranch::<17>::default(),
                PoseidonBranch::<17>::default(),
            ],
            input_notes_sk: vec![
                ssk1.sk_r(note1.stealth_address()),
                ssk2.sk_r(note2.stealth_address()),
            ],
            input_notes_pk: vec![
                JubJubAffine::from(note1.stealth_address().pk_r()),
                JubJubAffine::from(note2.stealth_address().pk_r()),
            ],
            input_notes_pk_prime: vec![sig1.3, sig2.3],
            input_commitments: vec![input_commitment_one, input_commitment_two],
            input_nonces: vec![*note1.nonce(), *note2.nonce()],
            input_values: vec![
                input_note_value_one.into(),
                input_note_value_two.into(),
            ],
            input_blinders: vec![
                input_note_blinder_one.into(),
                input_note_blinder_two.into(),
            ],
            input_randomness: vec![
                note1.stealth_address().R().into(),
                note2.stealth_address().R().into(),
            ],
            input_ciphers_one: vec![note1.cipher()[0], note2.cipher()[0]],
            input_ciphers_two: vec![note1.cipher()[1], note2.cipher()[1]],
            input_ciphers_three: vec![note1.cipher()[2], note2.cipher()[2]],
            schnorr_sigs: vec![sig1.0, sig2.0],
            schnorr_r: vec![sig1.1, sig2.1],
            schnorr_r_prime: vec![sig1.2, sig2.2],
            schnorr_messages: vec![BlsScalar::one(), BlsScalar::one()],
            crossover_commitment,
            crossover_commitment_value: crossover_commitment_value.into(),
            crossover_commitment_blinder: crossover_commitment_blinder.into(),
            obfuscated_commitment_points: vec![obfuscated_commitment_one],
            obfuscated_note_values: vec![obfuscated_note_value_one.into()],
            obfuscated_note_blinders: vec![obfuscated_note_blinder_one.into()],
            fee,
            trim_size: 1 << 16,
            pi_positions: vec![],
        };

        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }
    pub fn compile_execute_circuit_6() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let value1 = 600u64;
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = circuit_note(ssk1, value1, 0, input_note_blinder_one);
        note1.set_pos(0);
        let input_note_value_one = JubJubScalar::from(value1);
        let input_commitment_one = compute_value_commitment(
            input_note_value_one,
            input_note_blinder_one,
        );
        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        let value2 = 200u64;
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        let mut note2 = circuit_note(ssk2, value2, 0, input_note_blinder_two);
        note2.set_pos(1);
        let input_note_value_two = JubJubScalar::from(value2);
        let input_commitment_two = compute_value_commitment(
            input_note_value_two,
            input_note_blinder_two,
        );

        let crossover_commitment_value = JubJubScalar::from(200 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = compute_value_commitment(
            crossover_commitment_value,
            crossover_commitment_blinder,
        );
        let obfuscated_note_value_one = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_one = JubJubScalar::from(100 as u64);
        let obfuscated_commitment_one = compute_value_commitment(
            obfuscated_note_value_one,
            obfuscated_note_blinder_one,
        );
        let obfuscated_note_value_two = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_two = JubJubScalar::from(200 as u64);
        let obfuscated_commitment_two = compute_value_commitment(
            obfuscated_note_value_two,
            obfuscated_note_blinder_two,
        );
        let sig1 = double_schnorr_sign(
            ssk1.sk_r(note1.stealth_address()),
            BlsScalar::one(),
        );
        let sig2 = double_schnorr_sign(
            ssk2.sk_r(note2.stealth_address()),
            BlsScalar::one(),
        );
        let fee = BlsScalar::from(200);

        let mut circuit = ExecuteCircuit {
            nullifiers: vec![
                note1.gen_nullifier(&ssk1),
                note2.gen_nullifier(&ssk2),
            ],
            note_hashes: vec![note1.hash(), note2.hash()],
            position_of_notes: vec![
                BlsScalar::from(note1.pos()),
                BlsScalar::from(note2.pos()),
            ],
            input_note_types: vec![
                BlsScalar::from(note1.note() as u64),
                BlsScalar::from(note2.note() as u64),
            ],
            input_poseidon_branches: vec![
                PoseidonBranch::<17>::default(),
                PoseidonBranch::<17>::default(),
            ],
            input_notes_sk: vec![
                ssk1.sk_r(note1.stealth_address()),
                ssk2.sk_r(note2.stealth_address()),
            ],
            input_notes_pk: vec![
                JubJubAffine::from(note1.stealth_address().pk_r()),
                JubJubAffine::from(note2.stealth_address().pk_r()),
            ],
            input_notes_pk_prime: vec![sig1.3, sig2.3],
            input_commitments: vec![input_commitment_one, input_commitment_two],
            input_nonces: vec![*note1.nonce(), *note2.nonce()],
            input_values: vec![
                input_note_value_one.into(),
                input_note_value_two.into(),
            ],
            input_blinders: vec![
                input_note_blinder_one.into(),
                input_note_blinder_two.into(),
            ],
            input_randomness: vec![
                note1.stealth_address().R().into(),
                note2.stealth_address().R().into(),
            ],
            input_ciphers_one: vec![note1.cipher()[0], note2.cipher()[0]],
            input_ciphers_two: vec![note1.cipher()[1], note2.cipher()[1]],
            input_ciphers_three: vec![note1.cipher()[2], note2.cipher()[2]],
            schnorr_sigs: vec![sig1.0, sig2.0],
            schnorr_r: vec![sig1.1, sig2.1],
            schnorr_r_prime: vec![sig1.2, sig2.2],
            schnorr_messages: vec![BlsScalar::one(), BlsScalar::one()],
            crossover_commitment,
            crossover_commitment_value: crossover_commitment_value.into(),
            crossover_commitment_blinder: crossover_commitment_blinder.into(),
            obfuscated_commitment_points: vec![
                obfuscated_commitment_one,
                obfuscated_commitment_two,
            ],
            obfuscated_note_values: vec![
                obfuscated_note_value_one.into(),
                obfuscated_note_value_two.into(),
            ],
            obfuscated_note_blinders: vec![
                obfuscated_note_blinder_one.into(),
                obfuscated_note_blinder_two.into(),
            ],
            fee,
            trim_size: 1 << 16,
            pi_positions: vec![],
        };

        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }
    pub fn compile_execute_circuit_7() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let value1 = 200u64;
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = circuit_note(ssk1, value1, 0, input_note_blinder_one);
        note1.set_pos(0);
        let input_note_value_one = JubJubScalar::from(value1);
        let input_commitment_one = compute_value_commitment(
            input_note_value_one,
            input_note_blinder_one,
        );
        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        let value2 = 200u64;
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        let mut note2 = circuit_note(ssk2, value2, 0, input_note_blinder_two);
        note2.set_pos(1);
        let input_note_value_two = JubJubScalar::from(value2);
        let input_commitment_two = compute_value_commitment(
            input_note_value_two,
            input_note_blinder_two,
        );
        let secret5 = JubJubScalar::from(500 as u64);
        let secret6 = JubJubScalar::from(600 as u64);
        let ssk3 = SecretSpendKey::new(secret5, secret6);
        let value3 = 100u64;
        let input_note_blinder_three = JubJubScalar::from(165 as u64);
        let note3 = circuit_note(ssk3, value3, 0, input_note_blinder_three);
        note1.set_pos(2);
        let input_note_value_three = JubJubScalar::from(value3);
        let input_commitment_three = compute_value_commitment(
            input_note_value_three,
            input_note_blinder_three,
        );
        let crossover_commitment_value = JubJubScalar::from(300 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = compute_value_commitment(
            crossover_commitment_value,
            crossover_commitment_blinder,
        );

        let sig1 = double_schnorr_sign(
            ssk1.sk_r(note1.stealth_address()),
            BlsScalar::one(),
        );
        let sig2 = double_schnorr_sign(
            ssk2.sk_r(note2.stealth_address()),
            BlsScalar::one(),
        );
        let sig3 = double_schnorr_sign(
            ssk3.sk_r(note3.stealth_address()),
            BlsScalar::one(),
        );
        let fee = BlsScalar::from(200);

        let mut circuit = ExecuteCircuit {
            nullifiers: vec![
                note1.gen_nullifier(&ssk1),
                note2.gen_nullifier(&ssk2),
                note3.gen_nullifier(&ssk3),
            ],
            note_hashes: vec![note1.hash(), note2.hash(), note3.hash()],
            position_of_notes: vec![
                BlsScalar::from(note1.pos()),
                BlsScalar::from(note2.pos()),
                BlsScalar::from(note3.pos()),
            ],
            input_note_types: vec![
                BlsScalar::from(note1.note() as u64),
                BlsScalar::from(note2.note() as u64),
                BlsScalar::from(note3.note() as u64),
            ],
            input_poseidon_branches: vec![
                PoseidonBranch::<17>::default(),
                PoseidonBranch::<17>::default(),
                PoseidonBranch::<17>::default(),
            ],
            input_notes_sk: vec![
                ssk1.sk_r(note1.stealth_address()),
                ssk2.sk_r(note2.stealth_address()),
                ssk3.sk_r(note2.stealth_address()),
            ],
            input_notes_pk: vec![
                JubJubAffine::from(note1.stealth_address().pk_r()),
                JubJubAffine::from(note2.stealth_address().pk_r()),
                JubJubAffine::from(note3.stealth_address().pk_r()),
            ],
            input_notes_pk_prime: vec![sig1.3, sig2.3, sig3.3],
            input_commitments: vec![
                input_commitment_one,
                input_commitment_two,
                input_commitment_three,
            ],
            input_nonces: vec![*note1.nonce(), *note2.nonce() * note3.nonce()],
            input_values: vec![
                input_note_value_one.into(),
                input_note_value_two.into(),
                input_note_value_three.into(),
            ],
            input_blinders: vec![
                input_note_blinder_one.into(),
                input_note_blinder_two.into(),
                input_note_blinder_three.into(),
            ],
            input_randomness: vec![
                note1.stealth_address().R().into(),
                note2.stealth_address().R().into(),
                note3.stealth_address().R().into(),
            ],
            input_ciphers_one: vec![
                note1.cipher()[0],
                note2.cipher()[0],
                note3.cipher()[0],
            ],
            input_ciphers_two: vec![
                note1.cipher()[1],
                note2.cipher()[1],
                note3.cipher()[1],
            ],
            input_ciphers_three: vec![
                note1.cipher()[2],
                note2.cipher()[2],
                note3.cipher()[2],
            ],
            schnorr_sigs: vec![sig1.0, sig2.0, sig3.0],
            schnorr_r: vec![sig1.1, sig2.1, sig3.1],
            schnorr_r_prime: vec![sig1.2, sig2.2, sig3.2],
            schnorr_messages: vec![
                BlsScalar::one(),
                BlsScalar::one(),
                BlsScalar::one(),
            ],
            crossover_commitment,
            crossover_commitment_value: crossover_commitment_value.into(),
            crossover_commitment_blinder: crossover_commitment_blinder.into(),
            obfuscated_commitment_points: vec![],
            obfuscated_note_values: vec![],
            obfuscated_note_blinders: vec![],
            fee,
            trim_size: 1 << 17,
            pi_positions: vec![],
        };

        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }
    pub fn compile_execute_circuit_8() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let value1 = 600u64;
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = circuit_note(ssk1, value1, 0, input_note_blinder_one);
        note1.set_pos(0);
        let input_note_value_one = JubJubScalar::from(value1);
        let input_commitment_one = compute_value_commitment(
            input_note_value_one,
            input_note_blinder_one,
        );
        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        let value2 = 200u64;
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        let mut note2 = circuit_note(ssk2, value2, 0, input_note_blinder_two);
        note2.set_pos(1);
        let input_note_value_two = JubJubScalar::from(value2);
        let input_commitment_two = compute_value_commitment(
            input_note_value_two,
            input_note_blinder_two,
        );
        let secret5 = JubJubScalar::from(500 as u64);
        let secret6 = JubJubScalar::from(600 as u64);
        let ssk3 = SecretSpendKey::new(secret5, secret6);
        let value3 = 100u64;
        let input_note_blinder_three = JubJubScalar::from(165 as u64);
        let mut note3 = circuit_note(ssk3, value3, 0, input_note_blinder_three);
        note3.set_pos(2);
        let input_note_value_three = JubJubScalar::from(value3);
        let input_commitment_three = compute_value_commitment(
            input_note_value_three,
            input_note_blinder_three,
        );
        let crossover_commitment_value = JubJubScalar::from(300 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = compute_value_commitment(
            crossover_commitment_value,
            crossover_commitment_blinder,
        );
        let obfuscated_note_value_one = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_one = JubJubScalar::from(100 as u64);
        let obfuscated_commitment_one = compute_value_commitment(
            obfuscated_note_value_one,
            obfuscated_note_blinder_one,
        );

        let sig1 = double_schnorr_sign(
            ssk1.sk_r(note1.stealth_address()),
            BlsScalar::one(),
        );
        let sig2 = double_schnorr_sign(
            ssk2.sk_r(note2.stealth_address()),
            BlsScalar::one(),
        );
        let sig3 = double_schnorr_sign(
            ssk3.sk_r(note3.stealth_address()),
            BlsScalar::one(),
        );
        let fee = BlsScalar::from(400);

        let mut circuit = ExecuteCircuit {
            nullifiers: vec![
                note1.gen_nullifier(&ssk1),
                note2.gen_nullifier(&ssk2),
                note3.gen_nullifier(&ssk3),
            ],
            note_hashes: vec![note1.hash(), note2.hash(), note3.hash()],
            position_of_notes: vec![
                BlsScalar::from(note1.pos()),
                BlsScalar::from(note2.pos()),
                BlsScalar::from(note3.pos()),
            ],
            input_note_types: vec![
                BlsScalar::from(note1.note() as u64),
                BlsScalar::from(note2.note() as u64),
                BlsScalar::from(note3.note() as u64),
            ],
            input_poseidon_branches: vec![
                PoseidonBranch::<17>::default(),
                PoseidonBranch::<17>::default(),
                PoseidonBranch::<17>::default(),
            ],
            input_notes_sk: vec![
                ssk1.sk_r(note1.stealth_address()),
                ssk2.sk_r(note2.stealth_address()),
                ssk3.sk_r(note2.stealth_address()),
            ],
            input_notes_pk: vec![
                JubJubAffine::from(note1.stealth_address().pk_r()),
                JubJubAffine::from(note2.stealth_address().pk_r()),
                JubJubAffine::from(note3.stealth_address().pk_r()),
            ],
            input_notes_pk_prime: vec![sig1.3, sig2.3, sig3.3],
            input_commitments: vec![
                input_commitment_one,
                input_commitment_two,
                input_commitment_three,
            ],
            input_nonces: vec![*note1.nonce(), *note2.nonce() * note3.nonce()],
            input_values: vec![
                input_note_value_one.into(),
                input_note_value_two.into(),
                input_note_value_three.into(),
            ],
            input_blinders: vec![
                input_note_blinder_one.into(),
                input_note_blinder_two.into(),
                input_note_blinder_three.into(),
            ],
            input_randomness: vec![
                note1.stealth_address().R().into(),
                note2.stealth_address().R().into(),
                note3.stealth_address().R().into(),
            ],
            input_ciphers_one: vec![
                note1.cipher()[0],
                note2.cipher()[0],
                note3.cipher()[0],
            ],
            input_ciphers_two: vec![
                note1.cipher()[1],
                note2.cipher()[1],
                note3.cipher()[1],
            ],
            input_ciphers_three: vec![
                note1.cipher()[2],
                note2.cipher()[2],
                note3.cipher()[2],
            ],
            schnorr_sigs: vec![sig1.0, sig2.0, sig3.0],
            schnorr_r: vec![sig1.1, sig2.1, sig3.1],
            schnorr_r_prime: vec![sig1.2, sig2.2, sig3.2],
            schnorr_messages: vec![
                BlsScalar::one(),
                BlsScalar::one(),
                BlsScalar::one(),
            ],
            crossover_commitment,
            crossover_commitment_value: crossover_commitment_value.into(),
            crossover_commitment_blinder: crossover_commitment_blinder.into(),
            obfuscated_commitment_points: vec![obfuscated_commitment_one],
            obfuscated_note_values: vec![obfuscated_note_value_one.into()],
            obfuscated_note_blinders: vec![obfuscated_note_blinder_one.into()],
            fee,
            trim_size: 1 << 17,
            pi_positions: vec![],
        };

        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }
    pub fn compile_execute_circuit_9() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let value1 = 600u64;
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = circuit_note(ssk1, value1, 0, input_note_blinder_one);
        note1.set_pos(0);
        let input_note_value_one = JubJubScalar::from(value1);
        let input_commitment_one = compute_value_commitment(
            input_note_value_one,
            input_note_blinder_one,
        );
        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        let value2 = 200u64;
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        let mut note2 = circuit_note(ssk2, value2, 0, input_note_blinder_two);
        note2.set_pos(1);
        let input_note_value_two = JubJubScalar::from(value2);
        let input_commitment_two = compute_value_commitment(
            input_note_value_two,
            input_note_blinder_two,
        );
        let secret5 = JubJubScalar::from(500 as u64);
        let secret6 = JubJubScalar::from(600 as u64);
        let ssk3 = SecretSpendKey::new(secret5, secret6);
        let value3 = 100u64;
        let input_note_blinder_three = JubJubScalar::from(165 as u64);
        let mut note3 = circuit_note(ssk3, value3, 0, input_note_blinder_three);
        note3.set_pos(2);
        let input_note_value_three = JubJubScalar::from(value3);
        let input_commitment_three = compute_value_commitment(
            input_note_value_three,
            input_note_blinder_three,
        );
        let crossover_commitment_value = JubJubScalar::from(300 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = compute_value_commitment(
            crossover_commitment_value,
            crossover_commitment_blinder,
        );
        let obfuscated_note_value_one = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_one = JubJubScalar::from(100 as u64);
        let obfuscated_commitment_one = compute_value_commitment(
            obfuscated_note_value_one,
            obfuscated_note_blinder_one,
        );
        let obfuscated_note_value_two = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_two = JubJubScalar::from(200 as u64);
        let obfuscated_commitment_two = compute_value_commitment(
            obfuscated_note_value_two,
            obfuscated_note_blinder_two,
        );
        let sig1 = double_schnorr_sign(
            ssk1.sk_r(note1.stealth_address()),
            BlsScalar::one(),
        );
        let sig2 = double_schnorr_sign(
            ssk2.sk_r(note2.stealth_address()),
            BlsScalar::one(),
        );
        let sig3 = double_schnorr_sign(
            ssk3.sk_r(note3.stealth_address()),
            BlsScalar::one(),
        );
        let fee = BlsScalar::from(200);

        let mut circuit = ExecuteCircuit {
            nullifiers: vec![
                note1.gen_nullifier(&ssk1),
                note2.gen_nullifier(&ssk2),
                note3.gen_nullifier(&ssk3),
            ],
            note_hashes: vec![note1.hash(), note2.hash(), note3.hash()],
            position_of_notes: vec![
                BlsScalar::from(note1.pos()),
                BlsScalar::from(note2.pos()),
                BlsScalar::from(note3.pos()),
            ],
            input_note_types: vec![
                BlsScalar::from(note1.note() as u64),
                BlsScalar::from(note2.note() as u64),
                BlsScalar::from(note3.note() as u64),
            ],
            input_poseidon_branches: vec![
                PoseidonBranch::<17>::default(),
                PoseidonBranch::<17>::default(),
                PoseidonBranch::<17>::default(),
            ],
            input_notes_sk: vec![
                ssk1.sk_r(note1.stealth_address()),
                ssk2.sk_r(note2.stealth_address()),
                ssk3.sk_r(note2.stealth_address()),
            ],
            input_notes_pk: vec![
                JubJubAffine::from(note1.stealth_address().pk_r()),
                JubJubAffine::from(note2.stealth_address().pk_r()),
                JubJubAffine::from(note3.stealth_address().pk_r()),
            ],
            input_notes_pk_prime: vec![sig1.3, sig2.3, sig3.3],
            input_commitments: vec![
                input_commitment_one,
                input_commitment_two,
                input_commitment_three,
            ],
            input_nonces: vec![*note1.nonce(), *note2.nonce() * note3.nonce()],
            input_values: vec![
                input_note_value_one.into(),
                input_note_value_two.into(),
                input_note_value_three.into(),
            ],
            input_blinders: vec![
                input_note_blinder_one.into(),
                input_note_blinder_two.into(),
                input_note_blinder_three.into(),
            ],
            input_randomness: vec![
                note1.stealth_address().R().into(),
                note2.stealth_address().R().into(),
                note3.stealth_address().R().into(),
            ],
            input_ciphers_one: vec![
                note1.cipher()[0],
                note2.cipher()[0],
                note3.cipher()[0],
            ],
            input_ciphers_two: vec![
                note1.cipher()[1],
                note2.cipher()[1],
                note3.cipher()[1],
            ],
            input_ciphers_three: vec![
                note1.cipher()[2],
                note2.cipher()[2],
                note3.cipher()[2],
            ],
            schnorr_sigs: vec![sig1.0, sig2.0, sig3.0],
            schnorr_r: vec![sig1.1, sig2.1, sig3.1],
            schnorr_r_prime: vec![sig1.2, sig2.2, sig3.2],
            schnorr_messages: vec![
                BlsScalar::one(),
                BlsScalar::one(),
                BlsScalar::one(),
            ],
            crossover_commitment,
            crossover_commitment_value: crossover_commitment_value.into(),
            crossover_commitment_blinder: crossover_commitment_blinder.into(),
            obfuscated_commitment_points: vec![
                obfuscated_commitment_one,
                obfuscated_commitment_two,
            ],
            obfuscated_note_values: vec![
                obfuscated_note_value_one.into(),
                obfuscated_note_value_two.into(),
            ],
            obfuscated_note_blinders: vec![
                obfuscated_note_blinder_one.into(),
                obfuscated_note_blinder_two.into(),
            ],
            fee,
            trim_size: 1 << 17,
            pi_positions: vec![],
        };

        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }
    pub fn compile_execute_circuit_10() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let value1 = 600u64;
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = circuit_note(ssk1, value1, 0, input_note_blinder_one);
        note1.set_pos(0);
        let input_note_value_one = JubJubScalar::from(value1);
        let input_commitment_one = compute_value_commitment(
            input_note_value_one,
            input_note_blinder_one,
        );
        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        let value2 = 200u64;
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        let mut note2 = circuit_note(ssk2, value2, 0, input_note_blinder_two);
        note2.set_pos(1);
        let input_note_value_two = JubJubScalar::from(value2);
        let input_commitment_two = compute_value_commitment(
            input_note_value_two,
            input_note_blinder_two,
        );
        let secret5 = JubJubScalar::from(500 as u64);
        let secret6 = JubJubScalar::from(600 as u64);
        let ssk3 = SecretSpendKey::new(secret5, secret6);
        let value3 = 100u64;
        let input_note_blinder_three = JubJubScalar::from(165 as u64);
        let mut note3 = circuit_note(ssk3, value3, 0, input_note_blinder_three);
        note3.set_pos(2);
        let input_note_value_three = JubJubScalar::from(value3);
        let input_commitment_three = compute_value_commitment(
            input_note_value_three,
            input_note_blinder_three,
        );
        let secret7 = JubJubScalar::from(400 as u64);
        let secret8 = JubJubScalar::from(220 as u64);
        let ssk4 = SecretSpendKey::new(secret7, secret8);
        let value4 = 200u64;
        let input_note_blinder_four = JubJubScalar::from(100 as u64);
        let mut note4 = circuit_note(ssk4, value4, 0, input_note_blinder_four);
        note4.set_pos(3);
        let input_note_value_four = JubJubScalar::from(value4);
        let input_commitment_four = compute_value_commitment(
            input_note_value_four,
            input_note_blinder_four,
        );
        let crossover_commitment_value = JubJubScalar::from(300 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = compute_value_commitment(
            crossover_commitment_value,
            crossover_commitment_blinder,
        );

        let sig1 = double_schnorr_sign(
            ssk1.sk_r(note1.stealth_address()),
            BlsScalar::one(),
        );
        let sig2 = double_schnorr_sign(
            ssk2.sk_r(note2.stealth_address()),
            BlsScalar::one(),
        );
        let sig3 = double_schnorr_sign(
            ssk3.sk_r(note3.stealth_address()),
            BlsScalar::one(),
        );
        let sig4 = double_schnorr_sign(
            ssk3.sk_r(note4.stealth_address()),
            BlsScalar::one(),
        );
        let fee = BlsScalar::from(800);

        let mut circuit = ExecuteCircuit {
            nullifiers: vec![
                note1.gen_nullifier(&ssk1),
                note2.gen_nullifier(&ssk2),
                note3.gen_nullifier(&ssk3),
                note4.gen_nullifier(&ssk4),
            ],
            note_hashes: vec![
                note1.hash(),
                note2.hash(),
                note3.hash(),
                note4.hash(),
            ],
            position_of_notes: vec![
                BlsScalar::from(note1.pos()),
                BlsScalar::from(note2.pos()),
                BlsScalar::from(note3.pos()),
                BlsScalar::from(note4.pos()),
            ],
            input_note_types: vec![
                BlsScalar::from(note1.note() as u64),
                BlsScalar::from(note2.note() as u64),
                BlsScalar::from(note3.note() as u64),
                BlsScalar::from(note4.note() as u64),
            ],
            input_poseidon_branches: vec![
                PoseidonBranch::<17>::default(),
                PoseidonBranch::<17>::default(),
                PoseidonBranch::<17>::default(),
                PoseidonBranch::<17>::default(),
            ],
            input_notes_sk: vec![
                ssk1.sk_r(note1.stealth_address()),
                ssk2.sk_r(note2.stealth_address()),
                ssk3.sk_r(note2.stealth_address()),
                ssk4.sk_r(note4.stealth_address()),
            ],
            input_notes_pk: vec![
                JubJubAffine::from(note1.stealth_address().pk_r()),
                JubJubAffine::from(note2.stealth_address().pk_r()),
                JubJubAffine::from(note3.stealth_address().pk_r()),
                JubJubAffine::from(note4.stealth_address().pk_r()),
            ],
            input_notes_pk_prime: vec![sig1.3, sig2.3, sig3.3, sig4.3],
            input_commitments: vec![
                input_commitment_one,
                input_commitment_two,
                input_commitment_three,
                input_commitment_four,
            ],
            input_nonces: vec![
                *note1.nonce(),
                *note2.nonce(),
                *note3.nonce(),
                *note4.nonce(),
            ],
            input_values: vec![
                input_note_value_one.into(),
                input_note_value_two.into(),
                input_note_value_three.into(),
                input_note_value_four.into(),
            ],
            input_blinders: vec![
                input_note_blinder_one.into(),
                input_note_blinder_two.into(),
                input_note_blinder_three.into(),
                input_note_blinder_four.into(),
            ],
            input_randomness: vec![
                note1.stealth_address().R().into(),
                note2.stealth_address().R().into(),
                note3.stealth_address().R().into(),
                note4.stealth_address().R().into(),
            ],
            input_ciphers_one: vec![
                note1.cipher()[0],
                note2.cipher()[0],
                note3.cipher()[0],
                note4.cipher()[0],
            ],
            input_ciphers_two: vec![
                note1.cipher()[1],
                note2.cipher()[1],
                note3.cipher()[1],
                note4.cipher()[1],
            ],
            input_ciphers_three: vec![
                note1.cipher()[2],
                note2.cipher()[2],
                note3.cipher()[2],
                note4.cipher()[2],
            ],
            schnorr_sigs: vec![sig1.0, sig2.0, sig3.0, sig4.0],
            schnorr_r: vec![sig1.1, sig2.1, sig3.1, sig4.1],
            schnorr_r_prime: vec![sig1.2, sig2.2, sig3.2, sig4.2],
            schnorr_messages: vec![
                BlsScalar::one(),
                BlsScalar::one(),
                BlsScalar::one(),
                BlsScalar::one(),
            ],
            crossover_commitment,
            crossover_commitment_value: crossover_commitment_value.into(),
            crossover_commitment_blinder: crossover_commitment_blinder.into(),
            obfuscated_commitment_points: vec![],
            obfuscated_note_values: vec![],
            obfuscated_note_blinders: vec![],
            fee,
            trim_size: 1 << 17,
            pi_positions: vec![],
        };

        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }
    pub fn compile_execute_circuit_11() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let value1 = 300u64;
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = circuit_note(ssk1, value1, 0, input_note_blinder_one);
        note1.set_pos(0);
        let input_note_value_one = JubJubScalar::from(value1);
        let input_commitment_one = compute_value_commitment(
            input_note_value_one,
            input_note_blinder_one,
        );
        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        let value2 = 200u64;
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        let mut note2 = circuit_note(ssk2, value2, 0, input_note_blinder_two);
        note2.set_pos(1);
        let input_note_value_two = JubJubScalar::from(value2);
        let input_commitment_two = compute_value_commitment(
            input_note_value_two,
            input_note_blinder_two,
        );
        let secret5 = JubJubScalar::from(500 as u64);
        let secret6 = JubJubScalar::from(600 as u64);
        let ssk3 = SecretSpendKey::new(secret5, secret6);
        let value3 = 100u64;
        let input_note_blinder_three = JubJubScalar::from(165 as u64);
        let mut note3 = circuit_note(ssk3, value3, 0, input_note_blinder_three);
        note3.set_pos(2);
        let input_note_value_three = JubJubScalar::from(value3);
        let input_commitment_three = compute_value_commitment(
            input_note_value_three,
            input_note_blinder_three,
        );
        let secret7 = JubJubScalar::from(400 as u64);
        let secret8 = JubJubScalar::from(220 as u64);
        let ssk4 = SecretSpendKey::new(secret7, secret8);
        let value4 = 200u64;
        let input_note_blinder_four = JubJubScalar::from(100 as u64);
        let mut note4 = circuit_note(ssk4, value4, 0, input_note_blinder_four);
        note4.set_pos(3);
        let input_note_value_four = JubJubScalar::from(value4);
        let input_commitment_four = compute_value_commitment(
            input_note_value_four,
            input_note_blinder_four,
        );
        let crossover_commitment_value = JubJubScalar::from(300 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = compute_value_commitment(
            crossover_commitment_value,
            crossover_commitment_blinder,
        );
        let obfuscated_note_value_one = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_one = JubJubScalar::from(100 as u64);
        let obfuscated_commitment_one = compute_value_commitment(
            obfuscated_note_value_one,
            obfuscated_note_blinder_one,
        );

        let sig1 = double_schnorr_sign(
            ssk1.sk_r(note1.stealth_address()),
            BlsScalar::one(),
        );
        let sig2 = double_schnorr_sign(
            ssk2.sk_r(note2.stealth_address()),
            BlsScalar::one(),
        );
        let sig3 = double_schnorr_sign(
            ssk3.sk_r(note3.stealth_address()),
            BlsScalar::one(),
        );
        let sig4 = double_schnorr_sign(
            ssk3.sk_r(note4.stealth_address()),
            BlsScalar::one(),
        );
        let fee = BlsScalar::from(300);

        let mut circuit = ExecuteCircuit {
            nullifiers: vec![
                note1.gen_nullifier(&ssk1),
                note2.gen_nullifier(&ssk2),
                note3.gen_nullifier(&ssk3),
                note4.gen_nullifier(&ssk4),
            ],
            note_hashes: vec![
                note1.hash(),
                note2.hash(),
                note3.hash(),
                note4.hash(),
            ],
            position_of_notes: vec![
                BlsScalar::from(note1.pos()),
                BlsScalar::from(note2.pos()),
                BlsScalar::from(note3.pos()),
                BlsScalar::from(note4.pos()),
            ],
            input_note_types: vec![
                BlsScalar::from(note1.note() as u64),
                BlsScalar::from(note2.note() as u64),
                BlsScalar::from(note3.note() as u64),
                BlsScalar::from(note4.note() as u64),
            ],
            input_poseidon_branches: vec![
                PoseidonBranch::<17>::default(),
                PoseidonBranch::<17>::default(),
                PoseidonBranch::<17>::default(),
                PoseidonBranch::<17>::default(),
            ],
            input_notes_sk: vec![
                ssk1.sk_r(note1.stealth_address()),
                ssk2.sk_r(note2.stealth_address()),
                ssk3.sk_r(note2.stealth_address()),
                ssk4.sk_r(note4.stealth_address()),
            ],
            input_notes_pk: vec![
                JubJubAffine::from(note1.stealth_address().pk_r()),
                JubJubAffine::from(note2.stealth_address().pk_r()),
                JubJubAffine::from(note3.stealth_address().pk_r()),
                JubJubAffine::from(note4.stealth_address().pk_r()),
            ],
            input_notes_pk_prime: vec![sig1.3, sig2.3, sig3.3, sig4.3],
            input_commitments: vec![
                input_commitment_one,
                input_commitment_two,
                input_commitment_three,
                input_commitment_four,
            ],
            input_nonces: vec![
                *note1.nonce(),
                *note2.nonce(),
                *note3.nonce(),
                *note4.nonce(),
            ],
            input_values: vec![
                input_note_value_one.into(),
                input_note_value_two.into(),
                input_note_value_three.into(),
                input_note_value_four.into(),
            ],
            input_blinders: vec![
                input_note_blinder_one.into(),
                input_note_blinder_two.into(),
                input_note_blinder_three.into(),
                input_note_blinder_four.into(),
            ],
            input_randomness: vec![
                note1.stealth_address().R().into(),
                note2.stealth_address().R().into(),
                note3.stealth_address().R().into(),
                note4.stealth_address().R().into(),
            ],
            input_ciphers_one: vec![
                note1.cipher()[0],
                note2.cipher()[0],
                note3.cipher()[0],
                note4.cipher()[0],
            ],
            input_ciphers_two: vec![
                note1.cipher()[1],
                note2.cipher()[1],
                note3.cipher()[1],
                note4.cipher()[1],
            ],
            input_ciphers_three: vec![
                note1.cipher()[2],
                note2.cipher()[2],
                note3.cipher()[2],
                note4.cipher()[2],
            ],
            schnorr_sigs: vec![sig1.0, sig2.0, sig3.0, sig4.0],
            schnorr_r: vec![sig1.1, sig2.1, sig3.1, sig4.1],
            schnorr_r_prime: vec![sig1.2, sig2.2, sig3.2, sig4.2],
            schnorr_messages: vec![
                BlsScalar::one(),
                BlsScalar::one(),
                BlsScalar::one(),
                BlsScalar::one(),
            ],
            crossover_commitment,
            crossover_commitment_value: crossover_commitment_value.into(),
            crossover_commitment_blinder: crossover_commitment_blinder.into(),
            obfuscated_commitment_points: vec![obfuscated_commitment_one],
            obfuscated_note_values: vec![obfuscated_note_value_one.into()],
            obfuscated_note_blinders: vec![obfuscated_note_blinder_one.into()],
            fee,
            trim_size: 1 << 17,
            pi_positions: vec![],
        };

        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }
    pub fn compile_execute_circuit_12() -> Result<(Vec<u8>, Vec<u8>)> {
        let pub_params = &PUB_PARAMS;
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let value1 = 600u64;
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = circuit_note(ssk1, value1, 0, input_note_blinder_one);
        note1.set_pos(0);
        let input_note_value_one = JubJubScalar::from(value1);
        let input_commitment_one = compute_value_commitment(
            input_note_value_one,
            input_note_blinder_one,
        );
        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        let value2 = 200u64;
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        let mut note2 = circuit_note(ssk2, value2, 0, input_note_blinder_two);
        note2.set_pos(1);
        let input_note_value_two = JubJubScalar::from(value2);
        let input_commitment_two = compute_value_commitment(
            input_note_value_two,
            input_note_blinder_two,
        );
        let secret5 = JubJubScalar::from(500 as u64);
        let secret6 = JubJubScalar::from(600 as u64);
        let ssk3 = SecretSpendKey::new(secret5, secret6);
        let value3 = 100u64;
        let input_note_blinder_three = JubJubScalar::from(165 as u64);
        let mut note3 = circuit_note(ssk3, value3, 0, input_note_blinder_three);
        note3.set_pos(2);
        let input_note_value_three = JubJubScalar::from(value3);
        let input_commitment_three = compute_value_commitment(
            input_note_value_three,
            input_note_blinder_three,
        );
        let secret7 = JubJubScalar::from(400 as u64);
        let secret8 = JubJubScalar::from(220 as u64);
        let ssk4 = SecretSpendKey::new(secret7, secret8);
        let value4 = 200u64;
        let input_note_blinder_four = JubJubScalar::from(100 as u64);
        let mut note4 = circuit_note(ssk4, value4, 0, input_note_blinder_four);
        note4.set_pos(3);
        let input_note_value_four = JubJubScalar::from(value4);
        let input_commitment_four = compute_value_commitment(
            input_note_value_four,
            input_note_blinder_four,
        );
        let crossover_commitment_value = JubJubScalar::from(300 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = compute_value_commitment(
            crossover_commitment_value,
            crossover_commitment_blinder,
        );
        let obfuscated_note_value_one = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_one = JubJubScalar::from(100 as u64);
        let obfuscated_commitment_one = compute_value_commitment(
            obfuscated_note_value_one,
            obfuscated_note_blinder_one,
        );
        let obfuscated_note_value_two = JubJubScalar::from(300 as u64);
        let obfuscated_note_blinder_two = JubJubScalar::from(200 as u64);
        let obfuscated_commitment_two = compute_value_commitment(
            obfuscated_note_value_two,
            obfuscated_note_blinder_two,
        );
        let sig1 = double_schnorr_sign(
            ssk1.sk_r(note1.stealth_address()),
            BlsScalar::one(),
        );
        let sig2 = double_schnorr_sign(
            ssk2.sk_r(note2.stealth_address()),
            BlsScalar::one(),
        );
        let sig3 = double_schnorr_sign(
            ssk3.sk_r(note3.stealth_address()),
            BlsScalar::one(),
        );
        let sig4 = double_schnorr_sign(
            ssk3.sk_r(note4.stealth_address()),
            BlsScalar::one(),
        );
        let fee = BlsScalar::from(300);

        let mut circuit = ExecuteCircuit {
            nullifiers: vec![
                note1.gen_nullifier(&ssk1),
                note2.gen_nullifier(&ssk2),
                note3.gen_nullifier(&ssk3),
                note4.gen_nullifier(&ssk4),
            ],
            note_hashes: vec![
                note1.hash(),
                note2.hash(),
                note3.hash(),
                note4.hash(),
            ],
            position_of_notes: vec![
                BlsScalar::from(note1.pos()),
                BlsScalar::from(note2.pos()),
                BlsScalar::from(note3.pos()),
                BlsScalar::from(note4.pos()),
            ],
            input_note_types: vec![
                BlsScalar::from(note1.note() as u64),
                BlsScalar::from(note2.note() as u64),
                BlsScalar::from(note3.note() as u64),
                BlsScalar::from(note4.note() as u64),
            ],
            input_poseidon_branches: vec![
                PoseidonBranch::<17>::default(),
                PoseidonBranch::<17>::default(),
                PoseidonBranch::<17>::default(),
                PoseidonBranch::<17>::default(),
            ],
            input_notes_sk: vec![
                ssk1.sk_r(note1.stealth_address()),
                ssk2.sk_r(note2.stealth_address()),
                ssk3.sk_r(note2.stealth_address()),
                ssk4.sk_r(note4.stealth_address()),
            ],
            input_notes_pk: vec![
                JubJubAffine::from(note1.stealth_address().pk_r()),
                JubJubAffine::from(note2.stealth_address().pk_r()),
                JubJubAffine::from(note3.stealth_address().pk_r()),
                JubJubAffine::from(note4.stealth_address().pk_r()),
            ],
            input_notes_pk_prime: vec![sig1.3, sig2.3, sig3.3, sig4.3],
            input_commitments: vec![
                input_commitment_one,
                input_commitment_two,
                input_commitment_three,
                input_commitment_four,
            ],
            input_nonces: vec![
                *note1.nonce(),
                *note2.nonce(),
                *note3.nonce(),
                *note4.nonce(),
            ],
            input_values: vec![
                input_note_value_one.into(),
                input_note_value_two.into(),
                input_note_value_three.into(),
                input_note_value_four.into(),
            ],
            input_blinders: vec![
                input_note_blinder_one.into(),
                input_note_blinder_two.into(),
                input_note_blinder_three.into(),
                input_note_blinder_four.into(),
            ],
            input_randomness: vec![
                note1.stealth_address().R().into(),
                note2.stealth_address().R().into(),
                note3.stealth_address().R().into(),
                note4.stealth_address().R().into(),
            ],
            input_ciphers_one: vec![
                note1.cipher()[0],
                note2.cipher()[0],
                note3.cipher()[0],
                note4.cipher()[0],
            ],
            input_ciphers_two: vec![
                note1.cipher()[1],
                note2.cipher()[1],
                note3.cipher()[1],
                note4.cipher()[1],
            ],
            input_ciphers_three: vec![
                note1.cipher()[2],
                note2.cipher()[2],
                note3.cipher()[2],
                note4.cipher()[2],
            ],
            schnorr_sigs: vec![sig1.0, sig2.0, sig3.0, sig4.0],
            schnorr_r: vec![sig1.1, sig2.1, sig3.1, sig4.1],
            schnorr_r_prime: vec![sig1.2, sig2.2, sig3.2, sig4.2],
            schnorr_messages: vec![
                BlsScalar::one(),
                BlsScalar::one(),
                BlsScalar::one(),
                BlsScalar::one(),
            ],
            crossover_commitment,
            crossover_commitment_value: crossover_commitment_value.into(),
            crossover_commitment_blinder: crossover_commitment_blinder.into(),
            obfuscated_commitment_points: vec![
                obfuscated_commitment_one,
                obfuscated_commitment_two,
            ],
            obfuscated_note_values: vec![
                obfuscated_note_value_one.into(),
                obfuscated_note_value_two.into(),
            ],
            obfuscated_note_blinders: vec![
                obfuscated_note_blinder_one.into(),
                obfuscated_note_blinder_two.into(),
            ],
            fee,
            trim_size: 1 << 17,
            pi_positions: vec![],
        };

        let (pk, vk) = circuit.compile(&pub_params)?;
        Ok((pk.to_bytes(), vk.to_bytes()))
    }
}

mod leaf {
    use canonical::{Canon, Store};
    use canonical_derive::Canon;
    use dusk_plonk::bls12_381::BlsScalar;
    use phoenix_core::Note;
    use poseidon252::tree::PoseidonLeaf;

    #[derive(Debug, Clone, Canon)]
    pub(crate) struct NoteLeaf(Note);

    impl From<Note> for NoteLeaf {
        fn from(note: Note) -> NoteLeaf {
            NoteLeaf(note)
        }
    }

    impl From<NoteLeaf> for Note {
        fn from(leaf: NoteLeaf) -> Note {
            leaf.0
        }
    }

    impl<S> PoseidonLeaf<S> for NoteLeaf
    where
        S: Store,
    {
        fn poseidon_hash(&self) -> BlsScalar {
            self.0.hash()
        }

        fn pos(&self) -> u64 {
            self.0.pos()
        }

        fn set_pos(&mut self, pos: u64) {
            self.0.set_pos(pos)
        }
    }
}
