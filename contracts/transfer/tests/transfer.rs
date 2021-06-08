// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use phoenix_core::Note;

mod wrapper;

use wrapper::TransferWrapper;

#[test]
fn withdraw_from_transparent() {
    let genesis_value = 1_000;
    let block_height = 1;
    let mut wrapper = TransferWrapper::new(2324, block_height, genesis_value);

    let (genesis_ssk, genesis_vk, _) = wrapper.genesis_identifier();
    let notes = wrapper.notes_owned_by(0, &genesis_vk);
    assert_eq!(1, notes.len());
    let unspent_note = notes[0];

    let account = wrapper.address();
    let balance = wrapper.balance(&account);
    assert_eq!(0, balance);

    let (refund_ssk, refund_vk, _) = wrapper.identifier();
    let (_, remainder_vk, remainder_psk) = wrapper.identifier();
    let account_value = 100;
    let gas_limit = 50;
    let gas_price = 2;
    wrapper
        .send_to_contract_transparent(
            &[unspent_note],
            &[genesis_ssk],
            &refund_ssk,
            &remainder_psk,
            true,
            gas_limit,
            gas_price,
            account,
            account_value,
        )
        .expect("Failed to load balance into contract");

    let balance = wrapper.balance(&account);
    assert_eq!(account_value, balance);

    let notes = wrapper.notes(block_height);
    assert_eq!(2, notes.len());

    let refund: Note = wrapper
        .notes_owned_by(1, &refund_vk)
        .first()
        .cloned()
        .unwrap()
        .into();
    assert!(refund.value(Some(&refund_vk)).unwrap() > 0);

    let remainder: Note = wrapper
        .notes_owned_by(1, &remainder_vk)
        .first()
        .cloned()
        .unwrap()
        .into();
    let remainder_value = genesis_value - account_value - gas_limit;
    assert_eq!(
        remainder_value,
        remainder.value(Some(&remainder_vk)).unwrap()
    );
}
