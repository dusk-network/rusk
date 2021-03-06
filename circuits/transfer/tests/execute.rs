// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use transfer_circuits::{ExecuteCircuit, TRANSCRIPT_LABEL};

use dusk_plonk::circuit;
use rand::rngs::StdRng;
use rand::SeedableRng;

#[test]
fn execute() {
    let mut rng = StdRng::seed_from_u64(2324u64);

    for inputs in 1..5 {
        for outputs in 0..3 {
            for use_crossover in [true, false].iter() {
                let (_, pp, _, vd, proof, pi) =
                    ExecuteCircuit::create_dummy_proof(
                        &mut rng,
                        inputs,
                        outputs,
                        *use_crossover,
                    )
                    .expect("Failed to create the circuit!");

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
        }
    }
}
