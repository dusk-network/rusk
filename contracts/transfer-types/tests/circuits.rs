// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_jubjub::JubJubScalar;
use dusk_pki::SecretSpendKey;
use phoenix_core::{Message, Note};
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use std::convert::TryInto;
use transfer_circuits::{
    SendToContractObfuscatedCircuit, SendToContractTransparentCircuit,
};
use transfer_contract_types::*;

#[test]
fn stct_sign() {
    let mut rng = StdRng::seed_from_u64(2819u64);

    let ssk = SecretSpendKey::random(&mut rng);
    let psk = ssk.public_spend_key();

    let value = 100;
    let mut bytes = [0u8; 32];
    rng.fill_bytes(&mut bytes);
    let address = rusk_abi::gen_module_id(&bytes[..]);

    let blinding_factor = JubJubScalar::random(&mut rng);
    let note = Note::obfuscated(&mut rng, &psk, value, blinding_factor);
    let (_, crossover) = note.try_into().unwrap();

    let m = SendToContractTransparentCircuit::sign_message(
        &crossover,
        value,
        &rusk_abi::module_to_scalar(&address),
    );

    let m_p = sign_message_stct(&crossover, value, &address);

    assert_eq!(m, m_p);
}

#[test]
fn stco_sign() {
    let mut rng = StdRng::seed_from_u64(2819u64);

    let ssk = SecretSpendKey::random(&mut rng);
    let psk = ssk.public_spend_key();

    let value = 100;
    let mut bytes = [0u8; 32];
    rng.fill_bytes(&mut bytes);
    let address = rusk_abi::gen_module_id(&bytes[..]);

    let r = JubJubScalar::random(&mut rng);
    let message = Message::new(&mut rng, &r, &psk, value);
    let blinding_factor = JubJubScalar::random(&mut rng);
    let note = Note::obfuscated(&mut rng, &psk, value, blinding_factor);
    let (_, crossover) = note.try_into().unwrap();

    let m = SendToContractObfuscatedCircuit::sign_message(
        &crossover,
        &message,
        &rusk_abi::module_to_scalar(&address),
    );

    let m_p = sign_message_stco(&crossover, &message, &address);

    assert_eq!(m, m_p);
}
