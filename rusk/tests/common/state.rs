// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::Path;

use dusk_bytes::Serializable;
use node::vm::VMExecution;
use rusk::{Result, Rusk};
use rusk_recovery_tools::state::{self, Snapshot};

use dusk_bls12_381_sign::PublicKey;
use dusk_consensus::operations::CallParams;
use dusk_wallet_core::Transaction as PhoenixTransaction;
use node_data::{
    bls::PublicKeyBytes,
    ledger::{Block, Certificate, Header, IterationsInfo, SpentTransaction},
    message::payload::Vote,
};
use tracing::info;

use crate::common::keys::BLS_SK;

// Creates a Rusk initial state in the given directory
pub fn new_state<P: AsRef<Path>>(dir: P, snapshot: &Snapshot) -> Result<Rusk> {
    let dir = dir.as_ref();

    let (_, commit_id) = state::deploy(dir, snapshot, |_| {})
        .expect("Deploying initial state should succeed");

    let rusk = Rusk::new(dir, None).expect("Instantiating rusk should succeed");

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

pub struct ExecuteResult {
    pub executed: usize,
    pub discarded: usize,
}

/// Executes the procedure a block generator will go through to generate a block
/// including all specified transactions, checking the outputs are as
/// expected. If not `expected` is specified, all txs must be included in the
/// block
pub fn generator_procedure(
    rusk: &Rusk,
    txs: &[PhoenixTransaction],
    block_height: u64,
    block_gas_limit: u64,
    missed_generators: Vec<PublicKey>,
    expected: Option<ExecuteResult>,
) -> anyhow::Result<Vec<SpentTransaction>> {
    let expected = expected.unwrap_or(ExecuteResult {
        executed: txs.len(),
        discarded: 0,
    });

    let txs: Vec<_> = txs.iter().map(|t| t.clone().into()).collect();
    for tx in &txs {
        rusk.preverify(tx)?;
    }

    let generator = PublicKey::from(&*BLS_SK);
    let generator_pubkey = node_data::bls::PublicKey::new(generator);
    let generator_pubkey_bytes = *generator_pubkey.bytes();
    let round = block_height;

    let mut failed_iterations = IterationsInfo::default();
    for to_slash in &missed_generators {
        failed_iterations.cert_list.push(Some((
            Certificate {
                result: node_data::message::payload::RatificationResult::Fail(
                    Vote::NoCandidate,
                ),
                ..Default::default()
            },
            PublicKeyBytes(to_slash.to_bytes()),
        )));
    }

    let call_params = CallParams {
        round,
        block_gas_limit,
        generator_pubkey,
        missed_generators,
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
            event_hash: execute_output.event_hash,
            failed_iterations,
            ..Default::default()
        },
        txs,
    )
    .expect("valid block");

    let verify_output = rusk.verify_state_transition(&block)?;
    info!("verify_state_transition new verification: {verify_output}",);

    let (accept_txs, accept_output) = rusk.accept(&block)?;

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
