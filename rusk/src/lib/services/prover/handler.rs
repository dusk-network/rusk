// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::{rusk_proto, ServiceRequestHandler};

use std::collections::HashMap;

use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_pki::PublicSpendKey;
use dusk_plonk::prelude::*;
use dusk_schnorr::Signature;
use dusk_wallet_core::UnprovenTransaction;
use once_cell::sync::Lazy;
use phoenix_core::{Crossover, Fee, Message};
use rusk_profile::keys_for;
use rusk_proto::{
    ExecuteProverRequest, ExecuteProverResponse, StcoProverRequest,
    StcoProverResponse, StctProverRequest, StctProverResponse,
    WfcoProverRequest, WfcoProverResponse, WfctProverRequest,
    WfctProverResponse,
};
use tonic::{Request, Response, Status};
use transfer_circuits::{
    CircuitInput, CircuitInputSignature, DeriveKey, ExecuteCircuit,
    SendToContractObfuscatedCircuit, SendToContractTransparentCircuit,
    StcoCrossover, StcoMessage, WfoChange, WfoCommitment,
    WithdrawFromObfuscatedCircuit, WithdrawFromTransparentCircuit,
};

use crate::PUB_PARAMS;

pub static EXECUTE_PROVER_KEYS: Lazy<HashMap<(usize, usize), ProverKey>> =
    Lazy::new(|| {
        let mut map = HashMap::new();

        for ninputs in [1, 2, 3, 4] {
            for noutputs in [0, 1, 2] {
                let circ = circuit_from_numbers(ninputs, noutputs)
                    .expect("circuit to exist");

                let keys =
                    keys_for(circ.circuit_id()).expect("keys to be available");
                let pk = keys.get_prover().expect("prover to be available");
                let pk =
                    ProverKey::from_slice(&pk).expect("prover key to be valid");

                map.insert((ninputs, noutputs), pk);
            }
        }

        map
    });

pub static WFCT_PROVER_KEY: Lazy<ProverKey> = Lazy::new(|| {
    let keys = keys_for(&WithdrawFromTransparentCircuit::CIRCUIT_ID)
        .expect("keys to be available");
    let pk = keys.get_prover().expect("prover to be available");
    ProverKey::from_slice(&pk).expect("prover key to be valid")
});

pub static WFCO_PROVER_KEY: Lazy<ProverKey> = Lazy::new(|| {
    let keys = keys_for(&WithdrawFromObfuscatedCircuit::CIRCUIT_ID)
        .expect("keys to be available");
    let pk = keys.get_prover().expect("prover to be available");
    ProverKey::from_slice(&pk).expect("prover key to be valid")
});

pub static STCT_PROVER_KEY: Lazy<ProverKey> = Lazy::new(|| {
    let keys = keys_for(&SendToContractTransparentCircuit::CIRCUIT_ID)
        .expect("keys to be available");
    let pk = keys.get_prover().expect("prover to be available");
    ProverKey::from_slice(&pk).expect("prover key to be valid")
});

pub static STCO_PROVER_KEY: Lazy<ProverKey> = Lazy::new(|| {
    let keys = keys_for(&SendToContractObfuscatedCircuit::CIRCUIT_ID)
        .expect("keys to be available");
    let pk = keys.get_prover().expect("prover to be available");
    ProverKey::from_slice(&pk).expect("prover key to be valid")
});

pub struct ExecuteProverHandler<'a> {
    _request: &'a Request<ExecuteProverRequest>,
}

impl<'a, 'b>
    ServiceRequestHandler<'a, 'b, ExecuteProverRequest, ExecuteProverResponse>
    for ExecuteProverHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<ExecuteProverRequest>) -> Self {
        Self { _request: request }
    }

    fn handle_request(
        &self,
    ) -> Result<Response<ExecuteProverResponse>, Status> {
        let utx = UnprovenTransaction::from_slice(&self._request.get_ref().utx)
            .map_err(|_| {
                Status::invalid_argument("Failed parsing unproven TX")
            })?;

        let (num_inputs, num_outputs) =
            (utx.inputs().len(), utx.outputs().len());
        let mut circ = circuit_from_numbers(num_inputs, num_outputs)
            .ok_or_else(|| {
                Status::invalid_argument(format!(
                    "No circuit found for number of inputs {} and outputs {}",
                    num_inputs, num_outputs
                ))
            })?;

        for input in utx.inputs() {
            let cis = CircuitInputSignature::from(input.signature());
            let cinput = CircuitInput::new(
                input.opening().clone(),
                *input.note(),
                input.pk_r_prime().into(),
                input.value(),
                input.blinding_factor(),
                input.nullifier(),
                cis,
            );
            circ.add_input(cinput).map_err(|e| {
                Status::internal(format!(
                    "Failed adding input to circuit: {}",
                    e
                ))
            })?;
        }
        for (note, value, blinder) in utx.outputs() {
            circ.add_output_with_data(*note, *value, *blinder).map_err(
                |e| {
                    Status::internal(format!(
                        "Failed adding output to circuit: {}",
                        e
                    ))
                },
            )?;
        }

        circ.set_tx_hash(utx.hash());
        circ.set_fee(utx.fee()).map_err(|e| {
            Status::invalid_argument(format!("Failed setting fee: {}", e))
        })?;

        let (crossover, value, blinder) = utx.crossover();
        circ.set_fee_crossover(utx.fee(), crossover, *value, *blinder);

        let proof = circ
            .prove(
                &PUB_PARAMS,
                EXECUTE_PROVER_KEYS.get(&(num_inputs, num_outputs)).unwrap(),
            )
            .map_err(|e| {
                Status::invalid_argument(format!(
                    "Failed proving transaction: {}",
                    e
                ))
            })?;
        let proof = proof.to_bytes().to_vec();

        Ok(Response::new(ExecuteProverResponse { proof }))
    }
}

