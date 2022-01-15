// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_jubjub::JubJubScalar;
use phoenix_core::{Message, Note};
use transfer_circuits::{
    DeriveKey, WfoChange, WfoCommitment, WithdrawFromObfuscatedCircuit,
    WithdrawFromTransparentCircuit,
};

mod wrapper;

use wrapper::TransferWrapper;

#[test]
fn send_to_contract_transparent() {
    let genesis_value = 10_000_000_000;
    let block_height = 1;
    let mut wrapper = TransferWrapper::new(2324, genesis_value);

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
        .unwrap();

    assert!(refund.value(Some(&refund_vk)).unwrap() > 0);

    let remainder: Note = wrapper
        .notes_owned_by(1, &remainder_vk)
        .first()
        .cloned()
        .unwrap();

    let remainder_value = genesis_value - account_value - gas_limit;
    assert_eq!(
        remainder_value,
        remainder.value(Some(&remainder_vk)).unwrap()
    );
}

#[test]
fn send_to_contract_obfuscated() {
    let genesis_value = 10_000_000_000;
    let mut wrapper = TransferWrapper::new(2324, genesis_value);

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
    let message_r = wrapper
        .send_to_contract_obfuscated(
            &[unspent_note],
            &[genesis_ssk],
            &refund_ssk,
            &remainder_psk,
            true,
            gas_limit,
            gas_price,
            account,
            &message_psk,
            account_value,
        )
        .expect("Failed to load balance into contract");

    let message_address = message_psk.gen_stealth_address(&message_r);
    wrapper
        .message(&account, message_address.pk_r())
        .expect("Failed to find appended message");
}

#[test]
fn alice_ping() {
    let genesis_value = 10_000_000_000;
    let mut wrapper = TransferWrapper::new(2324, genesis_value);

    let (genesis_ssk, genesis_vk, _) = wrapper.genesis_identifier();
    let notes = wrapper.notes_owned_by(0, &genesis_vk);
    assert_eq!(1, notes.len());
    let unspent_note = notes[0];

    let alice = *wrapper.alice();
    let ping = TransferWrapper::tx_ping();

    let (refund_ssk, _, _) = wrapper.identifier();
    let (_, _, remainder_psk) = wrapper.identifier();
    let gas_limit = 175_000_000;
    let gas_price = 2;
    wrapper
        .execute(
            &[unspent_note],
            &[genesis_ssk],
            &refund_ssk,
            &remainder_psk,
            true,
            gas_limit,
            gas_price,
            0,
            Some((alice, ping)),
        )
        .expect("Failed to ping alice");
}

#[test]
fn withdraw_from_transparent() {
    let genesis_value = 50_000_000_000;
    let block_height = 1;
    let mut wrapper = TransferWrapper::new(2324, genesis_value);

    let (genesis_ssk, genesis_vk, _) = wrapper.genesis_identifier();
    let notes = wrapper.notes_owned_by(0, &genesis_vk);
    assert_eq!(1, notes.len());
    let unspent_note = notes[0];

    let alice = *wrapper.alice();
    let balance = wrapper.balance(&alice);
    assert_eq!(0, balance);

    let (refund_ssk, refund_vk, _) = wrapper.identifier();
    let (remainder_ssk, remainder_vk, remainder_psk) = wrapper.identifier();
    let alice_value = 100;
    let gas_limit = 500_000_000;
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
            alice,
            alice_value,
        )
        .expect("Failed to load balance into contract");

    let balance = wrapper.balance(&alice);
    assert_eq!(alice_value, balance);

    let notes = wrapper.notes(block_height);
    assert_eq!(2, notes.len());

    let refund: Note = wrapper
        .notes_owned_by(1, &refund_vk)
        .first()
        .cloned()
        .unwrap();

    assert!(refund.value(Some(&refund_vk)).unwrap() > 0);

    let remainder: Note = wrapper
        .notes_owned_by(1, &remainder_vk)
        .first()
        .cloned()
        .unwrap();

    let remainder_value = genesis_value - alice_value - gas_limit;
    assert_eq!(
        remainder_value,
        remainder.value(Some(&remainder_vk)).unwrap()
    );

    let withdraw_value = 25;
    let (withdraw_ssk, withdraw_note) =
        wrapper.generate_note(true, withdraw_value);
    let withdraw_vk = withdraw_ssk.view_key();
    let withdraw_commitment = *withdraw_note.value_commitment();
    let withdraw_value = withdraw_note
        .value(Some(&withdraw_vk))
        .expect("Failed to decrypt value");
    let withdraw_blinder = withdraw_note
        .blinding_factor(Some(&withdraw_vk))
        .expect("Failed to decrypt blinder");
    let withdraw_circuit = WithdrawFromTransparentCircuit::new(
        withdraw_commitment,
        withdraw_value,
        withdraw_blinder,
    );
    let withdraw_proof = wrapper.generate_proof(withdraw_circuit);
    let withdraw_tx = TransferWrapper::tx_withdraw(
        withdraw_value,
        withdraw_note,
        withdraw_proof,
    );

    let (new_refund_ssk, new_refund_vk, _) = wrapper.identifier();

    let gas_price = 1;
    wrapper
        .execute(
            &[refund],
            &[refund_ssk],
            &new_refund_ssk,
            &remainder_psk,
            true,
            gas_limit,
            gas_price,
            withdraw_value,
            Some((alice, withdraw_tx)),
        )
        .expect("Failed to withdraw from alice");

    let balance = wrapper.balance(&alice);
    assert_eq!(alice_value - withdraw_value, balance);

    let withdraw: Note = wrapper
        .notes_owned_by(1, &withdraw_vk)
        .first()
        .cloned()
        .unwrap();

    assert_eq!(withdraw_value, withdraw.value(Some(&withdraw_vk)).unwrap());

    let refund: Note = wrapper
        .notes_owned_by(1, &new_refund_vk)
        .first()
        .cloned()
        .unwrap();

    assert!(refund.value(Some(&new_refund_vk)).unwrap() > 0);

    let transfer_value = 15;
    let bob = *wrapper.bob();
    let transfer_tx =
        TransferWrapper::tx_withdraw_to_contract(bob, transfer_value);
    wrapper
        .execute(
            &[remainder],
            &[remainder_ssk],
            &refund_ssk,
            &remainder_psk,
            true,
            gas_limit,
            gas_price,
            0,
            Some((alice, transfer_tx)),
        )
        .expect("Failed to withdraw from alice");

    let balance = wrapper.balance(&alice);
    assert_eq!(alice_value - withdraw_value - transfer_value, balance);

    let balance = wrapper.balance(&bob);
    assert_eq!(transfer_value, balance);
}

