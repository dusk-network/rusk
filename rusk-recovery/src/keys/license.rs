// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::keys::PUB_PARAMS;
use crate::keys::{CircuitLoader, TRANSCRIPT_LABEL};
use dusk_plonk::prelude::*;
use rand::rngs::OsRng;
use transfer_circuits::*;
use license_circuits::*;

// todo: remove duplication
macro_rules! loader_impl {
    ($loader:ident, $circuit:ty, $circuit_name:expr) => {
        pub struct $loader;
        impl CircuitLoader for $loader {
            fn circuit_id(&self) -> &[u8; 32] {
                <$circuit>::circuit_id()
            }

            fn circuit_name(&self) -> &'static str {
                $circuit_name
            }

            fn compile_circuit(
                &self,
            ) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
                let (prover, verifier) = Compiler::compile::<$circuit>(
                    &PUB_PARAMS,
                    &TRANSCRIPT_LABEL,
                )?;

                Ok((prover.to_bytes(), verifier.to_bytes()))
            }
        }
    };
}

// todo: remove duplication
macro_rules! loader_impl_execute {
    ($loader:ident, $circuit:ty, $circuit_name:expr) => {
        pub struct $loader;
        impl CircuitLoader for $loader {
            fn circuit_id(&self) -> &[u8; 32] {
                <$circuit>::circuit_id()
            }

            fn circuit_name(&self) -> &'static str {
                $circuit_name
            }

            fn compile_circuit(
                &self,
            ) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
                let rng = &mut OsRng;
                let circuit = <$circuit>::create_dummy_circuit(
                    rng,
                    true,
                    BlsScalar::default(),
                )?;

                let (prover, verifier) = Compiler::compile_with_circuit(
                    &PUB_PARAMS,
                    &TRANSCRIPT_LABEL,
                    &circuit,
                )?;

                Ok((prover.to_bytes(), verifier.to_bytes()))
            }
        }
    };
}

loader_impl!(
    LicenseCircuitLoader,
    LicenseCircuit,
    "LicenseCircuit"
);
// loader_impl_execute!(
//     ExecuteOneTwoCircuitLoader,
//     ExecuteCircuitOneTwo,
//     "ExecuteOneTwo"
// );
