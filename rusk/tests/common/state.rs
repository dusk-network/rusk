// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::{path::Path, usize};

use dusk_bytes::Serializable;
use node::vm::VMExecution;
use rusk::node::RuskVmConfig;
use rusk::{Result, Rusk, DUSK_CONSENSUS_KEY};
use rusk_recovery_tools::state::{self, Snapshot};

use dusk_consensus::{
    config::{RATIFICATION_COMMITTEE_CREDITS, VALIDATION_COMMITTEE_CREDITS},
    operations::CallParams,
};
use dusk_core::{
    signatures::bls::PublicKey as BlsPublicKey, transfer::Transaction,
};
use node_data::{
    bls::PublicKeyBytes,
    ledger::{
        Attestation, Block, Header, IterationsInfo, Slash, SpentTransaction,
    },
    message::payload::Vote,
};

use tokio::sync::broadcast;
use tracing::info;

const CHAIN_ID: u8 = 0xFA;
pub const DEFAULT_GAS_PER_DEPLOY_BYTE: u64 = 100;
pub const DEFAULT_MIN_DEPLOYMENT_GAS_PRICE: u64 = 2000;
pub const DEFAULT_MIN_GAS_LIMIT: u64 = 75000;
pub const DEFAULT_MIN_DEPLOY_POINTS: u64 = 5000000;

// Creates a Rusk initial state in the given directory
pub fn new_state<P: AsRef<Path>>(
    dir: P,
    snapshot: &Snapshot,
    vm_config: RuskVmConfig,
) -> Result<Rusk> {
    new_state_with_chainid(dir, snapshot, vm_config, CHAIN_ID)
}

// Creates a Rusk initial state in the given directory
pub fn new_state_with_chainid<P: AsRef<Path>>(
    dir: P,
    snapshot: &Snapshot,
    vm_config: RuskVmConfig,
    chain_id: u8,
) -> Result<Rusk> {
    let dir = dir.as_ref();

    let (_, commit_id) =
        state::deploy(dir, snapshot, *DUSK_CONSENSUS_KEY, |_| {})
            .expect("Deploying initial state should succeed");

    let (sender, _) = broadcast::channel(10);

    let rusk = Rusk::new(
        dir,
        chain_id,
        None,
        vm_config,
        DEFAULT_MIN_GAS_LIMIT,
        u64::MAX,
        sender,
    )
    .expect("Instantiating rusk should succeed");

    assert_eq!(
        commit_id,
        rusk.state_root(),
        "The current commit should be the commit of the initial state"
    );
    assert_eq!(
        commit_id,
        rusk.base_root(),
        "The base commit should be the commit of the initial state"
    );

    Ok(rusk)
}

#[allow(dead_code)]
pub struct ExecuteResult {
    pub executed: usize,
    pub discarded: usize,
}

/// Executes the procedure a block generator will go through to generate a block
/// including all specified transactions, checking the outputs are as
/// expected. If not `expected` is specified, all txs must be included in the
/// block
#[allow(dead_code)]
pub fn generator_procedure(
    rusk: &Rusk,
    txs: &[Transaction],
    block_height: u64,
    block_gas_limit: u64,
    missed_generators: Vec<BlsPublicKey>,
    expected: Option<ExecuteResult>,
) -> anyhow::Result<Vec<SpentTransaction>> {
    let prev_root = rusk.state_root();
    let expected = expected.unwrap_or(ExecuteResult {
        executed: txs.len(),
        discarded: 0,
    });

    let txs: Vec<_> = txs.iter().map(|t| t.clone().into()).collect();
    for tx in &txs {
        rusk.preverify(tx)?;
    }

    let generator_pubkey = node_data::bls::PublicKey::new(*DUSK_CONSENSUS_KEY);
    let generator_pubkey_bytes = *generator_pubkey.bytes();
    let round = block_height;

    let mut failed_iterations = IterationsInfo::default();
    for to_slash in &missed_generators {
        failed_iterations.att_list.push(Some((
            Attestation {
                result: node_data::message::payload::RatificationResult::Fail(
                    Vote::NoCandidate,
                ),
                ..Default::default()
            },
            PublicKeyBytes(to_slash.to_bytes()),
        )));
    }

    let faults = vec![];

    let to_slash =
        Slash::from_iterations_and_faults(&failed_iterations, &faults)?;

    let voter = (generator_pubkey.clone(), 1);
    let voters_size =
        VALIDATION_COMMITTEE_CREDITS + RATIFICATION_COMMITTEE_CREDITS;
    let voters = vec![voter; voters_size];

    let call_params = CallParams {
        round,
        generator_pubkey,
        to_slash,
        voters_pubkey: voters.clone(),
        max_txs_bytes: usize::MAX,
        prev_state_root: prev_root,
    };

    let (transfer_txs, discarded, execute_output) =
        rusk.execute_state_transition(&call_params, txs.into_iter())?;

    assert_eq!(transfer_txs.len(), expected.executed, "all txs accepted");
    assert_eq!(discarded.len(), expected.discarded, "no discarded tx");

    info!(
        "execute_state_transition new verification: {}",
        execute_output
    );

    let txs: Vec<_> = transfer_txs.into_iter().map(|tx| tx.inner).collect();

    let block = Block::new(
        Header {
            height: block_height,
            gas_limit: block_gas_limit,
            generator_bls_pubkey: generator_pubkey_bytes,
            state_hash: execute_output.state_root,
            event_bloom: execute_output.event_bloom,
            failed_iterations,
            ..Default::default()
        },
        txs,
        vec![],
    )
    .expect("valid block");

    let verify_output =
        rusk.verify_state_transition(prev_root, &block, &voters)?;
    info!("verify_state_transition new verification: {verify_output}",);

    let (accept_txs, accept_output, _) =
        rusk.accept(prev_root, &block, &voters)?;

    assert_eq!(accept_txs.len(), expected.executed, "all txs accepted");

    info!(
        "accept block {} with new verification: {accept_output}",
        block_height,
    );

    assert_eq!(
        accept_output, execute_output,
        "Verification outputs should be equal"
    );

    Ok(accept_txs)
}

