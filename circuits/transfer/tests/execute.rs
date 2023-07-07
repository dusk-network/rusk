// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use transfer_circuits::{
    ExecuteCircuitFourTwo, ExecuteCircuitOneTwo, ExecuteCircuitThreeTwo,
    ExecuteCircuitTwoTwo,
};

use rand::rngs::StdRng;
use rand::SeedableRng;

use dusk_plonk::prelude::*;

macro_rules! execute {
    ($test_name:ident, $ty:ty) => {
        #[test]
        fn $test_name() {
            let mut rng = StdRng::seed_from_u64(2324u64);

            let tx_hash = BlsScalar::random(&mut rng);
            for use_crossover in [true, false].iter() {
                let (_, _, verifier, proof, public_inputs) =
                    <$ty>::create_dummy_proof(
                        &mut rng,
                        *use_crossover,
                        tx_hash,
                    )
                    .expect("Creating a proof should succeed");

                verifier
                    .verify(&proof, &public_inputs)
                    .expect("Proof verification should be successful");
            }
        }
    };
}

execute!(execute_1_2, ExecuteCircuitOneTwo<(), 17, 4>);
execute!(execute_2_2, ExecuteCircuitTwoTwo<(), 17, 4>);
execute!(execute_3_2, ExecuteCircuitThreeTwo<(), 17, 4>);
execute!(execute_4_2, ExecuteCircuitFourTwo<(), 17, 4>);
