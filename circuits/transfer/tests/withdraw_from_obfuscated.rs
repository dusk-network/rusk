// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use transfer_circuits::{
    CircuitValueOpening, WithdrawFromObfuscatedChange,
    WithdrawFromObfuscatedCircuit, TRANSCRIPT_LABEL,
};

use dusk_pki::SecretSpendKey;
use dusk_plonk::circuit;
use phoenix_core::{Message, Note};
use rand::rngs::StdRng;
use rand::SeedableRng;

use dusk_plonk::prelude::*;

mod keys;

#[test]
fn withdraw_from_obfuscated_public() {
    let rng = &mut StdRng::seed_from_u64(2324u64);

    let m_r = JubJubScalar::random(rng);
    let m_ssk = SecretSpendKey::random(rng);
    let m_psk = m_ssk.public_spend_key();
    let m_value = 100;
    let m = Message::new(rng, &m_r, &m_psk, m_value);

    let c_r = JubJubScalar::random(rng);
    let c_ssk = SecretSpendKey::random(rng);
    let c_psk = c_ssk.public_spend_key();
    let c_value = 13;
    let c = Message::new(rng, &c_r, &c_psk, c_value);

    let o_ssk = SecretSpendKey::random(rng);
    let o_vk = o_ssk.view_key();
    let o_psk = o_ssk.public_spend_key();
    let o_value = 87;
    let o_blinding_factor = JubJubScalar::random(rng);
    let o = Note::obfuscated(rng, &o_psk, o_value, o_blinding_factor);

    let input = CircuitValueOpening::from_message(&m, &m_psk, &m_r)
        .expect("Failed to generate WFO input");

    let output = CircuitValueOpening::from_note(&o, Some(&o_vk))
        .expect("Failed to generate WFO output");

    for public_derive_key in [true, false].iter() {
        let change = WithdrawFromObfuscatedChange::new(
            c,
            c_r,
            c_psk,
            *public_derive_key,
        )
        .expect("Failed to generate WFO change");

        let mut circuit =
            WithdrawFromObfuscatedCircuit::new(input, change, output);

        let (pp, pk, vd) =
            keys::circuit_keys::<WithdrawFromObfuscatedCircuit>()
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
}
