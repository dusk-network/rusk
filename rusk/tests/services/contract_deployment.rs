// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use execution_core::bytecode::Bytecode;
use execution_core::transfer::{CallOrDeploy, ContractDeploy};
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::{Result, Rusk};
use rusk_abi::Error::ContractDoesNotExist;
use rusk_abi::{ContractData, ContractId};
use rusk_recovery_tools::state;
use tempfile::tempdir;
use test_wallet::{self as wallet};
use tokio::sync::broadcast;
use tracing::info;

use crate::common::logger;
use crate::common::state::{generator_procedure, ExecuteResult};
use crate::common::wallet::{TestProverClient, TestStateClient, TestStore};

const BLOCK_HEIGHT: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 1_000_000_000_000;
const GAS_LIMIT: u64 = 200_000_000;
const GAS_PRICE: u64 = 2;
const POINT_LIMIT: u64 = 0x10000000;
const SENDER_INDEX: u64 = 0;

const ALICE_CONTRACT_ID: ContractId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0xFA;
    ContractId::from_bytes(bytes)
};

const OWNER: [u8; 32] = [1; 32];

const BOB_ECHO_VALUE: u64 = 775;
const BOB_INIT_VALUE: u8 = 5;

fn initial_state<P: AsRef<Path>>(dir: P, deploy_bob: bool) -> Result<Rusk> {
    let dir = dir.as_ref();

    let snapshot =
        toml::from_str(include_str!("../config/contract_deployment.toml"))
            .expect("Cannot deserialize config");

    let (_vm, _commit_id) = state::deploy(dir, &snapshot, |session| {
        let alice_bytecode = include_bytes!(
            "../../../target/dusk/wasm32-unknown-unknown/release/alice.wasm"
        );

        session
            .deploy(
                alice_bytecode,
                ContractData::builder()
                    .owner(OWNER)
                    .contract_id(ALICE_CONTRACT_ID),
                POINT_LIMIT,
            )
            .expect("Deploying the alice contract should succeed");

        if deploy_bob {
            let bob_bytecode = include_bytes!(
                "../../../target/dusk/wasm32-unknown-unknown/release/bob.wasm"
            );

            session
                .deploy(
                    bob_bytecode,
                    ContractData::builder()
                        .owner(OWNER)
                        .constructor_arg(&BOB_INIT_VALUE),
                    POINT_LIMIT,
                )
                .expect("Deploying the alice contract should succeed");
        }
    })
    .expect("Deploying initial state should succeed");

    let (sender, _) = broadcast::channel(10);

    let rusk = Rusk::new(dir, None, u64::MAX, sender)
        .expect("Instantiating rusk should succeed");
    Ok(rusk)
}

fn bytecode_hash(bytecode: impl AsRef<[u8]>) -> ContractId {
    let hash = blake3::hash(bytecode.as_ref());
    ContractId::from_bytes(hash.into())
}

fn make_and_execute_transaction_deploy(
    rusk: &Rusk,
    wallet: &wallet::Wallet<TestStore, TestStateClient, TestProverClient>,
    bytecode: impl AsRef<[u8]>,
    gas_limit: u64,
    init_value: u8,
    should_fail: bool,
) {
    let mut rng = StdRng::seed_from_u64(0xcafe);

    let constructor_args = Some(vec![init_value]);

    let hash = bytecode_hash(bytecode.as_ref()).to_bytes();
    let tx = wallet
        .execute(
            &mut rng,
            CallOrDeploy::Deploy(ContractDeploy {
                bytecode: Bytecode {
                    hash,
                    bytes: bytecode.as_ref().to_vec(),
                },
                owner: OWNER.to_vec(),
                constructor_args,
            }),
            SENDER_INDEX,
            gas_limit,
            GAS_PRICE,
            0u64,
        )
        .expect("Making transaction should succeed");

    let expected = ExecuteResult {
        discarded: 0,
        executed: 1,
    };

    let result = generator_procedure(
        rusk,
        &[tx],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![],
        Some(expected),
    );
    let spent_transactions =
        result.expect("generator procedure should succeed");
    let mut spent_transactions = spent_transactions.into_iter();
    if should_fail {
        let tx = spent_transactions
            .next()
            .expect("There should be one spent transactions");
        assert!(tx.err.is_some(), "Transaction should fail");
    }
}

