// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::Path;

use node::vm::VMExecution;
use rusk::{Result, Rusk};
use rusk_recovery_tools::state::{self, Snapshot};

use dusk_bls12_381_sign::PublicKey;
use dusk_consensus::contract_state::CallParams;
use dusk_wallet_core::Transaction as PhoenixTransaction;
use node_data::bls::PublicKeyBytes;
use node_data::ledger::{Block, SpentTransaction, Transaction};
use tracing::info;

use crate::common::keys::BLS_SK;

// Creates a Rusk initial state in the given directory
pub fn new_state<P: AsRef<Path>>(dir: P, snapshot: &Snapshot) -> Result<Rusk> {
    let dir = dir.as_ref();

    let (_, commit_id) = state::deploy(dir, snapshot)
        .expect("Deploying initial state should succeed");

    let rusk = Rusk::new(dir).expect("Instantiating rusk should succeed");

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
    expected: Option<ExecuteResult>,
) -> anyhow::Result<Vec<SpentTransaction>> {
    let expected = expected.unwrap_or(ExecuteResult {
        executed: txs.len(),
        discarded: 0,
    });

    let txs: Vec<_> = txs
        .iter()
        .map(|t| Transaction {
            inner: t.clone(),
            r#type: 1,
            version: 1,
        })
        .collect();
    for tx in &txs {
        rusk.preverify(tx)?;
    }

    let generator = PublicKey::from(&*BLS_SK);
    let generator_pubkey = node_data::bls::PublicKey::new(generator);
    let generator_pubkey_bytes = PublicKeyBytes(*generator_pubkey.bytes());
    let round = block_height;
    // let txs = vec![];

    let (transfer_txs, discarded, execute_state_root) = rusk
        .execute_state_transition(CallParams {
            txs,
            round,
            block_gas_limit,
            generator_pubkey: generator_pubkey.clone(),
        })
        .expect("msg");

    assert_eq!(transfer_txs.len(), expected.executed, "all txs accepted");
    assert_eq!(discarded.len(), expected.discarded, "no discarded tx");

    info!(
        "execute_state_transition new root: {:?}",
        hex::encode(execute_state_root)
    );

    let txs: Vec<_> = transfer_txs.into_iter().map(|tx| tx.inner).collect();
    let verify_param = CallParams {
        round,
        txs,
        block_gas_limit,
        generator_pubkey,
    };
    let verify_root = rusk.verify_state_transition(&verify_param)?;
    info!(
        "verify_state_transition new root: {:?}",
        hex::encode(verify_root)
    );

    let mut block = Block::default();
    block.header.generator_bls_pubkey = generator_pubkey_bytes;
    block.header.gas_limit = block_gas_limit;
    block.header.height = block_height;
    block.header.state_hash = execute_state_root;
    block.txs = verify_param.txs;

    let (accept_txs, accept_state_root) = rusk.accept(&block)?;

    assert_eq!(accept_txs.len(), expected.executed, "all txs accepted");

    info!(
        "accept block {} with new root: {:?}",
        block_height,
        hex::encode(accept_state_root)
    );

    assert_eq!(
        accept_state_root, execute_state_root,
        "Root should be equal"
    );

    Ok(accept_txs)
}
