// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;

const WFCO_INPUT_LEN: usize = u64::SIZE
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
lazy_static! {
    static ref WFCO_PROVER_KEY: ProverKey = {
        let keys = keys_for(&WithdrawFromObfuscatedCircuit::CIRCUIT_ID)
            .expect("keys to be available");
        let pk = keys.get_prover().expect("prover to be available");
        ProverKey::from_slice(&pk).expect("prover key to be valid")
    };
}
impl Rusk {
    pub(crate) fn prove_wfco(
        &self,
        request: &WfcoProverRequest,
    ) -> Result<Response<WfcoProverResponse>, Status> {
        let mut reader = &request.circuit_inputs[..];

        if reader.len() != WFCO_INPUT_LEN {
            return Err(Status::invalid_argument(format!(
                "Expected length {} got {}",
                WFCO_INPUT_LEN,
                reader.len()
            )));
        }

        let input_value = u64::from_reader(&mut reader).map_err(|_| {
            Status::invalid_argument("Failed deserializing input value")
        })?;
        let input_blinder =
            JubJubScalar::from_reader(&mut reader).map_err(|_| {
                Status::invalid_argument("Failed deserializing input blinder")
            })?;
        let input_commitment = JubJubAffine::from_reader(&mut reader)
            .map_err(|_| {
                Status::invalid_argument("Failed deserializing input blinder")
            })?
            .into();

        let input = WfoCommitment {
            value: input_value,
            blinder: input_blinder,
            commitment: input_commitment,
        };

        let change_value = u64::from_reader(&mut reader).map_err(|_| {
            Status::invalid_argument("Failed deserializing change value")
        })?;
        let change_message =
            Message::from_reader(&mut reader).map_err(|_| {
                Status::invalid_argument("Failed deserializing change message")
            })?;
        let change_blinder =
            JubJubScalar::from_reader(&mut reader).map_err(|_| {
                Status::invalid_argument("Failed deserializing change blinder")
            })?;
        let r = JubJubScalar::from_reader(&mut reader).map_err(|_| {
            Status::invalid_argument("Failed deserializing change 'r'")
        })?;
        let is_public = u64::from_reader(&mut reader).map_err(|_| {
            Status::invalid_argument("Failed deserializing is_public")
        })? != 0;
        let psk = PublicSpendKey::from_reader(&mut reader).map_err(|_| {
            Status::invalid_argument("Failed deserializing public spend key")
        })?;
        let pk_r = JubJubAffine::from_reader(&mut reader)
            .map_err(|_| {
                Status::invalid_argument("Failed deserializing 'pk_r'")
            })?
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

        let output_value = u64::from_reader(&mut reader).map_err(|_| {
            Status::invalid_argument("Failed deserializing output value")
        })?;
        let output_blinder =
            JubJubScalar::from_reader(&mut reader).map_err(|_| {
                Status::invalid_argument("Failed deserializing output blinder")
            })?;
        let output_commitment = JubJubAffine::from_reader(&mut reader)
            .map_err(|_| {
                Status::invalid_argument("Failed deserializing output blinder")
            })?
            .into();

        let output = WfoCommitment {
            value: output_value,
            blinder: output_blinder,
            commitment: output_commitment,
        };

        let mut circ = WithdrawFromObfuscatedCircuit {
            input,
            change,
            output,
        };

        let proof = circ
            .prove(&*crate::PUB_PARAMS, &WFCO_PROVER_KEY, b"dusk-network")
            .map_err(|e| {
                Status::internal(format!("Failed proving the circuit: {}", e))
            })?;
        let proof = proof.to_bytes().to_vec();

        Ok(Response::new(WfcoProverResponse { proof }))
    }
}
