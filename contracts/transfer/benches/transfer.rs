// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use test_utils::TransferWrapper;

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

    let (refund_ssk, _, _) = wrapper.identifier();
    let (_, _, remainder_psk) = wrapper.identifier();

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

fn send_to_contract_obfuscated(c: &mut Criterion) {
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

    let (refund_ssk, _, _) = wrapper.identifier();
    let (_, _, remainder_psk) = wrapper.identifier();
    let (_, _, message_psk) = wrapper.identifier();

    let account_value = 100;
    let gas_limit = 175_000_000;
    let gas_price = 2;
    c.bench_function("send obfuscated", |b| {
        b.iter(|| {
            wrapper.send_to_contract_obfuscated(
                black_box(&[unspent_note]),
                black_box(&[genesis_ssk]),
                black_box(&refund_ssk),
                black_box(&remainder_psk),
                black_box(true),
                black_box(gas_limit),
                black_box(gas_price),
                black_box(account),
                black_box(&message_psk),
                black_box(account_value),
            )
        })
    });
}

fn withdraw_from_transparent(c: &mut Criterion) {
    let genesis_value = 50_000_000_000;
    let block_height = 1;
    let mut wrapper = TransferWrapper::new(2324, block_height, genesis_value);

    let (genesis_ssk, genesis_vk, _) = wrapper.genesis_identifier();
    let notes = wrapper.notes_owned_by(0, &genesis_vk);
    let unspent_note = notes[0];

    let alice = *wrapper.alice();

    let (refund_ssk, _, _) = wrapper.identifier();
    let (_, _, remainder_psk) = wrapper.identifier();

    let alice_value = 100;
    let gas_limit = 500_000_000;
    let gas_price = 2;

    c.bench_function("withdraw transparent", |b| {
        b.iter(|| {
            wrapper.send_to_contract_transparent(
                black_box(&[unspent_note]),
                black_box(&[genesis_ssk]),
                black_box(&refund_ssk),
                black_box(&remainder_psk),
                black_box(true),
                black_box(gas_limit),
                black_box(gas_price),
                black_box(alice),
                black_box(alice_value),
            )
        })
    });
}

fn withdraw_from_obfuscated(c: &mut Criterion) {
    let genesis_value = 10_000_000_000;
    let block_height = 1;
    let mut wrapper = TransferWrapper::new(2324, block_height, genesis_value);

    let (genesis_ssk, genesis_vk, _) = wrapper.genesis_identifier();
    let notes = wrapper.notes_owned_by(0, &genesis_vk);
    let unspent_note = notes[0];

    let account = *wrapper.alice();

    let (refund_ssk, _, _) = wrapper.identifier();
    let (_, _, remainder_psk) = wrapper.identifier();
    let (_, _, message_psk) = wrapper.identifier();

    let account_value = 100;
    let gas_limit = 175_000_000;
    let gas_price = 2;

    c.bench_function("withdraw obfuscated", |b| {
        b.iter(|| {
            wrapper.send_to_contract_obfuscated(
                black_box(&[unspent_note]),
                black_box(&[genesis_ssk]),
                black_box(&refund_ssk),
                black_box(&remainder_psk),
                black_box(true),
                black_box(gas_limit),
                black_box(gas_price),
                black_box(account),
                black_box(&message_psk),
                black_box(account_value),
            )
        })
    });
}

criterion_group!(
    benches,
    send_to_contract_transparent,
    send_to_contract_obfuscated,
    withdraw_from_transparent,
    withdraw_from_obfuscated
);
criterion_main!(benches);
