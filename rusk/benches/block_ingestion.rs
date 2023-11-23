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
use std::time;

use criterion::measurement::WallTime;
use criterion::{
    criterion_group, criterion_main, BenchmarkGroup, BenchmarkId, Criterion,
};
use dusk_bls12_381_sign::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};
use node_data::ledger::Transaction;
use phoenix_core::Transaction as PhoenixTransaction;
use rand::prelude::StdRng;
use rand::SeedableRng;
use tempfile::tempdir;

use common::state::new_state;

fn load_txs() -> Vec<Transaction> {
    const TXS_BYTES: &[u8] = include_bytes!("block");

    let mut txs = Vec::new();

    for line in BufReader::new(TXS_BYTES).lines() {
        let line = line.unwrap();
        let tx_bytes = hex::decode(line).unwrap();
        let tx = PhoenixTransaction::from_slice(&tx_bytes).unwrap();
        txs.push(Transaction {
            version: 1,
            r#type: 0,
            inner: tx,
        });
    }

    txs
}

pub fn with_group<T, F>(name: &str, c: &mut Criterion, closure: F) -> T
where
    F: FnOnce(&mut BenchmarkGroup<WallTime>) -> T,
{
    let mut group = c.benchmark_group(name);
    // group.measurement_time(time::Duration::from_secs(200));
    let r = closure(&mut group);
    group.finish();
    r
}

pub fn accept_benchmark(c: &mut Criterion) {
    with_group("State Transitions", c, |group| {
        let tmp = tempdir().expect("Creating a temp dir should work");
        let snapshot =
            toml::from_str(include_str!("../tests/config/bench.toml"))
                .expect("Cannot deserialize config");

        let rusk =
            new_state(&tmp, &snapshot).expect("Creating state should work");
        let txs = Arc::new(load_txs());

        let generator = {
            let mut rng = StdRng::seed_from_u64(0xbeef);
            let sk = BlsSecretKey::random(&mut rng);
            BlsPublicKey::from(&sk)
        };

        const BLOCK_HEIGHT: u64 = 1;
        const BLOCK_GAS_LIMIT: u64 = 1_000_000_000_000;

        const N_TXS: &[usize] =
            &[1, 2, 3, 4, 5, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100];

        for n_txs in N_TXS {
            let rusk = rusk.clone();
            let txs = txs.clone();

            group.bench_with_input(
                BenchmarkId::new("AST", format!("{n_txs} TXs")),
                n_txs,
                move |b, n_txs| {
                    b.iter(|| {
                        let txs = txs.as_ref()[..*n_txs].to_vec();

                        rusk.accept_transactions(
                            BLOCK_HEIGHT,
                            BLOCK_GAS_LIMIT,
                            generator,
                            txs,
                            None,
                        )
                        .expect("Accepting transactions should succeed");

                        rusk.revert().expect("Reverting should succeed");
                    })
                },
            );
        }
    });
}

criterion_group!(benches, accept_benchmark);
criterion_main!(benches);
