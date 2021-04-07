// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use transfer_circuits::{builder, WithdrawFromObfuscatedCircuit};

use dusk_pki::SecretSpendKey;
use dusk_plonk::circuit;
use phoenix_core::{Message, Note};
use rand::rngs::StdRng;
use rand::SeedableRng;

use dusk_plonk::prelude::*;

#[test]
fn withdraw_from_obfuscated() {
    let mut rng = StdRng::seed_from_u64(2324u64);

    let i_ssk = SecretSpendKey::random(&mut rng);
    let i_vk = i_ssk.view_key();
    let i_psk = i_ssk.public_spend_key();
    let i_value = 100;
    let i_blinding_factor = JubJubScalar::random(&mut rng);
    let i_note = Note::obfuscated(&mut rng, &i_psk, i_value, i_blinding_factor);

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
    let o_note = Note::obfuscated(&mut rng, &o_psk, o_value, o_blinding_factor);

    let mut circuit = WithdrawFromObfuscatedCircuit::new(
        &i_note,
        Some(&i_vk),
        &c,
        c_r,
        &c_psk,
        &o_note,
        Some(&o_vk),
    )
    .expect("Failed to generate circuit!");

    let id = WithdrawFromObfuscatedCircuit::rusk_keys_id();
    let (pp, pk, vd) =
        builder::circuit_keys(&mut rng, None, &mut circuit, id, true)
            .expect("Failed to generate circuit!");

    let proof = circuit
        .gen_proof(&pp, &pk, b"dusk-network")
        .expect("Failed to generate proof!");
    let pi = circuit.public_inputs();

    circuit::verify_proof(
        &pp,
        vd.key(),
        &proof,
        pi.as_slice(),
        vd.pi_pos(),
        b"dusk-network",
    )
    .expect("Failed to verify the proof!");
}
