// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;
use crate::prover::fetch_prover;

pub const WFCT_INPUT_LEN: usize =
    JubJubAffine::SIZE + u64::SIZE + JubJubScalar::SIZE;

pub static WFCT_PROVER: Lazy<PlonkProver> =
    Lazy::new(|| fetch_prover("WithdrawFromTransparentCircuit"));

impl LocalProver {
    pub(crate) fn local_prove_wfct(
        &self,
        circuit_inputs: &[u8],
    ) -> Result<Vec<u8>, ProverError> {
        let mut reader = circuit_inputs;

        if reader.len() != WFCT_INPUT_LEN {
            return Err(ProverError::from(format!(
                "Expected length {} got {}",
                WFCT_INPUT_LEN,
                reader.len()
            )));
        }

        let commitment = JubJubAffine::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("map_err", e))?
            .into();

        let value = u64::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("value", e))?;

        let blinder = JubJubScalar::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("blinder", e))?;

        let circ =
            WithdrawFromTransparentCircuit::new(commitment, value, blinder);

        let (proof, _) = WFCT_PROVER.prove(&mut OsRng, &circ).map_err(|e| {
            ProverError::with_context("Failed proving the circuit", e)
        })?;
        Ok(proof.to_bytes().to_vec())
    }
}
