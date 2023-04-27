// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate alloc;

#[allow(unused)]
#[path = "../src/msg.rs"]
mod msg;
use msg::*;

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};
use dusk_pki::{PublicKey, SecretKey};
use piecrust::{ModuleId, Session, VM};
use rand::rngs::StdRng;
use rand::SeedableRng;

const GOVERNANCE_ID: ModuleId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0xf0;
    ModuleId::from_bytes(bytes)
};

const POINT_LIMIT: u64 = 0x10000000;
const TIMESTAMP: u64 = 946681200; // 2000.01.01 00:00

/// Instantiate the virtual machine with the transfer contract deployed, with a
/// single note owned by the given public spend key.
fn instantiate(
    vm: &mut VM,
    authority: &BlsPublicKey,
    broker: &PublicKey,
) -> Session {
    rusk_abi::register_host_queries(vm);

    let governance_bytecode = include_bytes!(
        "../../../target/wasm32-unknown-unknown/release/governance_contract.wasm"
    );

    let mut session = vm.genesis_session();

    session.set_point_limit(POINT_LIMIT);
    rusk_abi::set_block_height(&mut session, 0);

    session
        .deploy_with_id(GOVERNANCE_ID, governance_bytecode)
        .expect("Deploying the stake contract should succeed");

    // Set the broker and the authority of the governance contract
    let _: () = session
        .transact(GOVERNANCE_ID, "set_broker", broker)
        .expect("Setting the broker should succeed");

    let _: () = session
        .transact(GOVERNANCE_ID, "set_authority", authority)
        .expect("Setting the authority should succeed");

    // sets the block height for all subsequent operations to 1
    rusk_abi::set_block_height(&mut session, 1);

    session
}

/// Query the total supply in the governance contract.
fn total_supply(session: &mut Session) -> u64 {
    session
        .query(GOVERNANCE_ID, "total_supply", &())
        .expect("Querying the total supply should succeed")
}

fn balance(session: &mut Session, pk: &PublicKey) -> u64 {
    session
        .query(GOVERNANCE_ID, "balance", pk)
        .expect("Querying the total supply should succeed")
}

#[test]
fn balance_overflow() {
    let rng = &mut StdRng::seed_from_u64(0xbeef);
    let mut vm = VM::ephemeral().expect("Creating a VM should succeed");

    let authority_sk = BlsSecretKey::random(rng);
    let authority = BlsPublicKey::from(&authority_sk);

    let broker = PublicKey::from(&SecretKey::random(rng));

    let alice = PublicKey::from(&SecretKey::random(rng));
    let bob = PublicKey::from(&SecretKey::random(rng));

    let session = &mut instantiate(&mut vm, &authority, &broker);

    assert_eq!(total_supply(session), 0);
    assert_eq!(balance(session, &alice), 0);
    assert_eq!(balance(session, &bob), 0);

    // Make a "mint" call
    let seed = BlsScalar::random(rng);
    let msg = mint_msg(seed, alice, u64::MAX);
    let signature = authority_sk.sign(&authority, &msg);

    let _: () = session
        .transact(GOVERNANCE_ID, "mint", &(signature, seed, alice, u64::MAX))
        .expect("Minting should succeed");

    assert_eq!(total_supply(session), u64::MAX);
    assert_eq!(balance(session, &alice), u64::MAX);
    assert_eq!(balance(session, &bob), 0);

    // Make a "transfer" call
    let transfer = (Some(bob), Some(alice), 200u64, TIMESTAMP);
    let batch = vec![transfer];

    let seed = BlsScalar::random(rng);
    let msg = transfer_msg(seed, &batch);
    let signature = authority_sk.sign(&authority, &msg);

    session
        .transact::<_, ()>(GOVERNANCE_ID, "transfer", &(signature, seed, batch))
        .expect_err("The transaction should fail due to overflow");

    assert_eq!(total_supply(session), u64::MAX);
    assert_eq!(balance(session, &alice), u64::MAX);
    assert_eq!(balance(session, &bob), 0);
}

#[test]
fn same_seed() {
    let rng = &mut StdRng::seed_from_u64(0xbeef);
    let mut vm = VM::ephemeral().expect("Creating a VM should succeed");

    let authority_sk = BlsSecretKey::random(rng);
    let authority = BlsPublicKey::from(&authority_sk);

    let broker = PublicKey::from(&SecretKey::random(rng));

    let session = &mut instantiate(&mut vm, &authority, &broker);

    let seed = BlsScalar::random(rng);
    let msg = pause_msg(seed);
    let signature = authority_sk.sign(&authority, &msg);

    let _: () = session
        .transact(GOVERNANCE_ID, "pause", &(signature, seed))
        .expect("Pausing the contract should succeed");

    let msg = unpause_msg(seed);
    let signature = authority_sk.sign(&authority, &msg);

    session
        .transact::<_, ()>(GOVERNANCE_ID, "unpause", &(signature, seed))
        .expect_err("Unpausing the contract with the same seed error");
}

