// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Integration tests for init-function gas charging during contract
//! deployment.
//!
//! These tests live in their own binary because they activate the Boreas
//! hardfork at genesis, which sets a process-wide `OnceLock` that would
//! conflict with the default localnet configuration (Boreas = NEVER) used
//! by the main test binary.

use std::cmp::max;

use dusk_core::abi::ContractId;
use dusk_core::transfer::data::{
    ContractBytecode, ContractDeploy, TransactionData,
};
use dusk_rusk_test::common::logger;
use dusk_rusk_test::common::state::{ExecuteResult, generator_procedure};
use dusk_rusk_test::{RuskVmConfig, SpentTransaction, TestContext};
use dusk_vm::{FeatureActivation, gen_contract_id};
use rand::SeedableRng;
use rand::rngs::StdRng;

const BLOCK_HEIGHT: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 1_000_000_000_000;
const GAS_LIMIT: u64 = 200_000_000;
const GAS_PRICE: u64 = 2000;
const SENDER_INDEX: u8 = 0;
const BOB_INIT_VALUE: u8 = 5;
const OWNER: [u8; 32] = [1; 32];

// Default VM config values
const GAS_PER_DEPLOY_BYTE: u64 = 100;
const MIN_DEPLOY_POINTS: u64 = 5_000_000;

fn bytecode_hash(bytecode: impl AsRef<[u8]>) -> ContractId {
    let hash = blake3::hash(bytecode.as_ref());
    ContractId::from_bytes(hash.into())
}

/// Compute the deploy charge for the given bytecode, mirroring
/// `Transaction::deploy_charge`.
fn deploy_charge(bytecode: &[u8]) -> u64 {
    max(
        bytecode.len() as u64 * GAS_PER_DEPLOY_BYTE,
        MIN_DEPLOY_POINTS,
    )
}

/// Deploy a contract via a phoenix transaction and return the spent
/// transaction.
fn deploy_contract(
    tc: &TestContext,
    bytecode: &[u8],
    init_args: Option<Vec<u8>>,
) -> SpentTransaction {
    let rusk = tc.rusk();
    let wallet = tc.wallet();
    let mut rng = StdRng::seed_from_u64(0xcafe);

    let hash = bytecode_hash(bytecode).to_bytes();
    let tx = wallet
        .phoenix_execute(
            &mut rng,
            SENDER_INDEX,
            GAS_LIMIT,
            GAS_PRICE,
            0u64,
            TransactionData::Deploy(ContractDeploy {
                bytecode: ContractBytecode {
                    hash,
                    bytes: bytecode.to_vec(),
                },
                owner: OWNER.to_vec(),
                init_args,
                nonce: 0,
            }),
        )
        .expect("Making transaction should succeed");

    let spent_txs = generator_procedure(
        rusk,
        &[tx],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![],
        Some(ExecuteResult {
            discarded: 0,
            executed: 1,
        }),
    )
    .expect("generator procedure should succeed");

    spent_txs
        .into_iter()
        .next()
        .expect("There should be one spent transaction")
}

/// Creates a test context with Boreas hardfork active at height 1.
async fn boreas_state() -> TestContext {
    let state = include_str!("config/contract_deployment.toml");
    let mut vm_config = RuskVmConfig::new();
    vm_config.with_feature("HARDFORK_BOREAS", FeatureActivation::Height(1));
    TestContext::instantiate(state, vm_config)
        .await
        .expect("Initializing should succeed")
}

/// Verify init gas is actually counted by comparing a deployment with init
/// (bob) against one without (alice). After subtracting the deploy-charge
/// difference from bytecode sizes, the remaining surplus proves init gas
/// is charged.
#[tokio::test(flavor = "multi_thread")]
async fn deploy_with_init_charges_more_gas_than_without() {
    logger();
    let tc = boreas_state().await;

    let alice_bytecode = include_bytes!("../../contracts/bin/alice.wasm");
    let bob_bytecode = include_bytes!("../../contracts/bin/bob.wasm");

    // Deploy alice (no init function).
    let spent_alice = deploy_contract(&tc, alice_bytecode, None);
    assert!(spent_alice.err.is_none(), "Alice deployment should succeed");

    // Deploy bob (has init function).
    let spent_bob =
        deploy_contract(&tc, bob_bytecode, Some(vec![BOB_INIT_VALUE]));
    assert!(spent_bob.err.is_none(), "Bob deployment should succeed");

    // Both must respect the gas_limit invariant.
    assert!(spent_alice.gas_spent <= GAS_LIMIT);
    assert!(spent_bob.gas_spent <= GAS_LIMIT);

    // The deploy charge difference is purely from bytecode size.
    let charge_alice = deploy_charge(alice_bytecode);
    let charge_bob = deploy_charge(bob_bytecode);
    let charge_diff = charge_bob.saturating_sub(charge_alice);

    // gas_spent(bob) - gas_spent(alice) must exceed the deploy charge
    // difference: the surplus is the init function gas.
    let gas_diff = spent_bob.gas_spent.saturating_sub(spent_alice.gas_spent);
    assert!(
        gas_diff > charge_diff,
        "Bob's extra gas ({gas_diff}) should exceed the deploy-charge \
         difference ({charge_diff}), proving init gas is charged"
    );
}

/// Deploy bob with init gas charging, verify gas_spent stays within
/// gas_limit, and confirm the contract is functional.
#[tokio::test(flavor = "multi_thread")]
async fn deploy_with_init_gas_charging_succeeds() {
    logger();
    let tc = boreas_state().await;
    let bob_bytecode = include_bytes!("../../contracts/bin/bob.wasm");
    let contract_id = gen_contract_id(bob_bytecode, 0u64, OWNER);

    let spent = deploy_contract(&tc, bob_bytecode, Some(vec![BOB_INIT_VALUE]));
    assert!(spent.err.is_none(), "Deployment should succeed");

    // Critical invariant: gas_spent must never exceed gas_limit.
    // Before the fix, gas_spent could be gas_limit + deploy_charge.
    assert!(
        spent.gas_spent <= GAS_LIMIT,
        "gas_spent ({}) must not exceed gas_limit ({GAS_LIMIT})",
        spent.gas_spent,
    );

    // Verify the contract is deployed and its init ran correctly.
    let value: u8 = tc
        .rusk()
        .query(contract_id, "value", &())
        .expect("Value call should succeed");
    assert_eq!(value, BOB_INIT_VALUE, "Init should have set the value");

    let echo: u64 = tc
        .rusk()
        .query(contract_id, "echo", &775u64)
        .expect("Echo call should succeed");
    assert_eq!(echo, 775);
}
