// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;

use dusk_plonk::prelude::Prover;
use rand::rngs::OsRng;

const WFCT_INPUT_LEN: usize =
    JubJubAffine::SIZE + u64::SIZE + JubJubScalar::SIZE;

pub static WFCT_PROVER: Lazy<Prover<WithdrawFromTransparentCircuit>> =
    Lazy::new(|| {
        let keys = keys_for(&WithdrawFromTransparentCircuit::circuit_id())
            .expect("keys to be available");
        let pk = keys.get_prover().expect("prover to be available");
        Prover::try_from_bytes(pk).expect("prover key to be valid")
    });

impl RuskProver {
    pub(crate) fn prove_wfct(
        &self,
        request: &WfctProverRequest,
    ) -> Result<Response<WfctProverResponse>, Status> {
        let mut reader = &request.circuit_inputs[..];

        if reader.len() != WFCT_INPUT_LEN {
            return Err(Status::invalid_argument(format!(
                "Expected length {} got {}",
                WFCT_INPUT_LEN,
                reader.len()
            )));
        }

        let commitment = JubJubAffine::from_reader(&mut reader)
            .map_err(|_| {
                Status::invalid_argument("Failed deserializing commitment")
            })?
            .into();

        let value = u64::from_reader(&mut reader).map_err(|_| {
            Status::invalid_argument("Failed deserializing value")
        })?;

        let blinder = JubJubScalar::from_reader(&mut reader).map_err(|_| {
            Status::invalid_argument("Failed deserializing blinder")
        })?;

        let circ =
            WithdrawFromTransparentCircuit::new(commitment, value, blinder);

        let (proof, _) = WFCT_PROVER.prove(&mut OsRng, &circ).map_err(|e| {
            Status::internal(format!("Failed proving the circuit: {}", e))
        })?;
        let proof = proof.to_bytes().to_vec();

        Ok(Response::new(WfctProverResponse { proof }))
    }
}
