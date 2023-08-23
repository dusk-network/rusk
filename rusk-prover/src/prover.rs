// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod execute;
mod stco;
mod stct;
mod wfco;
mod wfct;

use crate::{ProverError, ProverResult};

use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_pki::PublicSpendKey;
use dusk_plonk::prelude::Prover as PlonkProver;
use once_cell::sync::Lazy;
use rand::rngs::OsRng;

use dusk_plonk::prelude::*;
use dusk_schnorr::Signature;
use phoenix_core::{Crossover, Fee, Message};
use rusk_profile::keys_for;

use transfer_circuits::{
    CircuitInput, CircuitInputSignature, DeriveKey, ExecuteCircuit,
    SendToContractObfuscatedCircuit, SendToContractTransparentCircuit,
    StcoCrossover, StcoMessage, WfoChange, WfoCommitment,
    WithdrawFromObfuscatedCircuit, WithdrawFromTransparentCircuit,
};

pub use stco::STCO_INPUT_LEN;
pub use stct::STCT_INPUT_LEN;
pub use wfco::WFCO_INPUT_LEN;
pub use wfct::WFCT_INPUT_LEN;

/// Arity of the transfer tree.
pub const A: usize = 4;

#[derive(Debug, Default)]
pub struct LocalProver;

impl crate::Prover for LocalProver {
    fn prove_execute(&self, circuit_inputs: &[u8]) -> ProverResult {
        self.local_prove_execute(circuit_inputs)
    }

    fn prove_stco(&self, circuit_inputs: &[u8]) -> ProverResult {
        self.local_prove_stco(circuit_inputs)
    }

    fn prove_stct(&self, circuit_inputs: &[u8]) -> ProverResult {
        self.local_prove_stct(circuit_inputs)
    }

    fn prove_wfco(&self, circuit_inputs: &[u8]) -> ProverResult {
        self.local_prove_wfco(circuit_inputs)
    }

    fn prove_wfct(&self, circuit_inputs: &[u8]) -> ProverResult {
        self.local_prove_wfct(circuit_inputs)
    }
}

#[macro_export]
macro_rules! lazy_prover {
    ($circuit:ty) => {
        Lazy::new(|| {
            let keys = keys_for(<$circuit>::circuit_id())
                .expect("keys to be available");
            let pk = keys.get_prover().expect("prover to be available");
            Prover::try_from_bytes(&pk).expect("prover key to be valid")
        })
    };
}

#[cfg(test)]
mod tests {
    use transfer_circuits::{
        ExecuteCircuitFourTwo, ExecuteCircuitOneTwo, ExecuteCircuitThreeTwo,
        ExecuteCircuitTwoTwo, SendToContractObfuscatedCircuit,
        SendToContractTransparentCircuit, WithdrawFromObfuscatedCircuit,
        WithdrawFromTransparentCircuit,
    };

    use super::*;
    use crate::Prover;

    #[test]
    fn test_prove_execute() {
        println!(
            "STCT   {}",
            hex::encode(SendToContractTransparentCircuit::circuit_id())
        );
        println!(
            "STCO   {}",
            hex::encode(SendToContractObfuscatedCircuit::circuit_id())
        );
        println!(
            "WFCT   {}",
            hex::encode(WithdrawFromTransparentCircuit::circuit_id())
        );
        println!(
            "WFCO   {}",
            hex::encode(WithdrawFromObfuscatedCircuit::circuit_id())
        );

        println!("Exec 1 {}", hex::encode(ExecuteCircuitOneTwo::circuit_id()));

        println!("Exec 2 {}", hex::encode(ExecuteCircuitTwoTwo::circuit_id()));

        println!(
            "Exec 3 {}",
            hex::encode(ExecuteCircuitThreeTwo::circuit_id())
        );

        println!(
            "Exec 4 {}",
            hex::encode(ExecuteCircuitFourTwo::circuit_id())
        );

        let utx_hex = include_str!("../tests/utx.hex");
        let utx_bytes = hex::decode(utx_hex).unwrap();
        let prover = LocalProver {};
        let proof = prover.prove_execute(&utx_bytes).unwrap();
        println!("{}", hex::encode(proof));
    }
}
