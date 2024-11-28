// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use criterion::{criterion_group, criterion_main, Criterion};
use dusk_consensus::config::MAX_NUMBER_OF_TRANSACTIONS;
use dusk_consensus::merkle::merkle_root;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

fn merkle(c: &mut Criterion) {
    let tx_hashes: Vec<_> = (0..MAX_NUMBER_OF_TRANSACTIONS)
        .map(|seed| {
            let rng = &mut StdRng::seed_from_u64(seed as u64);
            let mut buf = [0u8; 32];
            rng.fill_bytes(&mut buf);
            buf
        })
        .collect();

    let label: String = format!("merkle_{}", MAX_NUMBER_OF_TRANSACTIONS);

    c.bench_function(&label, |b| {
        b.iter(|| {
            let _ = merkle_root(&tx_hashes[..]);
        })
    });
}

criterion_group!(benches, merkle);
criterion_main!(benches);
