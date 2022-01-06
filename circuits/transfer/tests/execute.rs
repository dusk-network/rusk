// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use transfer_circuits::ExecuteCircuit;

use rand::rngs::StdRng;
use rand::SeedableRng;

use dusk_plonk::prelude::*;

macro_rules! execute {
    ($test_name:ident, $inputs:expr, $outputs:expr) => {
        #[test]
        fn $test_name() {
            let mut rng = StdRng::seed_from_u64(2324u64);

            let tx_hash = BlsScalar::random(&mut rng);
            for use_crossover in [true, false].iter() {
                let (_, pp, _, vd, proof, pi) =
                    ExecuteCircuit::create_dummy_proof(
                        &mut rng,
                        $inputs,
                        $outputs,
                        *use_crossover,
                        tx_hash,
                    )
                    .expect("Failed to create the circuit!");

                ExecuteCircuit::verify(&pp, &vd, &proof, pi.as_slice())
                    .expect("Failed to verify the proof!");
            }
        }
    };
}

execute!(execute_1_0, 1, 0);
execute!(execute_1_1, 1, 1);
execute!(execute_1_2, 1, 2);

execute!(execute_2_0, 2, 0);
execute!(execute_2_1, 2, 1);
execute!(execute_2_2, 2, 2);

execute!(execute_3_0, 3, 0);
execute!(execute_3_1, 3, 1);
execute!(execute_3_2, 3, 2);

execute!(execute_4_0, 4, 0);
execute!(execute_4_1, 4, 1);
execute!(execute_4_2, 4, 2);