#[test]
fn wrong_signature() {
    let rng = &mut StdRng::seed_from_u64(0xbeef);
    let mut vm = VM::ephemeral().expect("Creating a VM should succeed");

    let authority_sk = BlsSecretKey::random(rng);
    let authority = BlsPublicKey::from(&authority_sk);

    let broker = PublicKey::from(&SecretKey::random(rng));

    let session = &mut instantiate(&mut vm, &authority, &broker);

    let seed = BlsScalar::random(rng);
    let wrong_message = vec![1, 0, 1, 0, 1, 0];
    let wrong_sig = authority_sk.sign(&authority, &wrong_message);

    session
        .transact::<_, ()>(GOVERNANCE_ID, "pause", &(wrong_sig, seed))
        .expect_err("Pausing the contract with a wrong signature should error");
}

#[test]
fn mint_burn_transfer() {
    let rng = &mut StdRng::seed_from_u64(0xbeef);
    let mut vm = VM::ephemeral().expect("Creating a VM should succeed");

    let authority_sk = BlsSecretKey::random(rng);
    let authority = BlsPublicKey::from(&authority_sk);

    let broker = PublicKey::from(&SecretKey::random(rng));

    let alice = PublicKey::from(&SecretKey::random(rng));
    let bob = PublicKey::from(&SecretKey::random(rng));

    let session = &mut instantiate(&mut vm, &authority, &broker);

    assert_eq!(total_supply(session), 0);
    assert_eq!(balance(session, &alice), 0);
    assert_eq!(balance(session, &bob), 0);

    // Make a "mint" call
    let seed = BlsScalar::random(rng);
    let msg = mint_msg(seed, alice, 100);
    let signature = authority_sk.sign(&authority, &msg);

    let _: () = session
        .transact(GOVERNANCE_ID, "mint", &(signature, seed, alice, 100))
        .expect("Minting should succeed");

    assert_eq!(total_supply(session), 100);
    assert_eq!(balance(session, &alice), 100);
    assert_eq!(balance(session, &bob), 0);

    // Make a "transfer" call
    let transfer = (Some(alice), Some(bob), 200, TIMESTAMP);
    let batch = vec![transfer];

    let seed = BlsScalar::random(rng);
    let msg = transfer_msg(seed, &batch);
    let signature = authority_sk.sign(&authority, &msg);

    session
        .transact::<_, ()>(GOVERNANCE_ID, "transfer", &(signature, seed, batch))
        .expect("The transaction should succeed");

    assert_eq!(total_supply(session), 200);
    assert_eq!(balance(session, &alice), 0);
    assert_eq!(balance(session, &bob), 200);

    // Make a "transfer" call
    let transfer = (Some(bob), Some(alice), 200, TIMESTAMP);
    let batch = vec![transfer];

    let seed = BlsScalar::random(rng);
    let msg = transfer_msg(seed, &batch);
    let signature = authority_sk.sign(&authority, &msg);

    session
        .transact::<_, ()>(GOVERNANCE_ID, "transfer", &(signature, seed, batch))
        .expect("The transaction should succeed");

    assert_eq!(total_supply(session), 200);
    assert_eq!(balance(session, &alice), 200);
    assert_eq!(balance(session, &bob), 0);
}

#[test]
fn fee() {
    let rng = &mut StdRng::seed_from_u64(0xbeef);
    let mut vm = VM::ephemeral().expect("Creating a VM should succeed");

    let authority_sk = BlsSecretKey::random(rng);
    let authority = BlsPublicKey::from(&authority_sk);

    let broker = PublicKey::from(&SecretKey::random(rng));

    let alice = PublicKey::from(&SecretKey::random(rng));
    let bob = PublicKey::from(&SecretKey::random(rng));

    let session = &mut instantiate(&mut vm, &authority, &broker);

    assert_eq!(total_supply(session), 0);
    assert_eq!(balance(session, &alice), 0);
    assert_eq!(balance(session, &bob), 0);
    assert_eq!(balance(session, &broker), 0);

    // Make two fees in a batch
    let batch = vec![
        (Some(alice), Some(bob), 200, TIMESTAMP),
        (Some(alice), Some(bob), 50, TIMESTAMP),
    ];

    let seed = BlsScalar::random(rng);
    let msg = fee_msg(seed, &batch);
    let signature = authority_sk.sign(&authority, &msg);

    session
        .transact::<_, ()>(GOVERNANCE_ID, "fee", &(signature, seed, batch))
        .expect("The fee payment should succeed");

    assert_eq!(total_supply(session), 250);
    assert_eq!(balance(session, &alice), 0);
    assert_eq!(balance(session, &bob), 0);
    assert_eq!(balance(session, &broker), 250);

    // Make four transfers in a batch
    let batch = vec![
        (None, Some(alice), 10, TIMESTAMP),
        (None, Some(bob), 30, TIMESTAMP),
        (Some(bob), None, 20, TIMESTAMP),
        (Some(alice), Some(broker), 100, TIMESTAMP),
    ];

    let seed = BlsScalar::random(rng);
    let msg = transfer_msg(seed, &batch);
    let signature = authority_sk.sign(&authority, &msg);

    session
        .transact::<_, ()>(GOVERNANCE_ID, "transfer", &(signature, seed, batch))
        .expect("The batch processing should succeed");

    assert_eq!(total_supply(session), 260);
    assert_eq!(balance(session, &alice), 0);
    assert_eq!(balance(session, &bob), 10);
    assert_eq!(balance(session, &broker), 250);
}
