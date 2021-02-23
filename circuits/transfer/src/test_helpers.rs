// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{builder, ExecuteCircuit};

use anyhow::Result;
use canonical_host::MemStore;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rand_core::{CryptoRng, RngCore};

use dusk_plonk::prelude::*;

pub fn execute_circuit(inputs: usize, outputs: usize, crossover: bool) {
    let mut rng = StdRng::seed_from_u64(2324u64);

    #[cfg(feature = "test-force-keys-generation")]
    let use_rusk_profile = false;

    #[cfg(not(feature = "test-force-keys-generation"))]
    let use_rusk_profile = true;

    let (mut circuit, pp, _, vk, proof, pi) =
        ExecuteCircuit::create_dummy_proof::<_, MemStore>(
            &mut rng,
            None,
            inputs,
            outputs,
            crossover,
            use_rusk_profile,
        )
        .expect("Failed to create the circuit!");

    let label = ExecuteCircuit::transcript_label();
    circuit
        .verify_proof(&pp, &vk, label, &proof, pi.as_slice())
        .expect("Failed to verify the proof!");
}

pub fn circuit<'a, R, C, F>(rng: &mut R, id: &str, circuit: F) -> Result<()>
where
    R: RngCore + CryptoRng,
    C: Circuit<'a>,
    F: FnOnce(&mut R) -> Result<C>,
{
    let mut circuit = circuit(rng)?;
    let (pp, pk, vk) =
        builder::circuit_keys(rng, None, &mut circuit, id, true)?;

    let proof = circuit.gen_proof(&pp, &pk, b"send-obfuscated")?;
    let pi = circuit.get_pi_positions().clone();

    let verify = circuit
        .verify_proof(&pp, &vk, b"send-obfuscated", &proof, pi.as_slice())
        .is_ok();
    assert!(verify);

    Ok(())
}
