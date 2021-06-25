// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::convert::TryInto;
use transfer_circuits::{SendToContractObfuscatedCircuit, TRANSCRIPT_LABEL};

use dusk_pki::SecretSpendKey;
use dusk_plonk::circuit;
use phoenix_core::{Message, Note};
use rand::rngs::StdRng;
use rand::SeedableRng;

use dusk_plonk::prelude::*;

mod keys;

#[test]
fn send_to_contract_obfuscated_public_key() {
    let mut rng = StdRng::seed_from_u64(2322u64);

    let ssk = SecretSpendKey::random(&mut rng);
    let vk = ssk.view_key();
    let psk = ssk.public_spend_key();

    let c_address = BlsScalar::random(&mut rng);

    let c_value = 100;
    let c_blinding_factor = JubJubScalar::random(&mut rng);
    let c_note = Note::obfuscated(&mut rng, &psk, c_value, c_blinding_factor);
    let (mut fee, crossover) = c_note
        .try_into()
        .expect("Failed to convert note into fee/crossover pair!");
    fee.gas_limit = 5;
    fee.gas_price = 1;

    let message_r = JubJubScalar::random(&mut rng);
    let message_value = 100;
    let message = Message::new(&mut rng, &message_r, &psk, message_value);

    let c_signature = SendToContractObfuscatedCircuit::sign(
        &mut rng, &ssk, &fee, &crossover, &message, &c_address,
    );

    let mut circuit = SendToContractObfuscatedCircuit::new(
        fee,
        crossover,
        &vk,
        c_signature,
        true,
        message,
        &psk,
        message_r,
        c_address,
    )
    .expect("Failed to generate circuit!");

    let (pp, pk, vd) = keys::circuit_keys::<SendToContractObfuscatedCircuit>()
        .expect("Failed to generate circuit!");

    let proof = circuit
        .gen_proof(&pp, &pk, TRANSCRIPT_LABEL)
        .expect("Failed to generate proof!");
    let pi = circuit.public_inputs();

    assert!(circuit.is_message_derive_key_public());

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

#[test]
fn send_to_contract_obfuscated_private_key() {
    let mut rng = StdRng::seed_from_u64(2322u64);

    let ssk = SecretSpendKey::random(&mut rng);
    let vk = ssk.view_key();
    let psk = ssk.public_spend_key();

    let c_address = BlsScalar::random(&mut rng);

    let c_value = 100;
    let c_blinding_factor = JubJubScalar::random(&mut rng);
    let c_note = Note::obfuscated(&mut rng, &psk, c_value, c_blinding_factor);
    let (mut fee, crossover) = c_note
        .try_into()
        .expect("Failed to convert note into fee/crossover pair!");
    fee.gas_limit = 5;
    fee.gas_price = 1;

    let message_r = JubJubScalar::random(&mut rng);
    let message_value = 100;
    let message = Message::new(&mut rng, &message_r, &psk, message_value);

    let c_signature = SendToContractObfuscatedCircuit::sign(
        &mut rng, &ssk, &fee, &crossover, &message, &c_address,
    );

    let mut circuit = SendToContractObfuscatedCircuit::new(
        fee,
        crossover,
        &vk,
        c_signature,
        false,
        message,
        &psk,
        message_r,
        c_address,
    )
    .expect("Failed to generate circuit!");

    let (pp, pk, vd) = keys::circuit_keys::<SendToContractObfuscatedCircuit>()
        .expect("Failed to generate circuit!");

    let proof = circuit
        .gen_proof(&pp, &pk, TRANSCRIPT_LABEL)
        .expect("Failed to generate proof!");
    let pi = circuit.public_inputs();

    assert!(!circuit.is_message_derive_key_public());

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
