// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::helpers::FETCH_PP_FROM_RUSK_PROFILE;
use crate::SendToContractObfuscatedCircuit;
use std::convert::TryInto;

use anyhow::{anyhow, Result};
use dusk_pki::{Ownable, SecretKey, SecretSpendKey};
use dusk_plonk::jubjub::GENERATOR_EXTENDED;
use phoenix_core::{Message, Note};
use poseidon252::sponge;
use rand::rngs::StdRng;
use rand::SeedableRng;
use schnorr::Signature;

use dusk_plonk::prelude::*;

#[test]
fn send_obfuscated() -> Result<()> {
    let mut rng = StdRng::seed_from_u64(2322u64);

    let ssk = SecretSpendKey::random(&mut rng);
    let psk = ssk.public_spend_key();

    let c_value = 100;
    let c_blinding_factor = JubJubScalar::random(&mut rng);

    let c_note = Note::obfuscated(&mut rng, &psk, c_value, c_blinding_factor);
    let c_sk_r = ssk.sk_r(c_note.stealth_address()).as_ref().clone();
    let c_pk_r = GENERATOR_EXTENDED * c_sk_r;

    let (_, crossover) = c_note.try_into().map_err(|e| {
        anyhow!("Failed to convert phoenix note into crossover: {:?}", e)
    })?;
    let c_value_commitment = *crossover.value_commitment();

    let c_schnorr_secret = SecretKey::from(c_sk_r);
    let c_commitment_hash = sponge::hash(&c_value_commitment.to_hash_inputs());
    let c_signature =
        Signature::new(&c_schnorr_secret, &mut rng, c_commitment_hash);

    let message_r = JubJubScalar::random(&mut rng);
    let message_value = 100;
    let message = Message::new(&mut rng, &message_r, &psk, message_value);
    let (message_value_p, message_blinding_factor) = message
        .decrypt(&message_r, &psk)
        .map_err(|e| anyhow!("Error decrypting the message: {:?}", e))?;
    assert_eq!(message_value, message_value_p);

    let mut circuit = SendToContractObfuscatedCircuit::new(
        c_value_commitment,
        c_pk_r,
        c_value,
        c_blinding_factor,
        c_signature,
        message_value,
        message_blinding_factor,
        message_r,
        *psk.A(),
        *message.value_commitment(),
        *message.nonce(),
        *message.cipher(),
    );

    let (pp, pk, vk) = if FETCH_PP_FROM_RUSK_PROFILE {
        circuit.rusk_circuit_args()?
    } else {
        let pp = PublicParameters::setup(circuit.get_trim_size(), &mut rng)?;
        let (pk, vk) = circuit.compile(&pp)?;

        circuit.get_mut_pi_positions().clear();

        (pp, pk, vk)
    };

    let proof = circuit.gen_proof(&pp, &pk, b"send-obfuscated")?;
    let pi = circuit.get_pi_positions().clone();

    let verify = circuit
        .verify_proof(&pp, &vk, b"send-obfuscated", &proof, pi.as_slice())
        .is_ok();
    assert!(verify);

    Ok(())
}
