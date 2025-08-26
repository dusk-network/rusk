// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::{path::Path, usize};

#[cfg(feature = "archive")]
use node::archive::Archive;
#[cfg(feature = "archive")]
use tempfile::tempdir;

use dusk_bytes::Serializable;
use node::vm::VMExecution;
use rusk::node::RuskVmConfig;
use rusk::{Result, Rusk, DUSK_CONSENSUS_KEY};
use rusk_recovery_tools::state::{self, Snapshot};

use dusk_consensus::{
    config::{RATIFICATION_COMMITTEE_CREDITS, VALIDATION_COMMITTEE_CREDITS},
    operations::StateTransitionData,
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
pub const DEFAULT_MIN_GAS_LIMIT: u64 = 75000;
pub const DEFAULT_DRIVER_STORE_LIMIT: u64 = 1024;

// Creates a Rusk initial state in the given directory
pub async fn new_state<P: AsRef<Path>>(
    dir: P,
    snapshot: &Snapshot,
    vm_config: RuskVmConfig,
) -> Result<Rusk> {
    new_state_with_chainid(dir, snapshot, vm_config, CHAIN_ID).await
}

// Creates a Rusk initial state in the given directory
pub async fn new_state_with_chainid<P: AsRef<Path>>(
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

    #[cfg(feature = "archive")]
    let archive_dir =
        tempdir().expect("Should be able to create temporary directory");
    #[cfg(feature = "archive")]
    let archive = Archive::create_or_open(archive_dir.path()).await;

    let rusk = Rusk::new(
        dir,
        chain_id,
        vm_config,
        DEFAULT_MIN_GAS_LIMIT,
        u64::MAX,
        sender,
        #[cfg(feature = "archive")]
        archive,
        DriverStore::new(None, DEFAULT_DRIVER_STORE_LIMIT)
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
    let prev_state = rusk.state_root();
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

    let slashes =
        Slash::from_iterations_and_faults(&failed_iterations, &faults)?;

    let voter = (generator_pubkey.clone(), 1);
    let voters_size =
        VALIDATION_COMMITTEE_CREDITS + RATIFICATION_COMMITTEE_CREDITS;
    let cert_voters = vec![voter; voters_size];

    let transition_data = StateTransitionData {
        round,
        generator: generator_pubkey,
        slashes,
        cert_voters: cert_voters.clone(),
        max_txs_bytes: usize::MAX,
        prev_state_root: prev_state,
    };

    let (executed_txs, discarded_txs, transition_result) =
        rusk.create_state_transition(&transition_data, txs.into_iter())?;

    assert_eq!(executed_txs.len(), expected.executed, "all txs accepted");
    assert_eq!(discarded_txs.len(), expected.discarded, "no discarded tx");

    info!("create_state_transition result: {transition_result}");

    let txs: Vec<_> = executed_txs.into_iter().map(|tx| tx.inner).collect();

    let block = Block::new(
        Header {
            height: block_height,
            gas_limit: block_gas_limit,
            generator_bls_pubkey: generator_pubkey_bytes,
            state_hash: transition_result.state_root,
            event_bloom: transition_result.event_bloom,
            failed_iterations,
            ..Default::default()
        },
        txs,
        vec![],
    )
    .expect("valid block");

    // Execute, verify, and persist state transition
    let (accept_txs, _) =
        rusk.accept_state_transition(prev_state, &block, &cert_voters)?;

    assert_eq!(accept_txs.len(), expected.executed, "all txs accepted");

    info!("accept_state_transition (block {block_height}) result: {transition_result}");

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
    let prev_state = rusk.state_root();
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

    let slashes =
        Slash::from_iterations_and_faults(&failed_iterations, &faults)?;

    let voter = (generator_pubkey.clone(), 1);
    let voters_size =
        VALIDATION_COMMITTEE_CREDITS + RATIFICATION_COMMITTEE_CREDITS;
    let cert_voters = vec![voter; voters_size];

    let transition_data = StateTransitionData {
        round,
        generator: generator_pubkey,
        slashes,
        cert_voters: cert_voters.clone(),
        max_txs_bytes: usize::MAX,
        prev_state_root: prev_state,
    };

    let (transfer_txs, discarded, transition_result) =
        rusk.create_state_transition(&transition_data, txs.into_iter())?;

    assert_eq!(transfer_txs.len(), expected.executed, "all txs accepted");
    assert_eq!(discarded.len(), expected.discarded, "no discarded tx");

    info!("create_state_transition result: {transition_result}");

    let txs: Vec<_> = transfer_txs.into_iter().map(|tx| tx.inner).collect();

    let block = Block::new(
        Header {
            height: block_height,
            gas_limit: block_gas_limit,
            generator_bls_pubkey: generator_pubkey_bytes,
            state_hash: transition_result.state_root,
            event_bloom: transition_result.event_bloom,
            failed_iterations,
            ..Default::default()
        },
        txs,
        vec![],
    )
    .expect("valid block");

    let (accept_txs, _) =
        rusk.accept_state_transition(prev_state, &block, &cert_voters)?;

    assert_eq!(accept_txs.len(), expected.executed, "all txs accepted");

    info!("accept_state_transition (block {block_height}) result: {transition_result}");

    Ok((accept_txs, transition_result.state_root))
}
