// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use transfer_circuits::{WithdrawFromObfuscatedCircuit, TRANSCRIPT_LABEL};

use dusk_pki::SecretSpendKey;
use dusk_plonk::circuit;
use phoenix_core::{Message, Note};
use rand::rngs::StdRng;
use rand::SeedableRng;

use dusk_plonk::prelude::*;

mod keys;

#[test]
fn withdraw_from_obfuscated() {
    let rng = &mut StdRng::seed_from_u64(2324u64);

    let m_r = JubJubScalar::random(rng);
    let m_ssk = SecretSpendKey::random(rng);
    let m_psk = m_ssk.public_spend_key();
    let m_value = 100;
    let m = Message::new(rng, &m_r, &m_psk, m_value);

    let o_ssk = SecretSpendKey::random(rng);
    let o_vk = o_ssk.view_key();
    let o_psk = o_ssk.public_spend_key();
    let o_value = 100;
    let o_blinding_factor = JubJubScalar::random(rng);
    let o = Note::obfuscated(rng, &o_psk, o_value, o_blinding_factor);

    let mut circuit =
        WithdrawFromObfuscatedCircuit::new(m_r, &m_ssk, &m, &o, Some(&o_vk))
            .expect("Failed to generate circuit!");

    let (pp, pk, vd) = keys::circuit_keys::<WithdrawFromObfuscatedCircuit>()
        .expect("Failed to generate circuit!");

    let proof = circuit
        .gen_proof(&pp, &pk, TRANSCRIPT_LABEL)
        .expect("Failed to generate proof!");
    let pi = circuit.public_inputs();

    circuit::verify_proof(
        &pp,
        vd.key(),
        &proof,
        pi.as_slice(),
        vd.pi_pos(),
        TRANSCRIPT_LABEL,
    )
    .expect("Failed to verify the proof!");
}
