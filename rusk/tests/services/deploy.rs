// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::BTreeMap;
use std::path::Path;

use rusk::{Result, Rusk};
use serde_json::Value;
use tempfile::tempdir;
use tracing::info;

use crate::common::logger;
use crate::common::state::{generator_procedure2, new_state_with_chainid};

// Creates the Rusk initial state for the tests below
#[allow(dead_code)]
fn initial_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;
    let snapshot = toml::from_str(include_str!(
        "../../../rusk-recovery/config/testnet.toml"
    ))
    .expect("Cannot deserialize config");

    new_state_with_chainid(dir, &snapshot, BLOCK_GAS_LIMIT, 0x3)
}

// disabling the test because the serialization of moonlight transaction changed
// and the transactions in "../assets/deploy.json" are no longer valid
// #[tokio::test(flavor = "multi_thread")]
#[allow(dead_code)]
pub async fn deploy_fail() -> Result<()> {
    // Setup the logger
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp)?;

    let txs = include_str!("../assets/deploy.json");
    let txs: Value = serde_json::from_str(txs).unwrap();

    let txs = txs
        .as_object()
        .unwrap()
        .get("transactions")
        .unwrap()
        .as_array()
        .unwrap();
    let finalize_up_to = 352;
    let finalize_at = 353;

    let mut state_to_delete = vec![];
    let mut new_base = [0u8; 32];
    let mut txs_by_height = BTreeMap::new();
    for t in txs.iter().rev() {
        let block_height = t.get("blockHeight").unwrap().as_u64().unwrap();
        let raw = t.get("tx").unwrap().get("raw").unwrap().as_str().unwrap();
        let raw = hex::decode(raw).unwrap();
        let tx =
            execution_core::transfer::Transaction::from_slice(&raw).unwrap();
        let txs = txs_by_height.entry(block_height).or_insert(vec![]);
        txs.push(tx);
    }

    for (&block_height, txs) in txs_by_height.iter() {
        let (_, state) = generator_procedure2(
            &rusk,
            txs,
            block_height,
            50000000,
            vec![],
            None,
        )
        .unwrap();
        if block_height == finalize_at {
            info!("finalizing state up to {finalize_up_to} ");
            rusk.finalize_state(new_base, state_to_delete.clone())?;
        } else if block_height < finalize_up_to {
            state_to_delete.push(state);
        } else if block_height == finalize_up_to {
            new_base = state
        } else {
            unreachable!()
        }
        println!("height: {block_height} - state: {}", hex::encode(state));
    }

    rusk_recovery_tools::state::restore_state(&tmp)?;

    let txs = include_str!("../assets/mempool.json");
    let txs: Value = serde_json::from_str(txs).unwrap();

    let txs = txs
        .as_object()
        .unwrap()
        .get("mempoolTxs")
        .unwrap()
        .as_array()
        .unwrap();

    let mut mempool = vec![];
    for t in txs.iter().rev() {
        let raw = t.get("raw").unwrap().as_str().unwrap();
        let raw = hex::decode(raw).unwrap();
        let tx =
            execution_core::transfer::Transaction::from_slice(&raw).unwrap();
        mempool.push(tx);
    }

    generator_procedure2(
        &rusk,
        &mempool,
        finalize_at + 1,
        50000000,
        vec![],
        None,
    )
    .unwrap();

    rusk_recovery_tools::state::restore_state(tmp)?;

    Ok(())
}