/// Executes the procedure a block generator will go through to generate a block
/// including all specified transactions, checking the outputs are as
/// expected. If not `expected` is specified, all txs must be included in the
/// block
#[allow(dead_code)]
pub fn generator_procedure2(
    rusk: &Rusk,
    txs: &[Transaction],
    block_height: u64,
    block_gas_limit: u64,
    missed_generators: Vec<BlsPublicKey>,
    expected: Option<ExecuteResult>,
    generator: Option<BlsPublicKey>,
) -> anyhow::Result<(Vec<SpentTransaction>, [u8; 32])> {
    let prev_root = rusk.state_root();
    let expected = expected.unwrap_or(ExecuteResult {
        executed: txs.len(),
        discarded: 0,
    });

    let txs: Vec<_> = txs.iter().map(|t| t.clone().into()).collect();
    for tx in &txs {
        rusk.preverify(tx)?;
    }

    let generator = generator.unwrap_or(*DUSK_CONSENSUS_KEY);
    let generator_pubkey = node_data::bls::PublicKey::new(generator);
    let generator_pubkey_bytes = *generator_pubkey.bytes();
    let round = block_height;

    let mut failed_iterations = IterationsInfo::default();
    for to_slash in &missed_generators {
        failed_iterations.att_list.push(Some((
            Attestation {
                result: node_data::message::payload::RatificationResult::Fail(
                    Vote::NoCandidate,
                ),
                ..Default::default()
            },
            PublicKeyBytes(to_slash.to_bytes()),
        )));
    }

    let faults = vec![];

    let to_slash =
        Slash::from_iterations_and_faults(&failed_iterations, &faults)?;

    let voter = (generator_pubkey.clone(), 1);
    let voters_size =
        VALIDATION_COMMITTEE_CREDITS + RATIFICATION_COMMITTEE_CREDITS;
    let voters = vec![voter; voters_size];

    let call_params = CallParams {
        round,
        generator_pubkey,
        to_slash,
        voters_pubkey: voters.clone(),
        max_txs_bytes: usize::MAX,
        prev_state_root: prev_root,
    };

    let (transfer_txs, discarded, execute_output) =
        rusk.execute_state_transition(&call_params, txs.into_iter())?;

    assert_eq!(transfer_txs.len(), expected.executed, "all txs accepted");
    assert_eq!(discarded.len(), expected.discarded, "no discarded tx");

    info!(
        "execute_state_transition new verification: {}",
        execute_output
    );

    let txs: Vec<_> = transfer_txs.into_iter().map(|tx| tx.inner).collect();

    let block = Block::new(
        Header {
            height: block_height,
            gas_limit: block_gas_limit,
            generator_bls_pubkey: generator_pubkey_bytes,
            state_hash: execute_output.state_root,
            event_bloom: execute_output.event_bloom,
            failed_iterations,
            ..Default::default()
        },
        txs,
        vec![],
    )
    .expect("valid block");

    let verify_output =
        rusk.verify_state_transition(prev_root, &block, &voters)?;
    info!("verify_state_transition new verification: {verify_output}",);

    let (accept_txs, accept_output, _) =
        rusk.accept(prev_root, &block, &voters)?;

    assert_eq!(accept_txs.len(), expected.executed, "all txs accepted");

    info!(
        "accept block {} with new verification: {accept_output}",
        block_height,
    );

    assert_eq!(
        accept_output, execute_output,
        "Verification outputs should be equal"
    );

    Ok((accept_txs, accept_output.state_root))
}