#[test]
fn withdraw_from_obfuscated() {
    let genesis_value = 10_000_000_000;
    let mut wrapper = TransferWrapper::new(2324, genesis_value);

    let (genesis_ssk, genesis_vk, _) = wrapper.genesis_identifier();
    let notes = wrapper.notes_owned_by(0, &genesis_vk);
    assert_eq!(1, notes.len());
    let unspent_note = notes[0];

    let account = *wrapper.alice();
    let balance = wrapper.balance(&account);
    assert_eq!(0, balance);

    let (refund_ssk, refund_vk, _) = wrapper.identifier();
    let (_, remainder_vk, remainder_psk) = wrapper.identifier();
    let (_, _, message_psk) = wrapper.identifier();
    let (_, _, change_psk) = wrapper.identifier();

    let account_value = 100;
    let gas_limit = 175_000_000;
    let gas_price = 2;
    let message_r = wrapper
        .send_to_contract_obfuscated(
            &[unspent_note],
            &[genesis_ssk],
            &refund_ssk,
            &remainder_psk,
            true,
            gas_limit,
            gas_price,
            account,
            &message_psk,
            account_value,
        )
        .expect("Failed to load balance into contract");

    let message_address = message_psk.gen_stealth_address(&message_r);
    let message = wrapper
        .message(&account, message_address.pk_r())
        .expect("Failed to find appended message");

    let (message_value, message_blinder) = message
        .decrypt(&message_r, &message_psk)
        .expect("Failed to decrypt message");

    let refund: Note = wrapper
        .notes_owned_by(1, &refund_vk)
        .first()
        .cloned()
        .unwrap();

    let remainder: Note = wrapper
        .notes_owned_by(1, &remainder_vk)
        .first()
        .cloned()
        .unwrap();

    let remainder_value = genesis_value - account_value - gas_limit;
    assert_eq!(
        remainder_value,
        remainder.value(Some(&remainder_vk)).unwrap()
    );

    let change_value = 10;
    let change_r = JubJubScalar::random(wrapper.rng());
    let change_address = change_psk.gen_stealth_address(&change_r);
    let change =
        Message::new(wrapper.rng(), &change_r, &change_psk, change_value);
    let (_, change_blinder) = change
        .decrypt(&change_r, &change_psk)
        .expect("Failed to decrypt change");
    let change_pk_r = *change_address.pk_r().as_ref();

    let withdraw_value = account_value - change_value;
    let (_, withdraw_vk, withdraw_psk) = wrapper.identifier();
    let withdraw_blinder = JubJubScalar::random(wrapper.rng());
    let withdraw = Note::obfuscated(
        wrapper.rng(),
        &withdraw_psk,
        withdraw_value,
        withdraw_blinder,
    );

    let wfo_input = WfoCommitment {
        value: message_value,
        blinder: message_blinder,
        commitment: *message.value_commitment(),
    };

    let wfo_change = {
        let derive_key = DeriveKey::new(false, &change_psk);
        WfoChange {
            blinder: change_blinder,
            derive_key,
            message: change,
            pk_r: change_pk_r,
            r: change_r,
            value: change_value,
        }
    };

    let wfo_output = WfoCommitment {
        blinder: withdraw_blinder,
        commitment: *withdraw.value_commitment(),
        value: withdraw_value,
    };

    let wfo_circuit = WithdrawFromObfuscatedCircuit {
        input: wfo_input,
        change: wfo_change,
        output: wfo_output,
    };

    let wfo_proof = wrapper.generate_proof(wfo_circuit);
    let wfo_tx = TransferWrapper::tx_withdraw_obfuscated(
        message,
        message_address,
        change,
        change_address,
        withdraw,
        wfo_proof,
    );

    let (new_refund_ssk, _, _) = wrapper.identifier();

    let gas_limit = 300_000_000;
    let gas_price = 1;
    wrapper
        .execute(
            &[refund],
            &[refund_ssk],
            &new_refund_ssk,
            &remainder_psk,
            true,
            gas_limit,
            gas_price,
            0,
            Some((account, wfo_tx)),
        )
        .expect("Failed to withdraw from alice");

    let withdraw_notes = wrapper.notes_owned_by(1, &withdraw_vk);

    assert_eq!(1, withdraw_notes.len());
    assert!(wrapper.message(&account, message_address.pk_r()).is_err());
    assert_eq!(
        withdraw_value,
        withdraw_notes[0].value(Some(&withdraw_vk)).unwrap()
    );

    let change_address = change_psk.gen_stealth_address(&change_r);
    wrapper
        .message(&account, change_address.pk_r())
        .expect("Failed to find appended change");
}
