// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use super::tests::wrapper::TransferWrapper;

fn send_to_contract_transparent(c: &mut Criterion) {
    let genesis_value = 10_000_000_000;
    let block_height = 1;
    let mut wrapper = TransferWrapper::new(2324, block_height, genesis_value);

    let (genesis_ssk, genesis_vk, _) = wrapper.genesis_identifier();
    let notes = wrapper.notes_owned_by(0, &genesis_vk);
    assert_eq!(1, notes.len());
    let unspent_note = notes[0];

    let account = *wrapper.alice();
    let balance = wrapper.balance(&account);
    assert_eq!(0, balance);

    let (refund_ssk, refund_vk, _) = wrapper.identifier();
    let (_, remainder_vk, remainder_psk) = wrapper.identifier();
    let account_value = 100;
    let gas_limit = 175_000_000;
    let gas_price = 2;
    c.bench_function("send transparent", |b| {
        b.iter(|| {
            wrapper.send_to_contract_transparent(
                black_box(&[unspent_note]),
                black_box(&[genesis_ssk]),
                black_box(&refund_ssk),
                black_box(&remainder_psk),
                black_box(true),
                black_box(gas_limit),
                black_box(gas_price),
                black_box(account),
                black_box(account_value),
            )
        })
    });
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
