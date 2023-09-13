// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;
use crate::prover::fetch_prover;

pub const WFCO_INPUT_LEN: usize = u64::SIZE
    + JubJubScalar::SIZE
    + JubJubAffine::SIZE
    + u64::SIZE
    + Message::SIZE
    + JubJubScalar::SIZE
    + JubJubScalar::SIZE
    + u64::SIZE
    + PublicSpendKey::SIZE
    + JubJubAffine::SIZE
    + u64::SIZE
    + JubJubScalar::SIZE
    + JubJubAffine::SIZE;

pub static WFCO_PROVER: Lazy<PlonkProver> =
    Lazy::new(|| fetch_prover("WithdrawFromObfuscatedCircuit"));

impl LocalProver {
    pub(crate) fn local_prove_wfco(
        &self,
        circuit_inputs: &[u8],
    ) -> Result<Vec<u8>, ProverError> {
        let mut reader = circuit_inputs;

        if reader.len() != WFCO_INPUT_LEN {
            return Err(ProverError::from(format!(
                "Expected length {} got {}",
                WFCO_INPUT_LEN,
                reader.len()
            )));
        }

        let input_value = u64::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("input_value", e))?;
        let input_blinder = JubJubScalar::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("input_blinder", e))?;
        let input_commitment = JubJubAffine::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("input_commitment", e))?
            .into();

        let input = WfoCommitment {
            value: input_value,
            blinder: input_blinder,
            commitment: input_commitment,
        };

        let change_value = u64::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("change_value", e))?;
        let change_message =
            Message::from_reader(&mut reader).map_err(|e| {
                ProverError::invalid_data("change_message", e.into())
            })?;
        let change_blinder = JubJubScalar::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("change_blinder", e))?;
        let r = JubJubScalar::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("r", e))?;
        let is_public = u64::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("is_public", e))?
            != 0;
        let psk = PublicSpendKey::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("psk", e))?;
        let pk_r = JubJubAffine::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("pk_r", e))?
            .into();

        let derive_key = DeriveKey::new(is_public, &psk);

        let change = WfoChange {
            value: change_value,
            message: change_message,
            blinder: change_blinder,
            r,
            derive_key,
            pk_r,
        };

        let output_value = u64::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("output_value", e))?;
        let output_blinder = JubJubScalar::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("output_blinder", e))?;
        let output_commitment = JubJubAffine::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("output_commitment", e))?
            .into();

        let output = WfoCommitment {
            value: output_value,
            blinder: output_blinder,
            commitment: output_commitment,
        };

        let circ = WithdrawFromObfuscatedCircuit {
            input,
            change,
            output,
        };

        let (proof, _) = WFCO_PROVER.prove(&mut OsRng, &circ).map_err(|e| {
            ProverError::with_context("Failed proving the circuit", e)
        })?;
        Ok(proof.to_bytes().to_vec())
    }
}
