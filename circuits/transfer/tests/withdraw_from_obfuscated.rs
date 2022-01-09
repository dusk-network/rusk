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
    let i_ssk = SecretSpendKey::random(rng);
    let i_psk = i_ssk.public_spend_key();

    let i_value = 100;
    let i_r = JubJubScalar::random(rng);
    let input = Message::new(rng, &i_r, &i_psk, i_value);

    let (_, i_blinder) = input
        .decrypt(&i_r, &i_psk)
        .expect("Failed to decrypt message");

    let c_ssk = SecretSpendKey::random(rng);
    let c_psk = c_ssk.public_spend_key();

    let c_value = 25;
    let c_r = JubJubScalar::random(rng);
    let change = Message::new(rng, &c_r, &c_psk, c_value);
    let c_pk_r = *c_psk.gen_stealth_address(&c_r).pk_r().as_ref();

    let (_, c_blinder) = change
        .decrypt(&c_r, &c_psk)
        .expect("Failed to decrypt message");

    let c_derive_key = DeriveKey::new(public_derive_key, &c_psk);

    let o_ssk = SecretSpendKey::random(rng);
    let o_psk = o_ssk.public_spend_key();

    let o_value = 75;

    let o_blinder = JubJubScalar::random(rng);
    let output = Note::obfuscated(rng, &o_psk, o_value, o_blinder);

    let input =
        WfoCommitment::new(i_value, i_blinder, *input.value_commitment());
    let change =
        WfoChange::new(change, c_value, c_blinder, c_r, c_pk_r, c_derive_key);
    let output =
        WfoCommitment::new(o_value, o_blinder, *output.value_commitment());

    WithdrawFromObfuscatedCircuit::new(input, change, output)
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
