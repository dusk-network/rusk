// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn send_to_contract_transparent(c: &mut Criterion) {
    unimplemented!()
}

fn send_to_contract_obfuscated(_c: &mut Criterion) {
    unimplemented!()
}

fn withdraw_from_transparent(_c: &mut Criterion) {
    unimplemented!()
}

fn withdraw_from_obfuscated(_c: &mut Criterion) {
    unimplemented!()
}

criterion_group!(
    benches,
    send_to_contract_transparent,
    send_to_contract_obfuscated,
    withdraw_from_transparent,
    withdraw_from_obfuscated
);
criterion_main!(benches);
