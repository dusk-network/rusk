// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;
use crate::prover::fetch_prover;

pub const STCT_INPUT_LEN: usize = Fee::SIZE
    + Crossover::SIZE
    + u64::SIZE
    + JubJubScalar::SIZE
    + BlsScalar::SIZE
    + Signature::SIZE;

pub static STCT_PROVER: Lazy<PlonkProver> =
    Lazy::new(|| fetch_prover("SendToContractTransparentCircuit"));

impl LocalProver {
    pub(crate) fn local_prove_stct(
        &self,
        circuit_inputs: &[u8],
    ) -> Result<Vec<u8>, ProverError> {
        let mut reader = circuit_inputs;

        if reader.len() != STCT_INPUT_LEN {
            return Err(ProverError::from(format!(
                "Expected length {} got {}",
                STCT_INPUT_LEN,
                reader.len()
            )));
        }

        let fee = Fee::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("fee", e))?;
        let crossover = Crossover::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("crossover", e))?;
        let crossover_value = u64::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("crossover_value", e))?;
        let crossover_blinder = JubJubScalar::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("crossover_blinder", e))?;
        let contract_address = BlsScalar::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("contract_address", e))?;
        let signature = Signature::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("signature", e))?;

        let circ = SendToContractTransparentCircuit::new(
            &fee,
            &crossover,
            crossover_value,
            crossover_blinder,
            contract_address,
            signature,
        );

        #[cfg(not(feature = "no_random"))]
        let rng = &mut OsRng;

        #[cfg(feature = "no_random")]
        let rng = &mut StdRng::seed_from_u64(0xbeef);

        let (proof, _) = STCT_PROVER.prove(rng, &circ).map_err(|e| {
            ProverError::with_context("Failed proving the circuit", e)
        })?;
        Ok(proof.to_bytes().to_vec())
    }
}
