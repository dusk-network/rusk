// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use transfer_circuits::{WithdrawFromTransparentCircuit, TRANSCRIPT_LABEL};

use dusk_pki::SecretSpendKey;
use phoenix_core::Note;
use rand::rngs::StdRng;
use rand::{CryptoRng, Rng, RngCore, SeedableRng};

use dusk_plonk::prelude::*;

mod keys;

fn create_random_circuit<R: RngCore + CryptoRng>(
    rng: &mut R,
) -> WithdrawFromTransparentCircuit {
    let ssk = SecretSpendKey::random(rng);
    let psk = ssk.public_spend_key();

    let value = rng.gen();
    let blinder = JubJubScalar::random(rng);

    let note = Note::obfuscated(rng, &psk, value, blinder);
    let commitment = *note.value_commitment();

    WithdrawFromTransparentCircuit::new(commitment, value, blinder)
}

#[test]
fn withdraw_from_transparent() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let (pp, pk, vd) = keys::circuit_keys::<WithdrawFromTransparentCircuit>()
        .expect("Failed to generate circuit!");

    let mut circuit = create_random_circuit(rng);

    let proof = circuit
        .prove(&pp, &pk, TRANSCRIPT_LABEL)
        .expect("Failed to prove circuit");
    let pi = circuit.public_inputs();

    WithdrawFromTransparentCircuit::verify(
        &pp,
        &vd,
        &proof,
        pi.as_slice(),
        TRANSCRIPT_LABEL,
    )
    .expect("Failed to verify");
}
