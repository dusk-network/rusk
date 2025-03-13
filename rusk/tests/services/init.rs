// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use dusk_core::transfer::data::{ContractCall, TransactionData};
use dusk_vm::{gen_contract_id, ContractData};
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::node::RuskVmConfig;
use rusk::{Result, Rusk};
use rusk_recovery_tools::state;
use tokio::sync::broadcast;

use dusk_core::abi::ContractId;
use node_data::ledger::SpentTransaction;
use tempfile::tempdir;
use tracing::info;

use crate::common::logger;
use crate::common::state::DEFAULT_MIN_GAS_LIMIT;
use crate::common::state::{generator_procedure, ExecuteResult};
use crate::common::wallet::{
    test_wallet as wallet, TestStateClient, TestStore,
};

const BLOCK_HEIGHT: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 1_000_000_000_000;
const INITIAL_BALANCE: u64 = 1_000_000_000_000;

const GAS_LIMIT: u64 = 300_000_000;
const GAS_PRICE: u64 = 1;
const DEPOSIT: u64 = 0;
const BOB_INIT_VALUE: u8 = 5;
const CHAIN_ID: u8 = 0xFA;
const OWNER: [u8; 32] = [1; 32];
const SENDER_INDEX: u8 = 0;

fn initial_state<P: AsRef<Path>>(
    dir: P,
    owner: impl AsRef<[u8]>,
) -> Result<(Rusk, ContractId)> {
    let dir = dir.as_ref();

    let snapshot =
        toml::from_str(include_str!("../config/contract_deployment.toml"))
            .expect("Cannot deserialize config");

    let dusk_key = *rusk::DUSK_CONSENSUS_KEY;
    let bob_bytecode = include_bytes!(
        "../../../target/dusk/wasm32-unknown-unknown/release/bob.wasm"
    );
    let contract_id = gen_contract_id(&bob_bytecode, 0u64, owner.as_ref());
    let deploy = state::deploy(dir, &snapshot, dusk_key, |session| {
        session
            .deploy(
                bob_bytecode,
                ContractData::builder()
                    .owner(owner.as_ref())
                    .init_arg(&BOB_INIT_VALUE)
                    .contract_id(contract_id),
                GAS_LIMIT,
            )
            .expect("Deploying the bob contract should succeed");
    })
    .expect("Deploying initial state should succeed");

    let (_vm, _commit_id) = deploy;

    let (sender, _) = broadcast::channel(10);

    let rusk = Rusk::new(
        dir,
        CHAIN_ID,
        RuskVmConfig::new(),
        DEFAULT_MIN_GAS_LIMIT,
        u64::MAX,
        sender,
    )
    .expect("Instantiating rusk should succeed");
    Ok((rusk, contract_id))
}

fn submit_transaction(
    rusk: &Rusk,
    wallet: &wallet::Wallet<TestStore, TestStateClient>,
    contract_id: &ContractId,
) -> SpentTransaction {
    let initial_balance_0 = wallet
        .get_balance(SENDER_INDEX)
        .expect("Getting initial balance should succeed")
        .value;

    assert_eq!(
        initial_balance_0, INITIAL_BALANCE,
        "The sender should have the given initial balance"
    );

    let mut rng = StdRng::seed_from_u64(0xdead);

    let contract_call = ContractCall {
        contract: *contract_id,
        fn_name: String::from("init"),
        fn_args: vec![0xab],
    };
    let tx_0 = wallet
        .phoenix_execute(
            &mut rng,
            SENDER_INDEX,
            GAS_LIMIT,
            GAS_PRICE,
            DEPOSIT,
            TransactionData::Call(contract_call.clone()),
        )
        .expect("Making the transaction should succeed");

    let expected = ExecuteResult {
        discarded: 0,
        executed: 1,
    };

    let spent_transactions = generator_procedure(
        rusk,
        &[tx_0],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![],
        Some(expected),
    )
    .expect("generator procedure should succeed");

    spent_transactions
        .into_iter()
        .next()
        .expect("There should be one spent transaction")
}

#[tokio::test(flavor = "multi_thread")]
pub async fn calling_init_via_tx_fails() -> Result<()> {
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let (rusk, contract_id) = initial_state(&tmp, OWNER)?;

    let cache = Arc::new(RwLock::new(HashMap::new()));

    let wallet = wallet::Wallet::new(
        TestStore,
        TestStateClient {
            rusk: rusk.clone(),
            cache,
        },
    );

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    assert!(submit_transaction(&rusk, &wallet, &contract_id)
        .err
        .is_some());

    // Check the state's root is changed from the original one
    let new_root = rusk.state_root();
    info!(
        "New root after the 1st transfer: {:?}",
        hex::encode(new_root)
    );
    assert_ne!(original_root, new_root, "Root should have changed");

    Ok(())
}
