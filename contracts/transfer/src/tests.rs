// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{ops, Call, TransferContract, TransferExecute};
use core::convert::{TryFrom, TryInto};

use alloc::vec::Vec;
use canonical_host::MemStore;
use dusk_bls12_381::BlsScalar;
use dusk_jubjub::JubJubScalar;
use dusk_pki::{Ownable, SecretSpendKey};
use phoenix_core::Note;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rusk_vm::{Contract, GasMeter, NetworkState, StandardABI};

const CODE: &'static [u8] = include_bytes!(
    "../target/wasm32-unknown-unknown/release/transfer_contract.wasm"
);

#[test]
fn withdraw_from_transparent() {
    let mut rng = StdRng::seed_from_u64(2324u64);
    let store = MemStore::new();

    let genesis_ssk = SecretSpendKey::random(&mut rng);
    let genesis_vk = genesis_ssk.view_key();
    let genesis_psk = genesis_ssk.public_spend_key();
    let genesis_value = 1_000;
    let genesis_note = Note::transparent(&mut rng, &genesis_psk, genesis_value);
    let transfer = TransferContract::try_from(genesis_note).unwrap();

    let block_height = 1;
    let contract = Contract::new(transfer, CODE.to_vec(), &store).unwrap();
    let mut network =
        NetworkState::<StandardABI<MemStore>, MemStore>::with_block_height(
            block_height,
        );
    let contract = network.deploy(contract).unwrap();
    let mut gas = GasMeter::with_limit(1_000);

    let block_height = 0;
    let notes = network
        .query::<_, Vec<Note>>(
            contract,
            (ops::QR_NOTES_FROM_HEIGHT, block_height),
            &mut gas,
        )
        .unwrap();
    assert_eq!(1, notes.len());
    let unspent_note = notes[0];
    assert!(genesis_vk.owns(&unspent_note));

    let root = network
        .query::<_, BlsScalar>(contract, ops::QR_ROOT, &mut gas)
        .unwrap();
    let genesis_nullifier = unspent_note.gen_nullifier(&genesis_ssk);
    let genesis_unspent_value = 850;
    let genesis_unspent_note =
        Note::transparent(&mut rng, &genesis_psk, genesis_unspent_value);

    let bob_ssk = SecretSpendKey::random(&mut rng);
    let bob_vk = bob_ssk.view_key();
    let bob_psk = bob_ssk.public_spend_key();
    let bob_address = BlsScalar::random(&mut rng);
    let bob_value = 100;
    let bob_blinding_factor = JubJubScalar::random(&mut rng);
    let bob_note =
        Note::obfuscated(&mut rng, &bob_psk, bob_value, bob_blinding_factor);

    let (mut bob_fee, bob_crossover) = bob_note.try_into().unwrap();
    // TODO define the gas price and limit
    // https://github.com/dusk-network/rusk/issues/187
    let gas_price = 1;
    let gas_limit = 50;
    bob_fee.gas_price = gas_price;
    bob_fee.gas_limit = gas_limit;

    let balance = network
        .query::<_, u64>(contract, (ops::QR_BALANCE, bob_address), &mut gas)
        .unwrap();
    assert_eq!(0u64, balance);

    // TODO implement proof verification
    // https://github.com/dusk-network/rusk/issues/194
    let spend_proof_execute = vec![0xfa];
    let spend_proof_stct = vec![0xfa];
    let call = TransferExecute {
        anchor: root,
        nullifiers: [genesis_nullifier].into(),
        fee: bob_fee,
        crossover: Some(bob_crossover),
        notes: [genesis_unspent_note].into(),
        spend_proof: spend_proof_execute,
        call: Some(Call::SendToContractTransparent {
            address: bob_address,
            value: bob_value,
            pk: bob_psk.A().into(),
            spend_proof: spend_proof_stct,
        }),
    };

    let result = network
        .transact::<_, bool>(contract, call, &mut gas)
        .unwrap();
    assert!(result);

    let balance = network
        .query::<_, u64>(contract, (ops::QR_BALANCE, bob_address), &mut gas)
        .unwrap();
    assert_eq!(bob_value, balance);

    let block_height = 1u64;
    let notes = network
        .query::<_, Vec<Note>>(
            contract,
            (ops::QR_NOTES_FROM_HEIGHT, block_height),
            &mut gas,
        )
        .unwrap();
    assert_eq!(2, notes.len());

    let genesis_unspent_note = notes
        .iter()
        .filter(|n| genesis_vk.owns(n.stealth_address()))
        .collect::<Vec<&Note>>()
        .first()
        .cloned()
        .unwrap();
    assert_eq!(
        genesis_unspent_value,
        genesis_unspent_note.value(Some(&genesis_vk)).unwrap()
    );

    let bob_note = notes
        .iter()
        .filter(|n| bob_vk.owns(n.stealth_address()))
        .collect::<Vec<&Note>>()
        .first()
        .cloned()
        .unwrap();
    let bob_refund = bob_note.value(Some(&bob_vk)).unwrap();
    assert!(bob_refund > 0);

    let root = network
        .query::<_, BlsScalar>(contract, ops::QR_ROOT, &mut gas)
        .unwrap();
    let bob_nullifier = bob_note.gen_nullifier(&bob_ssk);

    let bob_blinding_factor = JubJubScalar::random(&mut rng);
    let bob_fee =
        Note::obfuscated(&mut rng, &bob_psk, bob_refund, bob_blinding_factor);
    let (mut bob_fee, _) = bob_fee.try_into().unwrap();
    let gas_price = 1;
    let gas_limit = 10;
    bob_fee.gas_price = gas_price;
    bob_fee.gas_limit = gas_limit;

    let bob_output_value = bob_refund - gas_limit;
    let bob_output = Note::transparent(&mut rng, &bob_psk, bob_output_value);

    let alice_ssk = SecretSpendKey::random(&mut rng);
    let alice_vk = alice_ssk.view_key();
    let alice_psk = alice_ssk.public_spend_key();
    let alice_withdraw_value = bob_value;
    let alice_withdraw =
        Note::transparent(&mut rng, &alice_psk, alice_withdraw_value);

    // TODO implement proof verification
    // https://github.com/dusk-network/rusk/issues/194
    let spend_proof_execute = vec![0xfa];
    let call = TransferExecute {
        anchor: root,
        nullifiers: [bob_nullifier].into(),
        fee: bob_fee,
        crossover: None,
        notes: [bob_output].into(),
        spend_proof: spend_proof_execute,
        call: Some(Call::WithdrawFromTransparent {
            address: bob_address,
            note: alice_withdraw,
        }),
    };

    let result = network
        .transact::<_, bool>(contract, call, &mut gas)
        .unwrap();
    assert!(result);

    let balance = network
        .query::<_, u64>(contract, (ops::QR_BALANCE, bob_address), &mut gas)
        .unwrap();
    assert_eq!(0u64, balance);

    let block_height = 1u64;
    let notes = network
        .query::<_, Vec<Note>>(
            contract,
            (ops::QR_NOTES_FROM_HEIGHT, block_height),
            &mut gas,
        )
        .unwrap();

    let alice_withdraw = notes
        .iter()
        .filter(|n| alice_vk.owns(n.stealth_address()))
        .collect::<Vec<&Note>>()
        .first()
        .cloned()
        .unwrap();
    assert_eq!(
        alice_withdraw_value,
        alice_withdraw.value(Some(&alice_vk)).unwrap()
    );
}

