// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod execute;
mod stct;
mod wfct;

use crate::{ProverError, ProverResult};

use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_plonk::prelude::Prover as PlonkProver;
use once_cell::sync::Lazy;

#[cfg(not(feature = "no_random"))]
use rand::rngs::OsRng;

#[cfg(feature = "no_random")]
use rand::{rngs::StdRng, SeedableRng};

use dusk_plonk::prelude::*;
use dusk_schnorr::Signature;
use phoenix_core::{Crossover, Fee};

use transfer_circuits::{
    CircuitInput, CircuitInputSignature, ExecuteCircuit,
    SendToContractTransparentCircuit, WithdrawFromTransparentCircuit,
};

pub use stct::STCT_INPUT_LEN;
pub use wfct::WFCT_INPUT_LEN;

/// Arity of the transfer tree.
pub const A: usize = 4;

#[derive(Debug, Default)]
pub struct LocalProver;

impl crate::Prover for LocalProver {
    fn prove_execute(&self, circuit_inputs: &[u8]) -> ProverResult {
        self.local_prove_execute(circuit_inputs)
    }

    fn prove_stct(&self, circuit_inputs: &[u8]) -> ProverResult {
        self.local_prove_stct(circuit_inputs)
    }

    fn prove_wfct(&self, circuit_inputs: &[u8]) -> ProverResult {
        self.local_prove_wfct(circuit_inputs)
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

    Prover::try_from_bytes(pk).expect("Prover key is expected to by valid")
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
