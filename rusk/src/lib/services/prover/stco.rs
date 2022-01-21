// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;

const STCO_INPUT_LEN: usize = u64::SIZE
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
lazy_static! {
    static ref STCO_PROVER_KEY: ProverKey = {
        let keys = keys_for(&SendToContractObfuscatedCircuit::CIRCUIT_ID)
            .expect("keys to be available");
        let pk = keys.get_prover().expect("prover to be available");
        ProverKey::from_slice(&pk).expect("prover key to be valid")
    };
}
impl Rusk {
    pub(crate) fn prove_stco(
        &self,
        request: &StcoProverRequest,
    ) -> Result<Response<StcoProverResponse>, Status> {
        let mut reader = &request.circuit_inputs[..];

        if reader.len() != STCO_INPUT_LEN {
            return Err(Status::invalid_argument(format!(
                "Expected length {} got {}",
                STCO_INPUT_LEN,
                reader.len()
            )));
        }

        let value = u64::from_reader(&mut reader).map_err(|_| {
            Status::invalid_argument("Failed deserializing value")
        })?;
        let r = JubJubScalar::from_reader(&mut reader).map_err(|_| {
            Status::invalid_argument("Failed deserializing 'r'")
        })?;
        let blinder = JubJubScalar::from_reader(&mut reader).map_err(|_| {
            Status::invalid_argument("Failed deserializing blinder")
        })?;
        let is_public = u64::from_reader(&mut reader).map_err(|_| {
            Status::invalid_argument("Failed deserializing is_public")
        })? != 0;
        let psk = PublicSpendKey::from_reader(&mut reader).map_err(|_| {
            Status::invalid_argument("Failed deserializing public spend key")
        })?;
        let pk_r = JubJubAffine::from_reader(&mut reader)
            .map_err(|_| Status::invalid_argument("Failed deserializing pk_r"))?
            .into();
        let message = Message::from_reader(&mut reader).map_err(|_| {
            Status::invalid_argument("Failed deserializing message")
        })?;
        let crossover_blinder = JubJubScalar::from_reader(&mut reader)
            .map_err(|_| {
                Status::invalid_argument(
                    "Failed deserializing crossover blinder",
                )
            })?;
        let crossover = Crossover::from_reader(&mut reader).map_err(|_| {
            Status::invalid_argument("Failed deserializing crossover")
        })?;
        let fee = Fee::from_reader(&mut reader).map_err(|_| {
            Status::invalid_argument("Failed deserializing fee")
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

        let derive_key = DeriveKey::new(is_public, &psk);

        let stco_message = StcoMessage {
            r,
            blinder,
            derive_key,
            pk_r,
            message,
        };
        let stco_crossover = StcoCrossover::new(crossover, crossover_blinder);

        let mut circ = SendToContractObfuscatedCircuit::new(
            value,
            stco_message,
            stco_crossover,
            &fee,
            contract_address,
            signature,
        );

        let proof = circ
            .prove(&*crate::PUB_PARAMS, &STCO_PROVER_KEY, b"dusk-network")
            .map_err(|e| {
                Status::internal(format!("Failed proving the circuit: {}", e))
            })?;
        let proof = proof.to_bytes().to_vec();

        Ok(Response::new(StcoProverResponse { proof }))
    }
}
