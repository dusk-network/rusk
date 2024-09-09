// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![feature(lazy_cell)]

#[path = "../tests/common/mod.rs"]
mod common;

use std::io::{BufRead, BufReader};
use std::sync::Arc;

use criterion::measurement::WallTime;
use criterion::{
    criterion_group, criterion_main, BenchmarkGroup, BenchmarkId, Criterion,
};
use execution_core::{
    signatures::bls::{PublicKey as BlsPublicKey, SecretKey as BlsSecretKey},
    transfer::Transaction as ProtocolTransaction,
};
use node_data::ledger::Transaction;
use rand::prelude::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rusk::Rusk;
use tempfile::tempdir;

use common::state::new_state;

const BLOCK_GAS_LIMIT: u64 = 1_000_000_000_000;

fn load_phoenix_txs() -> Vec<Transaction> {
    // The file "phoenix-txs" can be generated using
    // `generate_phoenix_txs()` in "tests/rusk-state.rs".
    const TXS_BYTES: &[u8] = include_bytes!("phoenix-txs");

    let mut txs = Vec::new();

    for line in BufReader::new(TXS_BYTES).lines() {
        let line = line.unwrap();
        let tx_bytes = hex::decode(line).unwrap();
        let tx = ProtocolTransaction::from_slice(&tx_bytes).unwrap();
        txs.push(tx.into());
    }

    preverify(&txs);

    txs
}

fn load_moonlight_txs() -> Vec<Transaction> {
    // The file "moonlight-txs" can be generated using
    // `generate_moonlight_txs()` in "tests/rusk-state.rs".
    const TXS_BYTES: &[u8] = include_bytes!("moonlight-txs");

    let mut txs = Vec::new();

    for line in BufReader::new(TXS_BYTES).lines() {
        let line = line.unwrap();
        let tx_bytes = hex::decode(line).unwrap();
        let tx = ProtocolTransaction::from_slice(&tx_bytes).unwrap();
        txs.push(tx.into());
    }

    preverify(&txs);

    txs
}

fn preverify(txs: &[Transaction]) {
    for tx in txs {
        match &tx.inner {
            ProtocolTransaction::Phoenix(tx) => {
                match rusk::verifier::verify_proof(tx) {
                    Ok(true) => Ok(()),
                    Ok(false) => Err(anyhow::anyhow!("Invalid proof")),
                    Err(e) => {
                        Err(anyhow::anyhow!("Cannot verify the proof: {e}"))
                    }
                }
                .unwrap()
            }
            ProtocolTransaction::Moonlight(tx) => {
                match rusk::verifier::verify_signature(tx) {
                    Ok(true) => Ok(()),
                    Ok(false) => Err(anyhow::anyhow!("Invalid signature")),
                    Err(e) => {
                        Err(anyhow::anyhow!("Cannot verify the signature: {e}"))
                    }
                }
                .unwrap()
            }
        }
    }
}

pub fn with_group<T, F>(name: &str, c: &mut Criterion, closure: F) -> T
where
    F: FnOnce(&mut BenchmarkGroup<WallTime>) -> T,
{
    let mut group = c.benchmark_group(name);
    let r = closure(&mut group);
    group.finish();
    r
}

fn bench_accept(
    group: &mut BenchmarkGroup<WallTime>,
    name: &str,
    rusk: Rusk,
    txs: Vec<Transaction>,
) {
    const BLOCK_HEIGHT: u64 = 1;
    const BLOCK_GAS_LIMIT: u64 = 1_000_000_000_000;

    let generator = {
        let mut rng = StdRng::seed_from_u64(0xbeef);
        let sk = BlsSecretKey::random(&mut rng);
        BlsPublicKey::from(&sk)
    };

    let txs = Arc::new(txs);

    for n_txs in N_TXS {
        let rusk = rusk.clone();
        let txs = txs.clone();

        group.bench_with_input(
            BenchmarkId::new(name, format!("{} TXs", n_txs)),
            n_txs,
            move |b, n_txs| {
                b.iter(|| {
                    let txs = txs[..*n_txs].to_vec();

                    rusk.accept_transactions(
                        BLOCK_HEIGHT,
                        BLOCK_GAS_LIMIT,
                        generator,
                        txs,
                        None,
                        vec![],
                        None,
                    )
                    .expect("Accepting transactions should succeed");

                    rusk.revert_to_base_root()
                        .expect("Reverting should succeed");
                })
            },
        );
    }
}

pub fn accept_benchmark(c: &mut Criterion) {
    let tmp = tempdir().expect("Creating a temp dir should work");
    let snapshot = toml::from_str(include_str!("../tests/config/bench.toml"))
        .expect("Cannot deserialize config");

    let rusk = new_state(&tmp, &snapshot, BLOCK_GAS_LIMIT)
        .expect("Creating state should work");

    let phoenix_txs = load_phoenix_txs();
    let moonlight_txs = load_moonlight_txs();

    let mut rng = StdRng::seed_from_u64(0xbeef);
    let mut mixed_txs = phoenix_txs.clone();
    mixed_txs.extend(moonlight_txs.clone());
    mixed_txs.shuffle(&mut rng);

    let mut group = c.benchmark_group("AST");
    bench_accept(&mut group, "Phoenix", rusk.clone(), phoenix_txs);
    bench_accept(&mut group, "Moonlight", rusk.clone(), moonlight_txs);
    bench_accept(&mut group, "Phoenix & Moonlight", rusk.clone(), mixed_txs);
    group.finish();
}

criterion_group!(benches, accept_benchmark);
criterion_main!(benches);

const N_TXS: &[usize] =
    &[1, 2, 3, 4, 5, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
