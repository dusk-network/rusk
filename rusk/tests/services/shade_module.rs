// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Service test for the `shade_module` lifecycle.
//!
//! The test verifies that a deployed 3rd-party contract:
//! 1. executes successfully before shading,
//! 2. fails with `Unknown` after shading,
//! 3. executes successfully again after recompilation.

use anyhow::Result;
use dusk_bytes::Serializable;
use dusk_core::abi::ContractId;
use dusk_core::transfer::data::ContractCall;
use dusk_rusk_test::{RuskVmConfig, TestContext};
use dusk_vm::gen_contract_id;

use crate::common::logger;

/// Gas limit reused for deployment and reset calls.
const GAS_LIMIT: u64 = 200_000_000;
/// Regular execution gas price.
const GAS_PRICE: u64 = 1;
/// Deployment gas price must respect min deployment gas price.
const DEPLOY_GAS_PRICE: u64 = 2_000;
/// Deploy nonce used to derive the deterministic contract id.
const DEPLOY_NONCE: u64 = 0;
/// Initial bob state value at deployment.
const BOB_INIT_VALUE: u8 = 5;
/// Value set before shading.
const BEFORE_SHADE_VALUE: u8 = 12;
/// Value set after recompiling the shaded module.
const AFTER_RECOMPILE_VALUE: u8 = 42;
/// First block height used by this lifecycle test.
const FIRST_BLOCK_HEIGHT: u64 = 1;

/// Instantiates the test context from the dedicated shade-module snapshot.
async fn shade_state() -> Result<TestContext> {
    let state = include_str!("../config/shade_module.toml");
    let vm_config = RuskVmConfig::new();
    TestContext::instantiate(state, vm_config).await
}

/// Builds a moonlight transaction that calls `reset(value)` on `contract_id`.
fn make_reset_tx(
    tc: &TestContext,
    contract_id: ContractId,
    value: u8,
) -> dusk_core::transfer::Transaction {
    let call = ContractCall::new(contract_id, "reset")
        .with_args(&value)
        .expect("Creating contract call should succeed");

    tc.wallet()
        .moonlight_execute(0, 0, 0, GAS_LIMIT, GAS_PRICE, Some(call))
        .expect("Creating reset transaction should succeed")
}

/// Asserts the contract `value()` query result is equal to `expected`.
fn assert_value(tc: &TestContext, contract_id: ContractId, expected: u8) {
    let value: u8 = tc
        .rusk()
        .query(contract_id, "value", &())
        .expect("Querying value should succeed");
    assert_eq!(value, expected, "Contract value mismatch");
}

/// Asserts shaded modules cannot be queried directly anymore.
fn assert_unreadable(tc: &TestContext, contract_id: ContractId) {
    tc.rusk()
        .query::<(), u8>(contract_id, "value", &())
        .expect_err("Querying value should panic");
}

/// Deploys the `bob` contract and returns its deterministic contract id.
fn deploy(tc: &TestContext) -> ContractId {
    let wallet = tc.wallet();

    let owner = wallet
        .account_public_key(0)
        .expect("Getting owner public key should succeed");
    let bob_bytecode = include_bytes!("../../../contracts/bin/bob.wasm");
    let contract_id =
        gen_contract_id(bob_bytecode, DEPLOY_NONCE, owner.to_bytes());

    let deploy_tx = wallet
        .moonlight_deployment(
            0,
            bob_bytecode,
            &owner,
            vec![BOB_INIT_VALUE],
            GAS_LIMIT,
            DEPLOY_GAS_PRICE,
            DEPLOY_NONCE,
        )
        .expect("Creating deployment transaction should succeed");
    tc.execute_transaction(deploy_tx, FIRST_BLOCK_HEIGHT, None);
    contract_id
}

/// Verifies the shade-module lifecycle for a deployed contract:
/// success before shading, failure while shaded, success after recompilation.
#[tokio::test(flavor = "multi_thread")]
pub async fn shade_module_lifecycle() -> Result<()> {
    logger();

    let tc = shade_state().await?;
    let contract_id = deploy(&tc);

    assert_value(&tc, contract_id, BOB_INIT_VALUE);

    let pre_shade_tx = make_reset_tx(&tc, contract_id, BEFORE_SHADE_VALUE);
    tc.execute_transaction(pre_shade_tx, FIRST_BLOCK_HEIGHT + 1, None);
    assert_value(&tc, contract_id, BEFORE_SHADE_VALUE);

    tc.rusk()
        .shade_3rd_party(contract_id)
        .expect("Shading module should succeed");
    assert_unreadable(&tc, contract_id);

    let shaded_tx = make_reset_tx(&tc, contract_id, BEFORE_SHADE_VALUE);
    tc.execute_transaction(shaded_tx, FIRST_BLOCK_HEIGHT + 2, "Unknown");

    assert_unreadable(&tc, contract_id);

    tc.rusk()
        .recompile_3rd_party(contract_id)
        .expect("Recompiling module should succeed");
    assert_value(&tc, contract_id, BEFORE_SHADE_VALUE);

    let recompiled_tx = make_reset_tx(&tc, contract_id, AFTER_RECOMPILE_VALUE);
    tc.execute_transaction(recompiled_tx, FIRST_BLOCK_HEIGHT + 3, None);
    assert_value(&tc, contract_id, AFTER_RECOMPILE_VALUE);

    Ok(())
}
