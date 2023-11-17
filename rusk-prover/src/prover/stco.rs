// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;
use crate::prover::fetch_prover;

pub const STCO_INPUT_LEN: usize = u64::SIZE
    + JubJubScalar::SIZE
    + JubJubScalar::SIZE
    + u64::SIZE
    + PublicSpendKey::SIZE
    + JubJubAffine::SIZE
    + Message::SIZE
    + JubJubScalar::SIZE
    + Crossover::SIZE
    + Fee::SIZE
    + BlsScalar::SIZE
    + Signature::SIZE;

pub static STCO_PROVER: Lazy<PlonkProver> =
    Lazy::new(|| fetch_prover("SendToContractObfuscatedCircuit"));

impl LocalProver {
    pub(crate) fn local_prove_stco(
        &self,
        circuit_inputs: &[u8],
    ) -> Result<Vec<u8>, ProverError> {
        let mut reader = circuit_inputs;

        if reader.len() != STCO_INPUT_LEN {
            return Err(ProverError::from(format!(
                "Expected length {} got {}",
                STCO_INPUT_LEN,
                reader.len()
            )));
        }

        let value = u64::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("value", e))?;
        let r = JubJubScalar::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("r", e))?;
        let blinder = JubJubScalar::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("blinder", e))?;
        let is_public = u64::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("is_public", e))?
            != 0;
        let psk = PublicSpendKey::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("psk", e))?;
        let pk_r = JubJubAffine::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("pk_r", e))?
            .into();
        let message = Message::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("message", e.into()))?;
        let crossover_blinder = JubJubScalar::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("crossover_blinder", e))?;
        let crossover = Crossover::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("crossover", e))?;
        let fee = Fee::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("fee", e))?;
        let contract_address = BlsScalar::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("contract_address", e))?;
        let signature = Signature::from_reader(&mut reader)
            .map_err(|e| ProverError::invalid_data("signature", e))?;

        let derive_key = DeriveKey::new(is_public, &psk);

        let stco_message = StcoMessage {
            r,
            blinder,
            derive_key,
            pk_r,
            message,
        };
        let stco_crossover = StcoCrossover::new(crossover, crossover_blinder);

        let circ = SendToContractObfuscatedCircuit::new(
            value,
            stco_message,
            stco_crossover,
            &fee,
            contract_address,
            signature,
        );

        #[cfg(not(feature = "no_random"))]
        let rng = &mut OsRng;

        #[cfg(feature = "no_random")]
        let rng = &mut StdRng::seed_from_u64(0xbeef);

        let (proof, _) = STCO_PROVER.prove(rng, &circ).map_err(|e| {
            ProverError::with_context("Failed proving the circuit", e)
        })?;
        Ok(proof.to_bytes().to_vec())
    }
}
