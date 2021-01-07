// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::SendToContractTransparentCircuit;

use dusk_pki::{Ownable, SecretSpendKey};
use dusk_plonk::jubjub::GENERATOR_EXTENDED;
use phoenix_core::Note;
use poseidon252::sponge;
use rand::rngs::StdRng;
use rand::SeedableRng;
use schnorr::single_key::SecretKey as SchnorrSecret;

use dusk_plonk::prelude::*;

use std::convert::TryInto;

#[test]
fn send_transparent() {
    let mut rng = StdRng::seed_from_u64(2322u64);

    let c_ssk = SecretSpendKey::random(&mut rng);
    let c_psk = c_ssk.public_key();

    let c_value = 100;
    let c_blinding_factor = JubJubScalar::random(&mut rng);

    let c_note = Note::obfuscated(&mut rng, &c_psk, c_value, c_blinding_factor);
    let c_sk_r = c_ssk.sk_r(c_note.stealth_address());
    let c_pk_r = GENERATOR_EXTENDED * c_sk_r;

    let (_, crossover) = c_note
        .try_into()
        .expect("Failed to generate fee and crossover!");
    let c_value_commitment = *crossover.value_commitment();

    let c_schnorr_secret = SchnorrSecret::from(c_sk_r);
    let c_commitment_hash = sponge::hash(&c_value_commitment.to_hash_inputs());
    let c_signature = c_schnorr_secret.sign(&mut rng, c_commitment_hash);

    let mut circuit = SendToContractTransparentCircuit::new(
        c_value_commitment,
        c_pk_r,
        c_value,
        c_blinding_factor,
        c_signature,
    );

    // Generate Composer & Public Parameters
    let pp = PublicParameters::setup(
        circuit.get_trim_size(),
        &mut rand::thread_rng(),
    )
    .expect("Failed to generate public parameters");

    let (pk, vk) = circuit.compile(&pp).expect("Failed to compile circuit");
    circuit.get_mut_pi_positions().clear();

    let proof = circuit
        .gen_proof(&pp, &pk, b"send-transparent")
        .expect("Failed to generate proof");
    let pi = circuit.get_pi_positions().clone();

    let verify = circuit
        .verify_proof(&pp, &vk, b"send-transparent", &proof, pi.as_slice())
        .is_ok();
    assert!(verify);
}
