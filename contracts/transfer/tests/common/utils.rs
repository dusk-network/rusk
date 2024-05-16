// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::mpsc;

use dusk_bls12_381::BlsScalar;
use dusk_plonk::prelude::*;
use phoenix_core::transaction::*;
use phoenix_core::{Note, ViewKey};
use poseidon_merkle::Opening as PoseidonOpening;
use rusk_abi::{
    ContractError, ContractId, EconomicMode, Error, Session, TRANSFER_CONTRACT,
};

const POINT_LIMIT: u64 = 0x10_000_000;

pub struct ExecutionResult {
    pub gas_spent: u64,
    pub economic_mode: EconomicMode,
}

impl ExecutionResult {
    pub const fn new(gas_spent: u64, economic_mode: EconomicMode) -> Self {
        Self {
            gas_spent,
            economic_mode,
        }
    }
}

pub fn leaves_from_height(
    session: &mut Session,
    height: u64,
) -> Result<Vec<TreeLeaf>, Error> {
    let (feeder, receiver) = mpsc::channel();

    session.feeder_call::<_, ()>(
        TRANSFER_CONTRACT,
        "leaves_from_height",
        &height,
        POINT_LIMIT,
        feeder,
    )?;

    Ok(receiver
        .iter()
        .map(|bytes| rkyv::from_bytes(&bytes).expect("Should return leaves"))
        .collect())
}

pub fn leaves_from_pos(
    session: &mut Session,
    pos: u64,
) -> Result<Vec<TreeLeaf>, Error> {
    let (feeder, receiver) = mpsc::channel();

    session.feeder_call::<_, ()>(
        TRANSFER_CONTRACT,
        "leaves_from_pos",
        &pos,
        POINT_LIMIT,
        feeder,
    )?;

    Ok(receiver
        .iter()
        .map(|bytes| rkyv::from_bytes(&bytes).expect("Should return leaves"))
        .collect())
}

pub fn num_notes(session: &mut Session) -> Result<u64, Error> {
    session
        .call(TRANSFER_CONTRACT, "num_notes", &(), u64::MAX)
        .map(|r| r.data)
}

pub fn update_root(session: &mut Session) -> Result<(), Error> {
    session
        .call(TRANSFER_CONTRACT, "update_root", &(), POINT_LIMIT)
        .map(|r| r.data)
}

pub fn root(session: &mut Session) -> Result<BlsScalar, Error> {
    session
        .call(TRANSFER_CONTRACT, "root", &(), POINT_LIMIT)
        .map(|r| r.data)
}

pub fn module_balance(
    session: &mut Session,
    contract: ContractId,
) -> Result<u64, Error> {
    session
        .call(TRANSFER_CONTRACT, "module_balance", &contract, POINT_LIMIT)
        .map(|r| r.data)
}

pub fn opening(
    session: &mut Session,
    pos: u64,
) -> Result<Option<PoseidonOpening<(), TRANSFER_TREE_DEPTH, 4>>, Error> {
    session
        .call(TRANSFER_CONTRACT, "opening", &pos, POINT_LIMIT)
        .map(|r| r.data)
}

pub fn prover_verifier(circuit_name: &str) -> (Prover, Verifier) {
    let circuit_profile = rusk_profile::Circuit::from_name(circuit_name)
        .expect(&format!(
            "There should be circuit data stored for {}",
            circuit_name
        ));
    let (pk, vd) = circuit_profile
        .get_keys()
        .expect(&format!("there should be keys stored for {}", circuit_name));

    let prover = Prover::try_from_bytes(pk).unwrap();
    let verifier = Verifier::try_from_bytes(vd).unwrap();

    (prover, verifier)
}

/// Executes a regular (not call-only) transaction.
/// Returns result containing gas spent and economic mode.
pub fn execute(
    session: &mut Session,
    tx: Transaction,
) -> Result<ExecutionResult, Error> {
    let receipt = session.call::<_, Result<Vec<u8>, ContractError>>(
        TRANSFER_CONTRACT,
        "spend_and_execute",
        &tx,
        u64::MAX,
    )?;

    let gas_spent = receipt.gas_spent;

    session
        .call::<_, ()>(
            TRANSFER_CONTRACT,
            "refund",
            &(tx.fee, gas_spent),
            u64::MAX,
        )
        .expect("Refunding must succeed");

    Ok(ExecutionResult::new(gas_spent, receipt.economic_mode))
}

/// Executes a call-only transaction.
/// Returns result containing gas spent and economic mode.
pub fn execute_call(
    session: &mut Session,
    tx: Transaction,
) -> Result<ExecutionResult, Error> {
    let receipt = session.call::<_, Result<Vec<u8>, ContractError>>(
        TRANSFER_CONTRACT,
        "execute",
        &tx,
        u64::MAX,
    )?;

    Ok(ExecutionResult::new(
        receipt.gas_spent,
        receipt.economic_mode,
    ))
}

/// Returns vector of notes owned by a given view key.
pub fn filter_notes_owned_by<I: IntoIterator<Item = Note>>(
    vk: ViewKey,
    iter: I,
) -> Vec<Note> {
    iter.into_iter().filter(|note| vk.owns(note)).collect()
}
