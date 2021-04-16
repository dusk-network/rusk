// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::convert::TryInto;
use transfer_circuits::{builder, SendToContractTransparentCircuit};

use dusk_pki::SecretSpendKey;
use dusk_plonk::circuit;
use phoenix_core::Note;
use rand::rngs::StdRng;
use rand::SeedableRng;

use dusk_plonk::prelude::*;

#[test]
fn send_to_contract_transparent() {
    let mut rng = StdRng::seed_from_u64(2322u64);

    let c_ssk = SecretSpendKey::random(&mut rng);
    let c_vk = c_ssk.view_key();
    let c_psk = c_ssk.public_spend_key();

    let c_address = BlsScalar::random(&mut rng);

    let c_value = 100;
    let c_blinding_factor = JubJubScalar::random(&mut rng);

    let c_note = Note::obfuscated(&mut rng, &c_psk, c_value, c_blinding_factor);
    let (mut fee, crossover) = c_note
        .try_into()
        .expect("Failed to convert note into fee/crossover pair!");
    fee.gas_limit = 5;
    fee.gas_price = 1;

    let c_signature = SendToContractTransparentCircuit::sign(
        &mut rng, &c_ssk, &fee, &crossover, c_value, &c_address,
    );

    let mut circuit = SendToContractTransparentCircuit::new(
        fee,
        crossover,
        &c_vk,
        c_address,
        c_signature,
    )
    .expect("Failed to create STCT circuit!");

    let id = SendToContractTransparentCircuit::rusk_keys_id();
    let (pp, pk, vd) =
        builder::circuit_keys(&mut rng, None, &mut circuit, id, true)
            .expect("Failed to generate circuit!");

    let proof = circuit
        .gen_proof(&pp, &pk, b"dusk-network")
        .expect("Failed to generate proof!");
    let pi = circuit.public_inputs();

    circuit::verify_proof(
        &pp,
        vd.key(),
        &proof,
        pi.as_slice(),
        vd.pi_pos(),
        b"dusk-network",
    )
    .expect("Failed to verify the proof!");
}
