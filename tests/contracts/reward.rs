// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical_host::{MemStore, Remote, Wasm};
use dusk_bls12_381_sign::{PublicKey, SecretKey, APK};
use dusk_plonk::bls12_381::BlsScalar;
use dusk_plonk::prelude::*;
use reward_contract::{Contract, PublicKeys};
use rusk::{RuskExternalError, RuskExternals};

const BYTECODE: &'static [u8] = include_bytes!(
    "../../contracts/reward/target/wasm32-unknown-unknown/release/reward_contract.wasm"
);

#[test]
fn distribute_call_works() {
    // Init Env & Contract
    let store = MemStore::new();
    let wasm_contract = Wasm::new(Contract::new(), BYTECODE);
    let mut remote = Remote::new(wasm_contract, &store).unwrap();

    let value = 100u64;
    // Create 128 public keys
    let mut keys = [APK::default(); 128];
    for i in 0..keys.len() {
        let sk = SecretKey::new(&mut rand_core::OsRng);
        let pk = PublicKey::from(&sk);
        let apk = APK::from(&pk);
        keys[i] = apk;
    }

    let pks = PublicKeys::from(keys.clone());

    // Call distribute
    let mut cast = remote
        .cast_mut::<Wasm<Contract<MemStore>, MemStore>>()
        .unwrap();
    let res = cast
        .transact(
            &Contract::<MemStore>::distribute(value, pks),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the distribute fn");
    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(res == true);

    // Fetch values and see if it stored correctly
    keys.iter().for_each(|pk| {
        let balance = cast
            .query(
                &Contract::<MemStore>::get_balance(*pk),
                store.clone(),
                RuskExternals::default(),
            )
            .expect("Failed to call the get_balance fn");

        assert!(balance.is_some());
        let balance = balance.unwrap();
        assert_eq!(balance, value);
    });
}

/*
#[test]
fn withdraw_call_works() {
    // Init Env & Contract
    let store = MemStore::new();
    let wasm_contract = Wasm::new(Contract::new(), BYTECODE);
    let mut remote = Remote::new(wasm_contract, &store).unwrap();
    // Create Stake and send it
    let block_height = 150;
    // Choose a value that is too low
    let value = 1_000_000u64;
    let sk = SecretKey::new(&mut rand_core::OsRng);
    let pk = PublicKey::from(&sk);
    let apk = APK::from(&pk);

    // Add stake to the Contract's map
    let mut cast = remote
        .cast_mut::<Wasm<Contract<MemStore>, MemStore>>()
        .unwrap();
    let (w_i, res) = cast
        .transact(
            &Contract::<MemStore>::stake(block_height, value, apk),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the stake fn");
    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(res == false);

    // Fetch stake and see if it stored correctly
    let stake = cast
        .query(
            &Contract::<MemStore>::find_stake(w_i, apk),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the find_stake fn");

    assert!(stake.is_none());
}

#[test]
fn withdraw_call_wrong_values() {
    // Init Env & Contract
    let store = MemStore::new();
    let wasm_contract = Wasm::new(Contract::new(), BYTECODE);
    let mut remote = Remote::new(wasm_contract, &store).unwrap();
    // Create Stake and send it
    let block_height = 150;
    let value = 1_000_000_000_000_000u64;
    let sk = SecretKey::new(&mut rand_core::OsRng);
    let pk = PublicKey::from(&sk);
    let apk = APK::from(&pk);

    // Add stake to the Contract's map
    let mut cast = remote
        .cast_mut::<Wasm<Contract<MemStore>, MemStore>>()
        .unwrap();
    let (w_i, res) = cast
        .transact(
            &Contract::<MemStore>::stake(block_height, value, apk),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the stake fn");
    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(res == true);

    // Attempt to extend stake
    let stake = cast
        .query(
            &Contract::<MemStore>::find_stake(w_i, apk),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the find_stake fn");

    assert!(stake.is_some());
    let stake = stake.unwrap();
    let sig = sk.sign(&pk, &BlsScalar::from(stake.expiration).to_bytes());
    let mut cast = remote
        .cast_mut::<Wasm<Contract<MemStore>, MemStore>>()
        .unwrap();
    let res = cast
        .transact(
            &Contract::<MemStore>::extend_stake(w_i, apk, sig),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the extend_stake fn");
    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(res == true);

    // Fetch stake and see if it stored correctly
    let stake = cast
        .query(
            &Contract::<MemStore>::find_stake(w_i, apk),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the find_stake fn");

    assert!(stake.is_some());
    let stake = stake.unwrap();
    assert_eq!(value, stake.value);
    assert_eq!(apk, stake.pk);
    assert_eq!(block_height, stake.eligibility);
    assert_eq!(block_height + 500_000, stake.expiration);
}

#[test]
fn withdraw_call_wrong_sig() {
    // Init Env & Contract
    let store = MemStore::new();
    let wasm_contract = Wasm::new(Contract::new(), BYTECODE);
    let mut remote = Remote::new(wasm_contract, &store).unwrap();
    // Create Stake and send it
    let block_height = 150;
    let value = 1_000_000_000_000_000u64;
    let sk = SecretKey::new(&mut rand_core::OsRng);
    let pk = PublicKey::from(&sk);
    let apk = APK::from(&pk);

    // Add stake to the Contract's map
    let mut cast = remote
        .cast_mut::<Wasm<Contract<MemStore>, MemStore>>()
        .unwrap();
    let (w_i, res) = cast
        .transact(
            &Contract::<MemStore>::stake(block_height, value, apk),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the stake fn");
    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(res == true);

    // Attempt to extend stake
    let stake = cast
        .query(
            &Contract::<MemStore>::find_stake(w_i, apk),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the find_stake fn");

    assert!(stake.is_some());
    let stake = stake.unwrap();
    // Mess up the signature
    let sig = sk.sign(&pk, &BlsScalar::from(stake.expiration + 1).to_bytes());
    let mut cast = remote
        .cast_mut::<Wasm<Contract<MemStore>, MemStore>>()
        .unwrap();
    let res = cast
        .transact(
            &Contract::<MemStore>::extend_stake(w_i, apk, sig),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the extend_stake fn");
    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(res == false);

    // Fetch stake and see if it hasn't changed
    let stake = cast
        .query(
            &Contract::<MemStore>::find_stake(w_i, apk),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the find_stake fn");

    assert!(stake.is_some());
    let stake = stake.unwrap();
    assert_eq!(value, stake.value);
    assert_eq!(apk, stake.pk);
    assert_eq!(block_height, stake.eligibility);
    assert_eq!(block_height + 250_000, stake.expiration);
}
*/
