// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![feature(lazy_cell)]

use criterion::{criterion_group, criterion_main, Criterion};

#[path = "../tests/common/mod.rs"]
mod common;

pub fn accept_benchmark(c: &mut Criterion) {
    c.bench_function("AST", |b| b.iter(|| {}));
}

criterion_group!(benches, accept_benchmark);
criterion_main!(benches);
