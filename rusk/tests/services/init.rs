// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use dusk_core::transfer::data::{
    ContractBytecode, ContractCall, ContractDeploy, TransactionData,
};
use dusk_vm::gen_contract_id;
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
const GAS_PRICE_DEPLOY: u64 = 2000;
const DEPOSIT: u64 = 0;
const BOB_INIT_VALUE: u8 = 5;
const CHAIN_ID: u8 = 0xFA;
const OWNER: [u8; 32] = [1; 32];
const DEPLOYER_INDEX: u8 = 0;
const SENDER_INDEX: u8 = 1;

fn initial_state<P: AsRef<Path>>(
    dir: P,
    owner: impl AsRef<[u8]>,
) -> Result<Rusk> {
    let dir = dir.as_ref();

    let snapshot = toml::from_str(include_str!("../config/init.toml"))
        .expect("Cannot deserialize config");

    let dusk_key = *rusk::DUSK_CONSENSUS_KEY;
    let deploy = state::deploy(dir, &snapshot, dusk_key, |_| {})
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
    Ok(rusk)
}

fn bytecode_hash(bytecode: impl AsRef<[u8]>) -> ContractId {
    let hash = blake3::hash(bytecode.as_ref());
    ContractId::from_bytes(hash.into())
}

fn submit_transactions(
    rusk: &Rusk,
    wallet: &wallet::Wallet<TestStore, TestStateClient>,
) -> SpentTransaction {
    let initial_balance_0 = wallet
        .get_balance(DEPLOYER_INDEX)
        .expect("Getting initial balance should succeed")
        .value;

    assert_eq!(
        initial_balance_0, INITIAL_BALANCE,
        "The deployer should have the given initial balance"
    );
    let initial_balance_1 = wallet
        .get_balance(SENDER_INDEX)
        .expect("Getting initial balance should succeed")
        .value;

    assert_eq!(
        initial_balance_1, INITIAL_BALANCE,
        "The sender should have the given initial balance"
    );

    let mut rng = StdRng::seed_from_u64(0xcafe);

    let bytecode = include_bytes!(
        "../../../target/dusk/wasm32-unknown-unknown/release/bob.wasm"
    );
    let contract_id = gen_contract_id(&bytecode, 0u64, OWNER);

    let init_args = Some(vec![BOB_INIT_VALUE]);

    let hash = bytecode_hash(bytecode.as_ref()).to_bytes();
    let tx_0 = wallet
        .phoenix_execute(
            &mut rng,
            DEPLOYER_INDEX,
            GAS_LIMIT,
            GAS_PRICE_DEPLOY,
            0u64,
            TransactionData::Deploy(ContractDeploy {
                bytecode: ContractBytecode {
                    hash,
                    bytes: bytecode.as_ref().to_vec(),
                },
                owner: OWNER.to_vec(),
                init_args,
                nonce: 0,
            }),
        )
        .expect("Making transaction should succeed");

    let contract_call = ContractCall {
        contract: contract_id,
        fn_name: String::from("init"),
        fn_args: vec![0xab],
    };
    let tx_1 = wallet
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
        executed: 2,
    };

    let spent_transactions = generator_procedure(
        rusk,
        &[tx_0, tx_1],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![],
        Some(expected),
    )
    .expect("generator procedure should succeed");

    assert_eq!(spent_transactions.len(), 2);
    spent_transactions
        .get(1)
        .map(|t| t.clone())
        .expect("There should be one spent transaction")
}

#[tokio::test(flavor = "multi_thread")]
pub async fn calling_init_via_tx_fails() -> Result<()> {
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp, OWNER)?;

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

    assert_eq!(
        submit_transactions(&rusk, &wallet).err,
        Some("Unknown".into())
    );

    // Check the state's root is changed from the original one
    let new_root = rusk.state_root();
    info!(
        "New root after the call transfer: {:?}",
        hex::encode(new_root)
    );
    assert_ne!(original_root, new_root, "Root should have changed");

    Ok(())
}
