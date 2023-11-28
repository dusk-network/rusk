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
use std::time::Duration;

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

        for input in INPUTS {
            let rusk = rusk.clone();
            let txs = txs.clone();

            group.measurement_time(Duration::from_secs(input.measurement_time));

            group.bench_with_input(
                BenchmarkId::new("AST", format!("{} TXs", input.n_txs)),
                &input.n_txs,
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

struct Input {
    n_txs: usize,
    measurement_time: u64, // secs
}

const INPUTS: &[Input] = &[
    Input {
        n_txs: 1,
        measurement_time: 5,
    },
    Input {
        n_txs: 2,
        measurement_time: 7,
    },
    Input {
        n_txs: 3,
        measurement_time: 9,
    },
    Input {
        n_txs: 4,
        measurement_time: 10,
    },
    Input {
        n_txs: 5,
        measurement_time: 12,
    },
    Input {
        n_txs: 10,
        measurement_time: 20,
    },
    Input {
        n_txs: 20,
        measurement_time: 35,
    },
    Input {
        n_txs: 30,
        measurement_time: 60,
    },
    Input {
        n_txs: 40,
        measurement_time: 67,
    },
    Input {
        n_txs: 50,
        measurement_time: 84,
    },
    Input {
        n_txs: 60,
        measurement_time: 99,
    },
    Input {
        n_txs: 70,
        measurement_time: 115,
    },
    Input {
        n_txs: 80,
        measurement_time: 131,
    },
    Input {
        n_txs: 90,
        measurement_time: 150,
    },
    Input {
        n_txs: 100,
        measurement_time: 164,
    },
];
