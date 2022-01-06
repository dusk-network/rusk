// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use transfer_circuits::{
    DeriveKey, SendToContractObfuscatedCircuit, StcoCrossover, StcoMessage,
    TRANSCRIPT_LABEL,
};

use dusk_pki::SecretSpendKey;
use phoenix_core::{Message, Note};
use rand::rngs::StdRng;
use rand::{CryptoRng, Rng, RngCore, SeedableRng};

use dusk_plonk::prelude::*;

mod keys;

fn create_random_circuit<R: RngCore + CryptoRng>(
    rng: &mut R,
    public_derive_key: bool,
) -> SendToContractObfuscatedCircuit {
    let c_ssk = SecretSpendKey::random(rng);
    let c_psk = c_ssk.public_spend_key();

    let value = rng.gen();

    let c_blinder = JubJubScalar::random(rng);
    let c_note = Note::obfuscated(rng, &c_psk, value, c_blinder);

    let (mut fee, crossover) = c_note
        .try_into()
        .expect("Failed to convert note into fee/crossover pair!");

    fee.gas_limit = 5;
    fee.gas_price = 1;

    let m_ssk = SecretSpendKey::random(rng);
    let m_psk = m_ssk.public_spend_key();

    let m_r = JubJubScalar::random(rng);
    let message = Message::new(rng, &m_r, &m_psk, value);
    let m_pk_r = *m_psk.gen_stealth_address(&m_r).pk_r().as_ref();

    let (_, m_blinder) = message
        .decrypt(&m_r, &m_psk)
        .expect("Failed to decrypt message");

    let m_derive_key = DeriveKey::new(public_derive_key, &m_psk);

    let address = BlsScalar::random(rng);
    let signature = SendToContractObfuscatedCircuit::sign(
        rng, &c_ssk, &fee, &crossover, &message, &address,
    );

    let message =
        StcoMessage::new(message, m_r, m_derive_key, m_pk_r, m_blinder);
    let crossover = StcoCrossover::new(crossover, c_blinder);

    SendToContractObfuscatedCircuit::new(
        value, message, crossover, &fee, address, signature,
    )
}

#[test]
fn send_to_contract_obfuscated_public_key() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let (pp, pk, vd) = keys::circuit_keys::<SendToContractObfuscatedCircuit>()
        .expect("Failed to load keys!");

    let mut circuit = create_random_circuit(rng, true);

    let proof = circuit
        .prove(&pp, &pk, TRANSCRIPT_LABEL)
        .expect("Failed to prove circuit");

    let pi = circuit.public_inputs();

    SendToContractObfuscatedCircuit::verify(
        &pp,
        &vd,
        &proof,
        pi.as_slice(),
        TRANSCRIPT_LABEL,
    )
    .expect("Failed to verify");
}

#[test]
fn send_to_contract_obfuscated_secret_key() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let (pp, pk, vd) = keys::circuit_keys::<SendToContractObfuscatedCircuit>()
        .expect("Failed to load keys!");

    let mut circuit = create_random_circuit(rng, false);

    let proof = circuit
        .prove(&pp, &pk, TRANSCRIPT_LABEL)
        .expect("Failed to prove circuit");

    let pi = circuit.public_inputs();

    SendToContractObfuscatedCircuit::verify(
        &pp,
        &vd,
        &proof,
        pi.as_slice(),
        TRANSCRIPT_LABEL,
    )
    .expect("Failed to verify");
}