pub struct StctProverHandler<'a> {
    _request: &'a Request<StctProverRequest>,
}

const STCT_INPUT_LEN: usize = Fee::SIZE
    + Crossover::SIZE
    + u64::SIZE
    + JubJubScalar::SIZE
    + BlsScalar::SIZE
    + Signature::SIZE;

impl<'a, 'b>
    ServiceRequestHandler<'a, 'b, StctProverRequest, StctProverResponse>
    for StctProverHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<StctProverRequest>) -> Self {
        Self { _request: request }
    }

    fn handle_request(&self) -> Result<Response<StctProverResponse>, Status> {
        let mut reader = &self._request.get_ref().circuit_inputs[..];

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

        let mut circ = SendToContractTransparentCircuit::new(
            &fee,
            &crossover,
            crossover_value,
            crossover_blinder,
            contract_address,
            signature,
        );

        let proof = circ
            .prove(&PUB_PARAMS, &STCT_PROVER_KEY, b"dusk-network")
            .map_err(|e| {
                Status::internal(format!("Failed proving the circuit: {}", e))
            })?;
        let proof = proof.to_bytes().to_vec();

        Ok(Response::new(StctProverResponse { proof }))
    }
}

pub struct StcoProverHandler<'a> {
    _request: &'a Request<StcoProverRequest>,
}

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

impl<'a, 'b>
    ServiceRequestHandler<'a, 'b, StcoProverRequest, StcoProverResponse>
    for StcoProverHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<StcoProverRequest>) -> Self {
        Self { _request: request }
    }

    fn handle_request(&self) -> Result<Response<StcoProverResponse>, Status> {
        let mut reader = &self._request.get_ref().circuit_inputs[..];

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
            .prove(&PUB_PARAMS, &STCO_PROVER_KEY, b"dusk-network")
            .map_err(|e| {
                Status::internal(format!("Failed proving the circuit: {}", e))
            })?;
        let proof = proof.to_bytes().to_vec();

        Ok(Response::new(StcoProverResponse { proof }))
    }
}

pub struct WfctProverHandler<'a> {
    _request: &'a Request<WfctProverRequest>,
}

const WFCT_INPUT_LEN: usize =
    JubJubAffine::SIZE + u64::SIZE + JubJubScalar::SIZE;

impl<'a, 'b>
    ServiceRequestHandler<'a, 'b, WfctProverRequest, WfctProverResponse>
    for WfctProverHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<WfctProverRequest>) -> Self {
        Self { _request: request }
    }

    fn handle_request(&self) -> Result<Response<WfctProverResponse>, Status> {
        let mut reader = &self._request.get_ref().circuit_inputs[..];

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

        let mut circ =
            WithdrawFromTransparentCircuit::new(commitment, value, blinder);

        let proof = circ
            .prove(&PUB_PARAMS, &WFCT_PROVER_KEY, b"dusk-network")
            .map_err(|e| {
                Status::internal(format!("Failed proving the circuit: {}", e))
            })?;
        let proof = proof.to_bytes().to_vec();

        Ok(Response::new(WfctProverResponse { proof }))
    }
}

pub struct WfcoProverHandler<'a> {
    _request: &'a Request<WfcoProverRequest>,
}

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

impl<'a, 'b>
    ServiceRequestHandler<'a, 'b, WfcoProverRequest, WfcoProverResponse>
    for WfcoProverHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<WfcoProverRequest>) -> Self {
        Self { _request: request }
    }

    fn handle_request(&self) -> Result<Response<WfcoProverResponse>, Status> {
        let mut reader = &self._request.get_ref().circuit_inputs[..];

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
            .prove(&PUB_PARAMS, &WFCO_PROVER_KEY, b"dusk-network")
            .map_err(|e| {
                Status::internal(format!("Failed proving the circuit: {}", e))
            })?;
        let proof = proof.to_bytes().to_vec();

        Ok(Response::new(WfcoProverResponse { proof }))
    }
}

fn circuit_from_numbers(
    num_inputs: usize,
    num_outputs: usize,
) -> Option<ExecuteCircuit> {
    use ExecuteCircuit::*;

    match (num_inputs, num_outputs) {
        (1, 0) => Some(ExecuteCircuitOneZero(Default::default())),
        (1, 1) => Some(ExecuteCircuitOneOne(Default::default())),
        (1, 2) => Some(ExecuteCircuitOneTwo(Default::default())),
        (2, 0) => Some(ExecuteCircuitTwoZero(Default::default())),
        (2, 1) => Some(ExecuteCircuitTwoOne(Default::default())),
        (2, 2) => Some(ExecuteCircuitTwoTwo(Default::default())),
        (3, 0) => Some(ExecuteCircuitThreeZero(Default::default())),
        (3, 1) => Some(ExecuteCircuitThreeOne(Default::default())),
        (3, 2) => Some(ExecuteCircuitThreeTwo(Default::default())),
        (4, 0) => Some(ExecuteCircuitFourZero(Default::default())),
        (4, 1) => Some(ExecuteCircuitFourOne(Default::default())),
        (4, 2) => Some(ExecuteCircuitFourTwo(Default::default())),
        _ => None,
    }
}
