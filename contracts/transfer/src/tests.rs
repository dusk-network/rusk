// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{ops, Call, Transfer, TransferExecute};
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

const CODE: &'static [u8] = include_bytes!("../transfer_contract.wasm");

#[test]
fn withdraw_from_transparent() {
    let mut rng = StdRng::seed_from_u64(2324u64);
    let store = MemStore::new();

    let genesis_ssk = SecretSpendKey::random(&mut rng);
    let genesis_vk = genesis_ssk.view_key();
    let genesis_psk = genesis_ssk.public_spend_key();
    let genesis_value = 1_000_000_000u64;
    let genesis_note = Note::transparent(&mut rng, &genesis_psk, genesis_value);
    let transfer = Transfer::try_from(genesis_note).unwrap();

    let block_height = 1;
    let contract = Contract::new(transfer, CODE.to_vec(), &store).unwrap();
    let mut network =
        NetworkState::<StandardABI<MemStore>, MemStore>::with_block_height(
            block_height,
        );
    let contract = network.deploy(contract).unwrap();
    let mut gas = GasMeter::with_limit(1_000_000_000);

    let address = BlsScalar::random(&mut rng);
    let balance = network
        .query::<_, u64>(contract, (ops::QR_BALANCE, address), &mut gas)
        .unwrap();
    assert_eq!(0u64, balance);

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
    let bob_ssk = SecretSpendKey::random(&mut rng);
    let bob_vk = bob_ssk.view_key();
    let bob_psk = bob_ssk.public_spend_key();
    let bob_value = 100;
    let bob_note = Note::transparent(&mut rng, &bob_psk, bob_value);

    let block_generator_ssk = SecretSpendKey::random(&mut rng);
    let block_generator_psk = block_generator_ssk.public_spend_key();

    let genesis_remainder_value = genesis_value - bob_value;
    let genesis_remainder_blinder = JubJubScalar::random(&mut rng);
    let genesis_remainder_note = Note::obfuscated(
        &mut rng,
        &genesis_psk,
        genesis_remainder_value,
        genesis_remainder_blinder,
    );
    let (mut genesis_remainder_fee, genesis_remainder_crossover) =
        genesis_remainder_note.try_into().unwrap();

    // TODO define the gas price and limit
    // https://github.com/dusk-network/rusk/issues/187
    let gas_price = 1;
    let gas_limit = 1000;
    genesis_remainder_fee.gas_price = gas_price;
    genesis_remainder_fee.gas_limit = gas_limit;

    let spend_proof = vec![0xfa];
    let call = TransferExecute {
        anchor: root,
        nullifiers: [genesis_nullifier].into(),
        fee: genesis_remainder_fee,
        crossover: genesis_remainder_crossover,
        notes: [bob_note].into(),
        spend_proof,
        call: Call::None,
    };

    let result = network
        .transact::<_, bool>(contract, call, &mut gas)
        .unwrap();
    assert!(result);

    let block_height = 1u64;
    let notes = network
        .query::<_, Vec<Note>>(
            contract,
            (ops::QR_NOTES_FROM_HEIGHT, block_height),
            &mut gas,
        )
        .unwrap();
    assert_eq!(3, notes.len());
    let bob_note = notes
        .iter()
        .filter(|note| bob_vk.owns(note.stealth_address()))
        .collect::<Vec<&Note>>()
        .first()
        .cloned()
        .unwrap();
    assert_eq!(bob_value, bob_note.value(None).unwrap());
}
