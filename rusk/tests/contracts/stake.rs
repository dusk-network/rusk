// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical_host::{MemStore, Remote, Wasm};
use dusk_bls12_381_sign::{PublicKey, SecretKey, APK};
use dusk_plonk::bls12_381::BlsScalar;
use dusk_plonk::prelude::*;
use rusk::{RuskExternalError, RuskExternals};
use stake_contract::{Contract, Counter};

const BYTECODE: &'static [u8] = include_bytes!(
    "../../contracts/stake/target/wasm32-unknown-unknown/release/stake_contract.wasm"
);

#[test]
fn stake_call_works() {
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
    assert_eq!(block_height + 250_000, stake.expiration);
}

#[test]
fn stake_call_wrong_values() {
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
fn extend_stake_call_works() {
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
fn extend_stake_call_wrong_sig() {
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

#[test]
fn extend_stake_call_wrong_w_i() {
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
    // Create a counter and put it on the wrong number
    let mut w_i2 = Counter::default();
    w_i2.increment();
    let res = cast
        .transact(
            &Contract::<MemStore>::extend_stake(w_i2, apk, sig),
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

#[test]
fn withdraw_stake_call_works() {
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

    // Attempt to withdraw stake
    let block_height = 300_000;
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
            &Contract::<MemStore>::withdraw_stake(block_height, w_i, apk, sig),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the withdraw_stake fn");
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

    assert!(stake.is_none());
}

#[test]
fn withdraw_stake_call_wrong_sig() {
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

    // Attempt to withdraw stake
    let block_height = 300_000;
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
            &Contract::<MemStore>::withdraw_stake(block_height, w_i, apk, sig),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the withdraw_stake fn");
    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(res == false);

    // Fetch stake and see if it is still there
    let stake = cast
        .query(
            &Contract::<MemStore>::find_stake(w_i, apk),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the find_stake fn");

    assert!(stake.is_some());
}

#[test]
fn withdraw_stake_call_wrong_key() {
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

    // Attempt to withdraw stake
    let block_height = 300_000;
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
    let sig = sk.sign(&pk, &BlsScalar::from(stake.expiration).to_bytes());
    let mut cast = remote
        .cast_mut::<Wasm<Contract<MemStore>, MemStore>>()
        .unwrap();
    let mut w_i2 = Counter::default();
    w_i2.increment();
    let res = cast
        .transact(
            &Contract::<MemStore>::withdraw_stake(block_height, w_i2, apk, sig),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the withdraw_stake fn");
    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(res == false);

    // Fetch stake and see if it is still there
    let stake = cast
        .query(
            &Contract::<MemStore>::find_stake(w_i, apk),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the find_stake fn");

    assert!(stake.is_some());
}

#[test]
fn slash_call_works() {
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
    let (_, res) = cast
        .transact(
            &Contract::<MemStore>::stake(block_height, value, apk),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the stake fn");
    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(res == true);

    // Attempt to slash our stake
    let message_1 = BlsScalar::from(100u64);
    let sig_1 = sk.sign(&pk, &message_1.to_bytes());
    let message_2 = BlsScalar::from(200u64);
    let sig_2 = sk.sign(&pk, &message_2.to_bytes());

    let mut cast = remote
        .cast_mut::<Wasm<Contract<MemStore>, MemStore>>()
        .unwrap();
    let res = cast
        .transact(
            &Contract::<MemStore>::slash(
                apk, 15, 2, message_1, message_2, sig_1, sig_2,
            ),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the slash fn");
    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(res == true);

    // TODO: no further action on the stake is currently specified, so we can't
    // test more than this
}

#[test]
fn slash_call_same_message() {
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
    let (_, res) = cast
        .transact(
            &Contract::<MemStore>::stake(block_height, value, apk),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the stake fn");
    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(res == true);

    // Attempt to slash our stake
    let message_1 = BlsScalar::from(100u64);
    let sig_1 = sk.sign(&pk, &message_1.to_bytes());
    let message_2 = BlsScalar::from(100u64);
    let sig_2 = sk.sign(&pk, &message_2.to_bytes());

    let mut cast = remote
        .cast_mut::<Wasm<Contract<MemStore>, MemStore>>()
        .unwrap();
    let res = cast
        .transact(
            &Contract::<MemStore>::slash(
                apk, 15, 2, message_1, message_2, sig_1, sig_2,
            ),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the slash fn");
    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(res == false);
}

#[test]
fn slash_call_wrong_sig() {
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
    let (_, res) = cast
        .transact(
            &Contract::<MemStore>::stake(block_height, value, apk),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the stake fn");
    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(res == true);

    // Attempt to slash our stake
    let message_1 = BlsScalar::from(100u64);
    let sig_1 = sk.sign(&pk, &message_1.to_bytes());
    let message_2 = BlsScalar::from(200u64);
    let sig_2 = sk.sign(&pk, &message_1.to_bytes());

    let mut cast = remote
        .cast_mut::<Wasm<Contract<MemStore>, MemStore>>()
        .unwrap();
    let res = cast
        .transact(
            &Contract::<MemStore>::slash(
                apk, 15, 2, message_1, message_2, sig_1, sig_2,
            ),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the slash fn");
    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(res == false);
}