#[test]
fn withdraw_from_transparent_to_contract() {
    let mut rng = StdRng::seed_from_u64(2324u64);
    let store = MemStore::new();

    let genesis_ssk = SecretSpendKey::random(&mut rng);
    let genesis_vk = genesis_ssk.view_key();
    let genesis_psk = genesis_ssk.public_spend_key();
    let genesis_value = 1_000;
    let genesis_note = Note::transparent(&mut rng, &genesis_psk, genesis_value);
    let transfer = TransferContract::try_from(genesis_note).unwrap();

    let block_height = 1;
    let contract = Contract::new(transfer, CODE.to_vec(), &store).unwrap();
    let mut network =
        NetworkState::<StandardABI<MemStore>, MemStore>::with_block_height(
            block_height,
        );
    let contract = network.deploy(contract).unwrap();
    let mut gas = GasMeter::with_limit(1_000);

    let block_height = 0;
    let notes = network
        .query::<_, Vec<Note>>(
            contract,
            (ops::QR_NOTES_FROM_HEIGHT, block_height),
            &mut gas,
        )
        .unwrap();
    assert_eq!(1, notes.len());
    let unspent_note = notes[0];
    assert!(genesis_vk.owns(&unspent_note));

    let root = network
        .query::<_, BlsScalar>(contract, ops::QR_ROOT, &mut gas)
        .unwrap();
    let genesis_nullifier = unspent_note.gen_nullifier(&genesis_ssk);
    let genesis_unspent_value = 850;
    let genesis_unspent_note =
        Note::transparent(&mut rng, &genesis_psk, genesis_unspent_value);

    let bob_ssk = SecretSpendKey::random(&mut rng);
    let bob_vk = bob_ssk.view_key();
    let bob_psk = bob_ssk.public_spend_key();
    let bob_address = BlsScalar::random(&mut rng);
    let bob_value = 100;
    let bob_blinding_factor = JubJubScalar::random(&mut rng);
    let bob_note =
        Note::obfuscated(&mut rng, &bob_psk, bob_value, bob_blinding_factor);

    let (mut bob_fee, bob_crossover) = bob_note.try_into().unwrap();
    // TODO define the gas price and limit
    // https://github.com/dusk-network/rusk/issues/187
    let gas_price = 1;
    let gas_limit = 50;
    bob_fee.gas_price = gas_price;
    bob_fee.gas_limit = gas_limit;

    let balance = network
        .query::<_, u64>(contract, (ops::QR_BALANCE, bob_address), &mut gas)
        .unwrap();
    assert_eq!(0u64, balance);

    // TODO implement proof verification
    // https://github.com/dusk-network/rusk/issues/194
    let spend_proof_execute = vec![0xfa];
    let spend_proof_stct = vec![0xfa];
    let call = TransferExecute {
        anchor: root,
        nullifiers: [genesis_nullifier].into(),
        fee: bob_fee,
        crossover: Some(bob_crossover),
        notes: [genesis_unspent_note].into(),
        spend_proof: spend_proof_execute,
        call: Some(Call::SendToContractTransparent {
            address: bob_address,
            value: bob_value,
            pk: bob_psk.A().into(),
            spend_proof: spend_proof_stct,
        }),
    };

    let result = network
        .transact::<_, bool>(contract, call, &mut gas)
        .unwrap();
    assert!(result);

    let balance = network
        .query::<_, u64>(contract, (ops::QR_BALANCE, bob_address), &mut gas)
        .unwrap();
    assert_eq!(bob_value, balance);

    let block_height = 1u64;
    let notes = network
        .query::<_, Vec<Note>>(
            contract,
            (ops::QR_NOTES_FROM_HEIGHT, block_height),
            &mut gas,
        )
        .unwrap();
    assert_eq!(2, notes.len());

    let bob_note = notes
        .iter()
        .filter(|n| bob_vk.owns(n.stealth_address()))
        .collect::<Vec<&Note>>()
        .first()
        .cloned()
        .unwrap();
    let bob_refund = bob_note.value(Some(&bob_vk)).unwrap();
    assert!(bob_refund > 0);

    let bob_nullifier = bob_note.gen_nullifier(&bob_ssk);
    let bob_blinding_factor = JubJubScalar::random(&mut rng);
    let bob_fee =
        Note::obfuscated(&mut rng, &bob_psk, bob_refund, bob_blinding_factor);
    let (mut bob_fee, _) = bob_fee.try_into().unwrap();
    let gas_price = 1;
    let gas_limit = 10;
    bob_fee.gas_price = gas_price;
    bob_fee.gas_limit = gas_limit;

    let bob_output_value = bob_refund - gas_limit;
    let bob_output = Note::transparent(&mut rng, &bob_psk, bob_output_value);

    let alice_address = BlsScalar::random(&mut rng);
    let alice_value = 45;

    // TODO implement proof verification
    // https://github.com/dusk-network/rusk/issues/194
    let spend_proof_execute = vec![0xfa];
    let call = TransferExecute {
        anchor: root,
        nullifiers: [bob_nullifier].into(),
        fee: bob_fee,
        crossover: None,
        notes: [bob_output].into(),
        spend_proof: spend_proof_execute,
        call: Some(Call::WithdrawFromTransparentToContract {
            from: bob_address,
            to: alice_address,
            value: alice_value,
        }),
    };

    let result = network
        .transact::<_, bool>(contract, call, &mut gas)
        .unwrap();
    assert!(result);

    let bob_value = bob_value - alice_value;
    let balance = network
        .query::<_, u64>(contract, (ops::QR_BALANCE, bob_address), &mut gas)
        .unwrap();
    assert_eq!(bob_value, balance);

    let balance = network
        .query::<_, u64>(contract, (ops::QR_BALANCE, alice_address), &mut gas)
        .unwrap();
    assert_eq!(alice_value, balance);

    let block_height = 1u64;
    let notes = network
        .query::<_, Vec<Note>>(
            contract,
            (ops::QR_NOTES_FROM_HEIGHT, block_height),
            &mut gas,
        )
        .unwrap();
    assert_eq!(4, notes.len());

    let bob_note = notes
        .iter()
        .filter(|n| bob_vk.owns(n.stealth_address()))
        .collect::<Vec<&Note>>()
        .last()
        .cloned()
        .unwrap();
    let bob_refund = bob_note.value(Some(&bob_vk)).unwrap();
    assert!(bob_refund > 0);

    let bob_nullifier = bob_note.gen_nullifier(&bob_ssk);
    let bob_blinding_factor = JubJubScalar::random(&mut rng);
    let bob_fee =
        Note::obfuscated(&mut rng, &bob_psk, bob_refund, bob_blinding_factor);
    let (mut bob_fee, _) = bob_fee.try_into().unwrap();
    let gas_price = 1;
    let gas_limit = 10;
    bob_fee.gas_price = gas_price;
    bob_fee.gas_limit = gas_limit;

    let bob_output_value = bob_refund - gas_limit;
    let bob_output = Note::transparent(&mut rng, &bob_psk, bob_output_value);

    let eve_address = BlsScalar::random(&mut rng);
    let eve_value = bob_value;

    // TODO implement proof verification
    // https://github.com/dusk-network/rusk/issues/194
    let spend_proof_execute = vec![0xfa];
    let call = TransferExecute {
        anchor: root,
        nullifiers: [bob_nullifier].into(),
        fee: bob_fee,
        crossover: None,
        notes: [bob_output].into(),
        spend_proof: spend_proof_execute,
        call: Some(Call::WithdrawFromTransparentToContract {
            from: bob_address,
            to: eve_address,
            value: eve_value,
        }),
    };

    let result = network
        .transact::<_, bool>(contract, call, &mut gas)
        .unwrap();
    assert!(result);

    let balance = network
        .query::<_, u64>(contract, (ops::QR_BALANCE, bob_address), &mut gas)
        .unwrap();
    assert_eq!(0u64, balance);

    let balance = network
        .query::<_, u64>(contract, (ops::QR_BALANCE, eve_address), &mut gas)
        .unwrap();
    assert_eq!(eve_value, balance);
}

#[test]
fn withdraw_from_obfuscated() {
    // TODO Implement obfuscated tests
    // https://github.com/dusk-network/rusk/issues/192
}
