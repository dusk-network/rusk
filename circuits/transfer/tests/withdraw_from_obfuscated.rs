// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use transfer_circuits::{
    DeriveKey, WfoChange, WfoCommitment, WithdrawFromObfuscatedCircuit,
    TRANSCRIPT_LABEL,
};

use dusk_pki::SecretSpendKey;
use phoenix_core::{Message, Note};
use rand::rngs::StdRng;
use rand::{CryptoRng, RngCore, SeedableRng};

use dusk_plonk::prelude::*;

mod keys;

fn create_random_circuit<R: RngCore + CryptoRng>(
    rng: &mut R,
    public_derive_key: bool,
) -> WithdrawFromObfuscatedCircuit {
    let input = {
        let ssk = SecretSpendKey::random(rng);
        let psk = ssk.public_spend_key();

        let value = 100;
        let r = JubJubScalar::random(rng);
        let message = Message::new(rng, &r, &psk, value);

        let (_, blinder) = message
            .decrypt(&r, &psk)
            .expect("Failed to decrypt message");
        let commitment = *message.value_commitment();
        WfoCommitment {
            blinder,
            commitment,
            value,
        }
    };
    let change = {
        let ssk = SecretSpendKey::random(rng);
        let psk = ssk.public_spend_key();

        let value = 25;
        let r = JubJubScalar::random(rng);
        let message = Message::new(rng, &r, &psk, value);
        let pk_r = *psk.gen_stealth_address(&r).pk_r().as_ref();

        let (_, blinder) = message
            .decrypt(&r, &psk)
            .expect("Failed to decrypt message");

        let derive_key = DeriveKey::new(public_derive_key, &psk);
        WfoChange {
            blinder,
            derive_key,
            message,
            pk_r,
            r,
            value,
        }
    };

    let output = {
        let ssk = SecretSpendKey::random(rng);
        let psk = ssk.public_spend_key();

        let value = 75;

        let blinder = JubJubScalar::random(rng);
        let output = Note::obfuscated(rng, &psk, value, blinder);
        let commitment = *output.value_commitment();
        WfoCommitment {
            blinder,
            commitment,
            value,
        }
    };

    WithdrawFromObfuscatedCircuit {
        input,
        change,
        output,
    }
}

#[test]
fn withdraw_from_obfuscated_public() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let (pp, pk, vd) = keys::circuit_keys::<WithdrawFromObfuscatedCircuit>()
        .expect("Failed to generate circuit!");

    let mut circuit = create_random_circuit(rng, true);

    let proof = circuit
        .prove(&pp, &pk, TRANSCRIPT_LABEL)
        .expect("Failed to prove circuit");
    let pi = circuit.public_inputs();

    WithdrawFromObfuscatedCircuit::verify(
        &pp,
        &vd,
        &proof,
        pi.as_slice(),
        TRANSCRIPT_LABEL,
    )
    .expect("Failed to verify");
}

#[test]
fn withdraw_from_obfuscated_private() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let (pp, pk, vd) = keys::circuit_keys::<WithdrawFromObfuscatedCircuit>()
        .expect("Failed to generate circuit!");

    let mut circuit = create_random_circuit(rng, false);

    let proof = circuit
        .prove(&pp, &pk, TRANSCRIPT_LABEL)
        .expect("Failed to prove circuit");
    let pi = circuit.public_inputs();

    WithdrawFromObfuscatedCircuit::verify(
        &pp,
        &vd,
        &proof,
        pi.as_slice(),
        TRANSCRIPT_LABEL,
    )
    .expect("Failed to verify");
}
