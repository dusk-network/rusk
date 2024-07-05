// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

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
const WALLET_BALANCE: u64 = 10_000_000_000;
const GAS_LIMIT: u64 = 200_000_000;
const GAS_PRICE: u64 = 2;
const POINT_LIMIT: u64 = 0x10000000;
const SENDER_INDEX: u64 = 0;

const ALICE_CONTRACT_ID: ContractId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0xFA;
    ContractId::from_bytes(bytes)
};
const ALICE_OWNER: [u8; 32] = [0; 32];

const BOB_CONTRACT_ID: ContractId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0xFB;
    ContractId::from_bytes(bytes)
};

const BOB_OWNER: [u8; 32] = [1; 32];

const BOB_ECHO_VALUE: u64 = 775;
const BOB_INIT_VALUE: u8 = 5;

fn initial_state<P: AsRef<Path>>(dir: P, deploy_bob: bool) -> Result<Rusk> {
    let dir = dir.as_ref();

    let snapshot = toml::from_str(include_str!("../config/contract_pays.toml"))
        .expect("Cannot deserialize config");

    let (_vm, _commit_id) = state::deploy(dir, &snapshot, |session| {
        let alice_bytecode = include_bytes!(
            "../../../target/dusk/wasm32-unknown-unknown/release/alice.wasm"
        );

        session
            .deploy(
                alice_bytecode,
                ContractData::builder()
                    .owner(ALICE_OWNER)
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
                        .owner(BOB_OWNER)
                        .contract_id(BOB_CONTRACT_ID)
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

fn make_and_execute_transaction_deploy(
    rusk: &Rusk,
    wallet: &wallet::Wallet<TestStore, TestStateClient, TestProverClient>,
    bytecode: impl AsRef<[u8]>,
    contract_id: &ContractId,
    gas_limit: u64,
    init_value: u8,
    should_discard: bool,
) {
    let initial_balance = wallet
        .get_balance(SENDER_INDEX)
        .expect("Getting initial balance should succeed")
        .value;

    assert_eq!(
        initial_balance, WALLET_BALANCE,
        "The sender should have the given initial balance"
    );

    let mut rng = StdRng::seed_from_u64(0xcafe);

    let constructor_args = Some(vec![init_value]);

    let tx = wallet
        .execute(
            &mut rng,
            CallOrDeploy::Deploy(ContractDeploy {
                contract_id: Some(contract_id.to_bytes()),
                bytecode: bytecode.as_ref().to_vec(),
                owner: BOB_OWNER.to_vec(),
                constructor_args,
            }),
            SENDER_INDEX,
            gas_limit,
            GAS_PRICE,
            0u64,
        )
        .expect("Making transaction should succeed");

    let expected = if should_discard {
        ExecuteResult {
            discarded: 1,
            executed: 0,
        }
    } else {
        ExecuteResult {
            discarded: 0,
            executed: 1,
        }
    };

    let result = generator_procedure(
        rusk,
        &[tx],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![],
        Some(expected),
    );
    if should_discard {
        assert!(result.is_err())
    } else {
        let spent_transactions =
            result.expect("generator procedure should succeed");
        let mut spent_transactions = spent_transactions.into_iter();
        let tx = spent_transactions
            .next()
            .expect("There should be one spent transactions");
        assert!(tx.err.is_none(), "Transaction should succeed");
    }
}

fn assert_bob_contract_is_not_deployed(path: &PathBuf, rusk: &Rusk) {
    let commit = rusk.state_root();
    let vm =
        rusk_abi::new_vm(path.as_path()).expect("VM creation should succeed");
    let mut session = rusk_abi::new_session(&vm, commit, 0)
        .expect("Session creation should succeed");
    let result = session.call::<_, u64>(
        BOB_CONTRACT_ID,
        "echo",
        &BOB_ECHO_VALUE,
        u64::MAX,
    );
    match result.err() {
        Some(ContractDoesNotExist(_)) => (),
        _ => assert!(false),
    }
}

fn assert_bob_contract_is_deployed(path: &PathBuf, rusk: &Rusk) {
    let commit = rusk.state_root();
    let vm =
        rusk_abi::new_vm(path.as_path()).expect("VM creation should succeed");
    let mut session = rusk_abi::new_session(&vm, commit, 0)
        .expect("Session creation should succeed");
    let result = session.call::<_, u64>(
        BOB_CONTRACT_ID,
        "echo",
        &BOB_ECHO_VALUE,
        u64::MAX,
    );
    assert_eq!(
        result.expect("Echo call should succeed").data,
        BOB_ECHO_VALUE
    );
    let result = session.call::<_, u8>(BOB_CONTRACT_ID, "value", &(), u64::MAX);
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

    let path = tmp.into_path();
    assert_bob_contract_is_not_deployed(&path, &rusk);
    let before_balance = wallet
        .get_balance(0)
        .expect("Getting wallet's balance should succeed")
        .value;
    make_and_execute_transaction_deploy(
        &rusk,
        &wallet,
        bob_bytecode,
        &BOB_CONTRACT_ID,
        GAS_LIMIT,
        BOB_INIT_VALUE,
        false,
    );
    let after_balance = wallet
        .get_balance(0)
        .expect("Getting wallet's balance should succeed")
        .value;
    assert_bob_contract_is_deployed(&path, &rusk);
    println!("total cost={}", before_balance - after_balance);
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

    let path = tmp.into_path();
    assert_bob_contract_is_deployed(&path, &rusk);
    make_and_execute_transaction_deploy(
        &rusk,
        &wallet,
        bob_bytecode,
        &BOB_CONTRACT_ID,
        GAS_LIMIT,
        BOB_INIT_VALUE,
        true,
    );
}