fn assert_bob_contract_is_not_deployed(
    path: &PathBuf,
    rusk: &Rusk,
    contract_id: &ContractId,
) {
    let commit = rusk.state_root();
    let vm =
        rusk_abi::new_vm(path.as_path()).expect("VM creation should succeed");
    let mut session = rusk_abi::new_session(&vm, commit, 0)
        .expect("Session creation should succeed");
    let result =
        session.call::<_, u64>(*contract_id, "echo", &BOB_ECHO_VALUE, u64::MAX);
    match result.err() {
        Some(ContractDoesNotExist(_)) => (),
        _ => assert!(false),
    }
}

fn assert_bob_contract_is_deployed(
    path: &PathBuf,
    rusk: &Rusk,
    contract_id: &ContractId,
) {
    let commit = rusk.state_root();
    let vm =
        rusk_abi::new_vm(path.as_path()).expect("VM creation should succeed");
    let mut session = rusk_abi::new_session(&vm, commit, 0)
        .expect("Session creation should succeed");
    let result =
        session.call::<_, u64>(*contract_id, "echo", &BOB_ECHO_VALUE, u64::MAX);
    assert_eq!(
        result.expect("Echo call should succeed").data,
        BOB_ECHO_VALUE
    );
    let result = session.call::<_, u8>(*contract_id, "value", &(), u64::MAX);
    assert_eq!(
        result.expect("Value call should succeed").data,
        BOB_INIT_VALUE
    );
}

/// We deploy a contract
#[tokio::test(flavor = "multi_thread")]
pub async fn contract_deploy() {
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp, false).expect("Initializing should succeed");

    let cache = Arc::new(RwLock::new(HashMap::new()));

    let wallet = wallet::Wallet::new(
        TestStore,
        TestStateClient {
            rusk: rusk.clone(),
            cache,
        },
        TestProverClient::default(),
    );

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    let bob_bytecode = include_bytes!(
        "../../../target/dusk/wasm32-unknown-unknown/release/bob.wasm"
    );
    let contract_id = bytecode_hash(bob_bytecode.as_ref());

    let path = tmp.into_path();
    assert_bob_contract_is_not_deployed(&path, &rusk, &contract_id);
    let before_balance = wallet
        .get_balance(0)
        .expect("Getting wallet's balance should succeed")
        .value;
    make_and_execute_transaction_deploy(
        &rusk,
        &wallet,
        bob_bytecode,
        GAS_LIMIT,
        BOB_INIT_VALUE,
        false,
    );
    let after_balance = wallet
        .get_balance(0)
        .expect("Getting wallet's balance should succeed")
        .value;
    assert_bob_contract_is_deployed(&path, &rusk, &contract_id);
    let funds_spent = before_balance - after_balance;
    println!("funds spent={}", funds_spent);
    assert!(funds_spent < GAS_LIMIT * GAS_PRICE);
}

/// We deploy a contract which is already deployed
#[tokio::test(flavor = "multi_thread")]
pub async fn contract_already_deployed() {
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp, true).expect("Initializing should succeed");

    let cache = Arc::new(RwLock::new(HashMap::new()));

    let wallet = wallet::Wallet::new(
        TestStore,
        TestStateClient {
            rusk: rusk.clone(),
            cache,
        },
        TestProverClient::default(),
    );

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    let bob_bytecode = include_bytes!(
        "../../../target/dusk/wasm32-unknown-unknown/release/bob.wasm"
    );
    let contract_id = bytecode_hash(bob_bytecode.as_ref());

    let path = tmp.into_path();
    assert_bob_contract_is_deployed(&path, &rusk, &contract_id);
    let before_balance = wallet
        .get_balance(0)
        .expect("Getting wallet's balance should succeed")
        .value;
    make_and_execute_transaction_deploy(
        &rusk,
        &wallet,
        bob_bytecode,
        GAS_LIMIT,
        BOB_INIT_VALUE,
        true,
    );
    let after_balance = wallet
        .get_balance(0)
        .expect("Getting wallet's balance should succeed")
        .value;
    let funds_spent = before_balance - after_balance;
    println!("funds spent={}", funds_spent);
    assert_eq!(funds_spent, GAS_LIMIT * GAS_PRICE);
}

