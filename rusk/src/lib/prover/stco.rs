// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;
use crate::error::Error;

use dusk_plonk::prelude::Prover;
use rand::rngs::OsRng;
use std::sync::LazyLock;

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

pub static STCO_PROVER: LazyLock<Prover> = LazyLock::new(|| {
    let keys = keys_for(SendToContractObfuscatedCircuit::circuit_id())
        .expect("keys to be available");
    let pk = keys.get_prover().expect("prover to be available");
    Prover::try_from_bytes(pk).expect("prover key to be valid")
});

impl RuskProver {
    pub fn prove_stco(&self, circuit_inputs: &[u8]) -> Result<Vec<u8>, Error> {
        info!("Received prove_stco request");
        let mut reader = circuit_inputs;

        if reader.len() != STCO_INPUT_LEN {
            return Err(other_error(
                format!(
                    "Expected length {} got {}",
                    STCO_INPUT_LEN,
                    reader.len()
                )
                .as_str(),
            )
            .into());
        }

        let value = u64::from_reader(&mut reader)
            .map_err(|_| other_error("Failed deserializing value"))?;
        let r = JubJubScalar::from_reader(&mut reader)
            .map_err(|_| other_error("Failed deserializing 'r'"))?;
        let blinder = JubJubScalar::from_reader(&mut reader)
            .map_err(|_| other_error("Failed deserializing blinder"))?;
        let is_public = u64::from_reader(&mut reader)
            .map_err(|_| other_error("Failed deserializing is_public"))?
            != 0;
        let psk = PublicSpendKey::from_reader(&mut reader).map_err(|_| {
            other_error("Failed deserializing public spend key")
        })?;
        let pk_r = JubJubAffine::from_reader(&mut reader)
            .map_err(|_| other_error("Failed deserializing pk_r"))?
            .into();
        let message = Message::from_reader(&mut reader)
            .map_err(|_| other_error("Failed deserializing message"))?;
        let crossover_blinder = JubJubScalar::from_reader(&mut reader)
            .map_err(|_| {
                other_error("Failed deserializing crossover blinder")
            })?;
        let crossover = Crossover::from_reader(&mut reader)
            .map_err(|_| other_error("Failed deserializing crossover"))?;
        let fee = Fee::from_reader(&mut reader)
            .map_err(|_| other_error("Failed deserializing fee"))?;
        let contract_address =
            BlsScalar::from_reader(&mut reader).map_err(|_| {
                other_error("Failed deserializing contract address")
            })?;
        let signature = Signature::from_reader(&mut reader)
            .map_err(|_| other_error("Failed deserializing signature"))?;

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

        let (proof, _) = STCO_PROVER.prove(&mut OsRng, &circ).map_err(|e| {
            other_error(format!("Failed proving the circuit: {e}").as_str())
        })?;
        let proof = proof.to_bytes().to_vec();

        Ok(proof)
    }
}
