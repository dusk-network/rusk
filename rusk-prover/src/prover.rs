// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod execute;

use crate::{ProverError, ProverResult};

use dusk_bytes::Serializable;
use execution_core::plonk::Prover as PlonkProver;
use once_cell::sync::Lazy;

#[cfg(not(feature = "no_random"))]
use rand::rngs::OsRng;

#[cfg(feature = "no_random")]
use rand::{rngs::StdRng, SeedableRng};

#[derive(Debug, Default)]
pub struct LocalProver;

impl crate::Prover for LocalProver {
    fn prove_execute(&self, circuit_inputs: &[u8]) -> ProverResult {
        self.local_prove_execute(circuit_inputs)
    }
}

pub fn fetch_prover(circuit_name: &str) -> PlonkProver {
    let circuit_profile = rusk_profile::Circuit::from_name(circuit_name)
        .unwrap_or_else(|_| {
            panic!("There should be circuit data stored for {}", circuit_name)
        });
    let pk = circuit_profile.get_prover().unwrap_or_else(|_| {
        panic!("there should be a prover key stored for {}", circuit_name)
    });

    PlonkProver::try_from_bytes(pk).expect("Prover key is expected to by valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Prover;

    #[test]
    fn test_prove_execute() {
        let utx_hex = include_str!("../tests/utx.hex");
        let utx_bytes = hex::decode(utx_hex).unwrap();
        let prover = LocalProver {};
        let proof = prover.prove_execute(&utx_bytes).unwrap();
        println!("{}", hex::encode(proof));
    }
}