/// We deploy a contract with a corrupted bytecode
#[tokio::test(flavor = "multi_thread")]
pub async fn contract_deploy_corrupted_bytecode() {
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp, false).expect("Initializing should succeed");

    let cache = Arc::new(RwLock::new(HashMap::new()));

    let wallet = wallet::Wallet::new(
        TestStore,
        TestStateClient {
            rusk: rusk.clone(),
            cache,
        },
        TestProverClient::default(),
    );

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    let bob_bytecode = include_bytes!(
        "../../../target/dusk/wasm32-unknown-unknown/release/bob.wasm"
    );
    let contract_id = bytecode_hash(bob_bytecode.as_ref());

    // let's corrupt the bytecode
    let bob_bytecode = &bob_bytecode[4..];

    let path = tmp.into_path();
    assert_bob_contract_is_not_deployed(&path, &rusk, &contract_id);
    let before_balance = wallet
        .get_balance(0)
        .expect("Getting wallet's balance should succeed")
        .value;
    make_and_execute_transaction_deploy(
        &rusk,
        &wallet,
        bob_bytecode,
        GAS_LIMIT,
        BOB_INIT_VALUE,
        true,
    );
    let after_balance = wallet
        .get_balance(0)
        .expect("Getting wallet's balance should succeed")
        .value;
    let funds_spent = before_balance - after_balance;
    println!("funds spent={}", funds_spent);
    assert_eq!(funds_spent, GAS_LIMIT * GAS_PRICE);
}

/// We deploy different contracts and compare the charge
#[tokio::test(flavor = "multi_thread")]
pub async fn contract_deploy_charge() {
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp, false).expect("Initializing should succeed");

    let cache = Arc::new(RwLock::new(HashMap::new()));

    let wallet = wallet::Wallet::new(
        TestStore,
        TestStateClient {
            rusk: rusk.clone(),
            cache,
        },
        TestProverClient::default(),
    );

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    let bob_bytecode = include_bytes!(
        "../../../target/dusk/wasm32-unknown-unknown/release/bob.wasm"
    );
    // let bob_contract_id = bytecode_hash(bob_bytecode.as_ref());

    let license_bytecode = include_bytes!(
        "../../../target/dusk/wasm32-unknown-unknown/release/license_contract.wasm"
    );
    // let license_contract_id = bytecode_hash(license_bytecode.as_ref());

    let before_balance = wallet
        .get_balance(0)
        .expect("Getting wallet's balance should succeed")
        .value;
    make_and_execute_transaction_deploy(
        &rusk,
        &wallet,
        bob_bytecode,
        GAS_LIMIT,
        BOB_INIT_VALUE,
        false,
    );
    let after_bob_balance = wallet
        .get_balance(0)
        .expect("Getting wallet's balance should succeed")
        .value;
    make_and_execute_transaction_deploy(
        &rusk,
        &wallet,
        license_bytecode,
        GAS_LIMIT,
        0,
        false,
    );
    let after_license_balance = wallet
        .get_balance(0)
        .expect("Getting wallet's balance should succeed")
        .value;
    let bob_deployment_cost = before_balance - after_bob_balance;
    let license_deployment_cost = after_bob_balance - after_license_balance;
    println!("bob deployment cost={}", bob_deployment_cost);
    println!("license deployment cost={}", license_deployment_cost);
    assert!(license_deployment_cost > bob_deployment_cost);
    assert!(license_deployment_cost - bob_deployment_cost > 10_000_000);
}
