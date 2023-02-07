// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;

use dusk_plonk::prelude::Prover;
use rand::rngs::OsRng;

const STCT_INPUT_LEN: usize = Fee::SIZE
    + Crossover::SIZE
    + u64::SIZE
    + JubJubScalar::SIZE
    + BlsScalar::SIZE
    + Signature::SIZE;

pub static STCT_PROVER: Lazy<Prover<SendToContractTransparentCircuit>> =
    Lazy::new(|| {
        let keys = keys_for(&SendToContractTransparentCircuit::circuit_id())
            .expect("keys to be available");
        let pk = keys.get_prover().expect("prover to be available");
        Prover::try_from_bytes(pk).expect("prover key to be valid")
    });

impl RuskProver {
    pub(crate) fn prove_stct(
        &self,
        request: &StctProverRequest,
    ) -> Result<Response<StctProverResponse>, Status> {
        let mut reader = &request.circuit_inputs[..];

        if reader.len() != STCT_INPUT_LEN {
            return Err(Status::invalid_argument(format!(
                "Expected length {} got {}",
                STCT_INPUT_LEN,
                reader.len()
            )));
        }

        let fee = Fee::from_reader(&mut reader).map_err(|_| {
            Status::invalid_argument("Failed deserializing fee")
        })?;
        let crossover = Crossover::from_reader(&mut reader).map_err(|_| {
            Status::invalid_argument("Failed deserializing crossover")
        })?;
        let crossover_value = u64::from_reader(&mut reader).map_err(|_| {
            Status::invalid_argument("Failed deserializing crossover value")
        })?;
        let crossover_blinder = JubJubScalar::from_reader(&mut reader)
            .map_err(|_| {
                Status::invalid_argument("Failed deserializing crossover value")
            })?;
        let contract_address =
            BlsScalar::from_reader(&mut reader).map_err(|_| {
                Status::invalid_argument(
                    "Failed deserializing contract address",
                )
            })?;
        let signature = Signature::from_reader(&mut reader).map_err(|_| {
            Status::invalid_argument("Failed deserializing signature")
        })?;

        let circ = SendToContractTransparentCircuit::new(
            &fee,
            &crossover,
            crossover_value,
            crossover_blinder,
            contract_address,
            signature,
        );

        let (proof, _) = STCT_PROVER.prove(&mut OsRng, &circ).map_err(|e| {
            Status::internal(format!("Failed proving the circuit: {}", e))
        })?;
        let proof = proof.to_bytes().to_vec();

        Ok(Response::new(StctProverResponse { proof }))
    }
}
