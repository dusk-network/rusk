// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use transfer_circuits::{WithdrawFromTransparentCircuit, TRANSCRIPT_LABEL};

use dusk_pki::SecretSpendKey;
use dusk_plonk::circuit;
use phoenix_core::Note;
use rand::rngs::StdRng;
use rand::SeedableRng;

use dusk_plonk::prelude::*;

mod keys;

#[test]
fn withdraw_from_transparent() {
    let mut rng = StdRng::seed_from_u64(2322u64);

    let ssk = SecretSpendKey::random(&mut rng);
    let vk = ssk.view_key();
    let psk = ssk.public_spend_key();

    let value = 100;
    let blinding_factor = JubJubScalar::random(&mut rng);

    let note = Note::obfuscated(&mut rng, &psk, value, blinding_factor);

    let mut circuit = WithdrawFromTransparentCircuit::new(&note, Some(&vk))
        .expect("Failed to create WFT circuit!");

    let (pp, pk, vd) = keys::circuit_keys::<WithdrawFromTransparentCircuit>()
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
