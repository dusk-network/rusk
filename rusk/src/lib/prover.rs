// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Prover service implementation for the Rusk server.

use rusk_prover::Prover;

use crate::error::Error;
use crate::Result;

use dusk_wallet_core::Transaction;
use phoenix_core::transaction::TRANSFER_TREE_DEPTH;
use rusk_profile::keys_for;

use transfer_circuits::{CircuitOutput, ExecuteCircuit};

const A: usize = 4;

#[derive(Debug, Default)]
pub struct RuskProver<P: Prover>(P);

pub fn verify_proof(tx: &Transaction) -> Result<bool> {
    let tx_hash = rusk_abi::hash(tx.to_hash_input_bytes());

    let inputs = &tx.nullifiers;
    let outputs = &tx.outputs;
    let proof = &tx.proof;

    let circuit = circuit_from_numbers(inputs.len(), outputs.len())
        .ok_or_else(|| {
            Error::InvalidCircuitArguments(inputs.len(), outputs.len())
        })?;

    let mut pi: Vec<rusk_abi::PublicInput> =
        Vec::with_capacity(9 + inputs.len());

    pi.push(tx_hash.into());
    pi.push(tx.anchor.into());
    pi.extend(inputs.iter().map(|n| n.into()));

    pi.push(
        tx.crossover()
            .copied()
            .unwrap_or_default()
            .value_commitment()
            .into(),
    );

    let fee_value = tx.fee().gas_limit * tx.fee().gas_price;

    pi.push(fee_value.into());
    pi.extend(outputs.iter().map(|n| n.value_commitment().into()));
    pi.extend(
        (0usize..2usize.saturating_sub(outputs.len()))
            .map(|_| CircuitOutput::ZERO_COMMITMENT.into()),
    );

    let keys = keys_for(circuit.circuit_id())?;
    let vd = keys.get_verifier()?;

    // Maybe we want to handle internal serialization error too, currently
    // they map to `false`.
    Ok(rusk_abi::verify_proof(vd, proof.clone(), pi))
}

pub(crate) fn circuit_from_numbers(
    num_inputs: usize,
    num_outputs: usize,
) -> Option<ExecuteCircuit<(), TRANSFER_TREE_DEPTH, A>> {
    use ExecuteCircuit::*;

    match num_inputs {
        1 if num_outputs < 3 => Some(OneTwo(Default::default())),
        2 if num_outputs < 3 => Some(TwoTwo(Default::default())),
        3 if num_outputs < 3 => Some(ThreeTwo(Default::default())),
        4 if num_outputs < 3 => Some(FourTwo(Default::default())),
        _ => None,
    }
}
