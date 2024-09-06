// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use execution_core::{
    transfer::data::{ContractBytecode, ContractDeploy, TransactionData},
    ContractId,
};
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::gen_id::gen_contract_id;
use rusk::{Result, Rusk};
use rusk_abi::{ContractData, PiecrustError};
use rusk_recovery_tools::state;
use tempfile::tempdir;
use test_wallet::{self as wallet, Wallet};
use tokio::sync::broadcast;
use tracing::info;

use crate::common::logger;
use crate::common::state::{generator_procedure, ExecuteResult};
use crate::common::wallet::{TestStateClient, TestStore};

const BLOCK_HEIGHT: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 1_000_000_000_000;
const GAS_LIMIT: u64 = 200_000_000;
const GAS_LIMIT_NOT_ENOUGH_TO_SPEND: u64 = 10_000_000;
const GAS_LIMIT_NOT_ENOUGH_TO_DEPLOY: u64 = 1_200_000;
const GAS_PRICE: u64 = 2000;
const POINT_LIMIT: u64 = 0x10000000;
const SENDER_INDEX: u8 = 0;

const ALICE_CONTRACT_ID: ContractId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0xFA;
    ContractId::from_bytes(bytes)
};

const OWNER: [u8; 32] = [1; 32];
const CHAIN_ID: u8 = 0xFA;

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
                        .init_arg(&BOB_INIT_VALUE)
                        .contract_id(gen_contract_id(
                            &bob_bytecode,
                            0u64,
                            &OWNER,
                        )),
                    POINT_LIMIT,
                )
                .expect("Deploying the bob contract should succeed");
        }
    })
    .expect("Deploying initial state should succeed");

    let (sender, _) = broadcast::channel(10);

    let rusk = Rusk::new(
        dir,
        CHAIN_ID,
        None,
        None,
        None,
        BLOCK_GAS_LIMIT,
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

fn make_and_execute_transaction_deploy(
    rusk: &Rusk,
    wallet: &wallet::Wallet<TestStore, TestStateClient>,
    bytecode: impl AsRef<[u8]>,
    gas_limit: u64,
    init_value: u8,
    should_fail: bool,
    should_discard: bool,
    gas_price: u64,
) {
    let mut rng = StdRng::seed_from_u64(0xcafe);

    let init_args = Some(vec![init_value]);

    let hash = bytecode_hash(bytecode.as_ref()).to_bytes();
    let tx = wallet
        .phoenix_execute(
            &mut rng,
            SENDER_INDEX,
            gas_limit,
            gas_price,
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

    let expected = ExecuteResult {
        discarded: if should_discard { 1 } else { 0 },
        executed: if should_discard { 0 } else { 1 },
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
    if !should_discard {
        let mut spent_transactions = spent_transactions.into_iter();
        let tx = spent_transactions
            .next()
            .expect("There should be one spent transactions");
        if should_fail {
            assert!(tx.err.is_some(), "Transaction should fail");
        } else {
            assert!(tx.err.is_none(), "Transaction should not fail");
        }
    }
}

struct Fixture {
    pub rusk: Rusk,
    pub wallet: Wallet<TestStore, TestStateClient>,
    pub bob_bytecode: Vec<u8>,
    pub contract_id: ContractId,
    pub path: PathBuf,
}

impl Fixture {
    fn build(deploy_bob: bool) -> Self {
        let tmp =
            tempdir().expect("Should be able to create temporary directory");
        let rusk = initial_state(&tmp, deploy_bob)
            .expect("Initializing should succeed");

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

        let bob_bytecode = include_bytes!(
            "../../../target/dusk/wasm32-unknown-unknown/release/bob.wasm"
        )
        .to_vec();
        let contract_id = gen_contract_id(&bob_bytecode, 0u64, &OWNER);

        let path = tmp.into_path();
        Self {
            rusk,
            wallet,
            bob_bytecode,
            contract_id,
            path,
        }
    }

    pub fn assert_bob_contract_is_not_deployed(&self) {
        let commit = self.rusk.state_root();
        let vm = rusk_abi::new_vm(self.path.as_path())
            .expect("VM creation should succeed");
        let mut session = rusk_abi::new_session(&vm, commit, CHAIN_ID, 0)
            .expect("Session creation should succeed");
        let result = session.call::<_, u64>(
            self.contract_id,
            "echo",
            &BOB_ECHO_VALUE,
            u64::MAX,
        );
        match result.err() {
            Some(PiecrustError::ContractDoesNotExist(_)) => (),
            _ => assert!(false),
        }
    }

    pub fn assert_bob_contract_is_deployed(&self) {
        let commit = self.rusk.state_root();
        let vm = rusk_abi::new_vm(self.path.as_path())
            .expect("VM creation should succeed");
        let mut session = rusk_abi::new_session(&vm, commit, CHAIN_ID, 0)
            .expect("Session creation should succeed");
        let result = session.call::<_, u64>(
            self.contract_id,
            "echo",
            &BOB_ECHO_VALUE,
            u64::MAX,
        );
        assert_eq!(
            result.expect("Echo call should succeed").data,
            BOB_ECHO_VALUE
        );
        let result =
            session.call::<_, u8>(self.contract_id, "value", &(), u64::MAX);
        assert_eq!(
            result.expect("Value call should succeed").data,
            BOB_INIT_VALUE
        );
    }

    pub fn wallet_balance(&self) -> u64 {
        self.wallet
            .get_balance(0)
            .expect("Getting wallet's balance should succeed")
            .value
    }
}

/// We deploy a contract.
/// Deployment will succeed and only gas used will be consumed.
/// Wallet will spend (gas used) x GAS_PRICE of funds.
/// Note that gas used will be proportional to the size of bytecode.
#[tokio::test(flavor = "multi_thread")]
pub async fn contract_deploy() {
    logger();
    let f = Fixture::build(false);

    f.assert_bob_contract_is_not_deployed();
    let before_balance = f.wallet_balance();
    make_and_execute_transaction_deploy(
        &f.rusk,
        &f.wallet,
        f.bob_bytecode.clone(),
        GAS_LIMIT,
        BOB_INIT_VALUE,
        false,
        false,
        GAS_PRICE,
    );
    let after_balance = f.wallet_balance();
    f.assert_bob_contract_is_deployed();
    let funds_spent = before_balance - after_balance;
    assert!(funds_spent < GAS_LIMIT * GAS_PRICE);
}

/// We deploy a contract which is already deployed.
/// Deployment will fail and all gas provided will be consumed.
/// Wallet will spend GAS_LIMIT x GAS_PRICE of funds.
#[tokio::test(flavor = "multi_thread")]
pub async fn contract_already_deployed() {
    logger();
    let f = Fixture::build(true);

    f.assert_bob_contract_is_deployed();
    let before_balance = f.wallet_balance();
    make_and_execute_transaction_deploy(
        &f.rusk,
        &f.wallet,
        f.bob_bytecode.clone(),
        GAS_LIMIT,
        BOB_INIT_VALUE,
        true,
        false,
        GAS_PRICE,
    );
    let after_balance = f.wallet_balance();
    let funds_spent = before_balance - after_balance;
    assert_eq!(funds_spent, GAS_LIMIT * GAS_PRICE);
}

/// We deploy a contract with a corrupted bytecode.
/// Deployment will fail and all gas provided will be consumed.
/// Wallet will spend GAS_LIMIT x GAS_PRICE of funds.
#[tokio::test(flavor = "multi_thread")]
pub async fn contract_deploy_corrupted_bytecode() {
    logger();
    let mut f = Fixture::build(false);

    // let's corrupt the bytecode
    f.bob_bytecode = f.bob_bytecode[4..].to_vec();

    f.assert_bob_contract_is_not_deployed();
    let before_balance = f.wallet_balance();
    make_and_execute_transaction_deploy(
        &f.rusk,
        &f.wallet,
        f.bob_bytecode.clone(),
        GAS_LIMIT,
        BOB_INIT_VALUE,
        true,
        false,
        GAS_PRICE,
    );
    let after_balance = f.wallet_balance();
    let funds_spent = before_balance - after_balance;
    assert_eq!(funds_spent, GAS_LIMIT * GAS_PRICE);
}

/// We deploy different contracts and compare the charge.
/// Charge difference should be related to the difference in bytecode sizes.
#[tokio::test(flavor = "multi_thread")]
pub async fn contract_deploy_charge() {
    logger();
    let f = Fixture::build(false);

    let license_bytecode = include_bytes!(
        "../../../target/dusk/wasm32-unknown-unknown/release/license_contract.wasm"
    );

    let before_balance = f.wallet_balance();
    make_and_execute_transaction_deploy(
        &f.rusk,
        &f.wallet,
        f.bob_bytecode.clone(),
        GAS_LIMIT,
        BOB_INIT_VALUE,
        false,
        false,
        GAS_PRICE,
    );
    let after_bob_balance = f.wallet_balance();
    make_and_execute_transaction_deploy(
        &f.rusk,
        &f.wallet,
        license_bytecode,
        GAS_LIMIT,
        0,
        false,
        false,
        GAS_PRICE,
    );
    let after_license_balance = f.wallet_balance();
    let bob_deployment_cost = before_balance - after_bob_balance;
    let license_deployment_cost = after_bob_balance - after_license_balance;
    assert!(license_deployment_cost > bob_deployment_cost);
    assert!(license_deployment_cost - bob_deployment_cost > 10_000_000);
}

/// We deploy a contract with insufficient gas limit.
/// The limit is so small that it is not enough to spend.
/// Transaction will be discarded and no funds will be spent by the wallet.
#[tokio::test(flavor = "multi_thread")]
pub async fn contract_deploy_not_enough_to_spend() {
    logger();
    let f = Fixture::build(false);

    f.assert_bob_contract_is_not_deployed();
    let before_balance = f.wallet_balance();
    make_and_execute_transaction_deploy(
        &f.rusk,
        &f.wallet,
        f.bob_bytecode.clone(),
        GAS_LIMIT_NOT_ENOUGH_TO_SPEND,
        BOB_INIT_VALUE,
        false,
        true,
        GAS_PRICE,
    );
    let after_balance = f.wallet_balance();
    f.assert_bob_contract_is_not_deployed();
    let funds_spent = before_balance - after_balance;
    assert_eq!(funds_spent, 0);
}

/// We deploy a contract with insufficient gas price.
/// Transaction will be discarded.
#[tokio::test(flavor = "multi_thread")]
pub async fn contract_deploy_gas_price_too_low() {
    const LOW_GAS_PRICE: u64 = 10;
    logger();
    let f = Fixture::build(false);

    f.assert_bob_contract_is_not_deployed();
    let before_balance = f.wallet_balance();
    make_and_execute_transaction_deploy(
        &f.rusk,
        &f.wallet,
        f.bob_bytecode.clone(),
        GAS_LIMIT,
        BOB_INIT_VALUE,
        false,
        true,
        LOW_GAS_PRICE,
    );
    let after_balance = f.wallet_balance();
    f.assert_bob_contract_is_not_deployed();
    let funds_spent = before_balance - after_balance;
    assert_eq!(funds_spent, 0);
}

/// We deploy a contract with insufficient gas limit.
/// The limit is such that it is not enough to deploy.
/// Transaction will be discarded and no funds will be spent by the wallet.
#[tokio::test(flavor = "multi_thread")]
pub async fn contract_deploy_gas_limit_too_low() {
    logger();
    let f = Fixture::build(false);

    f.assert_bob_contract_is_not_deployed();
    let before_balance = f.wallet_balance();
    make_and_execute_transaction_deploy(
        &f.rusk,
        &f.wallet,
        f.bob_bytecode.clone(),
        GAS_LIMIT_NOT_ENOUGH_TO_DEPLOY,
        BOB_INIT_VALUE,
        false,
        true,
        GAS_PRICE,
    );
    let after_balance = f.wallet_balance();
    f.assert_bob_contract_is_not_deployed();
    let funds_spent = before_balance - after_balance;
    assert_eq!(funds_spent, 0);
}
