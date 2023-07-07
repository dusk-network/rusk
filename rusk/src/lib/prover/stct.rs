// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::LazyLock;

use super::*;
use crate::error::Error;

use dusk_plonk::prelude::Prover;
use rand::rngs::OsRng;

pub const STCT_INPUT_LEN: usize = Fee::SIZE
    + Crossover::SIZE
    + u64::SIZE
    + JubJubScalar::SIZE
    + BlsScalar::SIZE
    + Signature::SIZE;

pub static STCT_PROVER: LazyLock<Prover> = LazyLock::new(|| {
    let keys = keys_for(SendToContractTransparentCircuit::circuit_id())
        .expect("keys to be available");
    let pk = keys.get_prover().expect("prover to be available");
    Prover::try_from_bytes(pk).expect("prover key to be valid")
});

impl RuskProver {
    pub fn prove_stct(&self, circuit_inputs: &[u8]) -> Result<Vec<u8>, Error> {
        info!("Received prove_stct request");
        let mut reader = circuit_inputs;

        if reader.len() != STCT_INPUT_LEN {
            return Err(other_error(
                format!(
                    "Expected length {} got {}",
                    STCT_INPUT_LEN,
                    reader.len()
                )
                .as_str(),
            )
            .into());
        }

        let fee = Fee::from_reader(&mut reader)
            .map_err(|_| other_error("Failed deserializing fee"))?;
        let crossover = Crossover::from_reader(&mut reader)
            .map_err(|_| other_error("Failed deserializing crossover"))?;
        let crossover_value = u64::from_reader(&mut reader)
            .map_err(|_| other_error("Failed deserializing crossover value"))?;
        let crossover_blinder = JubJubScalar::from_reader(&mut reader)
            .map_err(|_| other_error("Failed deserializing crossover value"))?;
        let contract_address =
            BlsScalar::from_reader(&mut reader).map_err(|_| {
                other_error("Failed deserializing contract address")
            })?;
        let signature = Signature::from_reader(&mut reader)
            .map_err(|_| other_error("Failed deserializing signature"))?;

        let circ = SendToContractTransparentCircuit::new(
            &fee,
            &crossover,
            crossover_value,
            crossover_blinder,
            contract_address,
            signature,
        );

        let (proof, _) = STCT_PROVER.prove(&mut OsRng, &circ).map_err(|e| {
            other_error(format!("Failed proving the circuit: {e}").as_str())
        })?;
        let proof = proof.to_bytes().to_vec();

        Ok(proof)
    }
}
