// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use anyhow::Result;
use dusk_core::JubJubScalar;
use dusk_core::abi::ContractId;
use dusk_core::stake::STAKE_CONTRACT;
use dusk_core::transfer::data::ContractCall;
use dusk_core::transfer::phoenix::{
    PublicKey as PhoenixPublicKey, SecretKey as PhoenixSecretKey,
};
use dusk_core::transfer::withdraw::{
    Withdraw, WithdrawReceiver, WithdrawReplayToken,
};
use dusk_core::transfer::{TRANSFER_CONTRACT, Transaction};
use dusk_rusk_test::TestContext;
use dusk_rusk_test::common::state::{ExecuteResult, generator_procedure};
use ff::Field;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rusk::node::{FEATURE_DISABLE_PHOENIX, RuskVmConfig};

use crate::common::logger;

const BLOCK_HEIGHT: u64 = 2;
const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;
const CONVERT_VALUE: u64 = 5_000_000_000;
const GAS_LIMIT: u64 = 1_000_000_000;
const GAS_PRICE: u64 = 1;

async fn convert_state() -> Result<TestContext> {
    let state = include_str!("../config/convert.toml");
    let mut vm_config =
        RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);
    vm_config.with_feature(FEATURE_DISABLE_PHOENIX, 1);

    TestContext::instantiate(state, vm_config).await
}

fn assert_discarded(tc: &TestContext, tx: Transaction) {
    let spent = generator_procedure(
        tc.rusk(),
        &[tx],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![],
        Some(ExecuteResult {
            executed: 0,
            discarded: 1,
        }),
    )
    .expect("generator procedure should succeed");

    assert!(spent.is_empty(), "discarded tx should not be spent");
}

fn assert_executed_with_phoenix_error(
    tc: &TestContext,
    tx: Transaction,
    block_height: u64,
) {
    let spent = generator_procedure(
        tc.rusk(),
        &[tx],
        block_height,
        BLOCK_GAS_LIMIT,
        vec![],
        Some(ExecuteResult {
            executed: 1,
            discarded: 0,
        }),
    )
    .expect("generator procedure should succeed");

    assert_eq!(spent.len(), 1, "tx should be spent");
    let err = spent[0].err.as_ref().expect("tx should have an error");
    assert!(
        err.contains("phoenix is not enabled in the VM"),
        "error should mention disabled Phoenix, got: {err}",
    );
}

fn withdraw_to_phoenix(contract: ContractId) -> Withdraw {
    let mut rng = StdRng::seed_from_u64(0x51a1e);
    let receiver_sk = PhoenixSecretKey::random(&mut rng);
    let receiver_pk = PhoenixPublicKey::from(&receiver_sk);
    let withdraw_address =
        receiver_pk.gen_stealth_address(&JubJubScalar::random(&mut rng));
    let withdraw_note_sk = receiver_sk.gen_note_sk(&withdraw_address);

    Withdraw::new(
        &mut rng,
        &withdraw_note_sk,
        contract,
        1,
        WithdrawReceiver::Phoenix(withdraw_address),
        WithdrawReplayToken::Moonlight(1),
    )
}

fn moonlight_call(tc: &TestContext, call: ContractCall) -> Transaction {
    tc.wallet()
        .moonlight_transaction(
            0,
            None,
            0,
            0,
            GAS_LIMIT,
            GAS_PRICE,
            Some(call),
            None,
        )
        .expect("creating moonlight transaction should succeed")
}

#[tokio::test(flavor = "multi_thread")]
pub async fn disabled_phoenix_discards_phoenix_transactions() -> Result<()> {
    logger();

    let tc = convert_state().await?;
    let wallet = tc.wallet();
    let mut rng = StdRng::seed_from_u64(0xdead);
    let phoenix_before = wallet
        .get_balance(0)
        .expect("getting phoenix balance should succeed")
        .value;

    let tx = wallet
        .phoenix_to_moonlight(
            &mut rng,
            0,
            0,
            CONVERT_VALUE,
            GAS_LIMIT,
            GAS_PRICE,
        )
        .expect("creating phoenix transaction should succeed");

    assert_discarded(&tc, tx);
    let phoenix_after = wallet
        .get_balance(0)
        .expect("getting phoenix balance should succeed")
        .value;
    assert_eq!(
        phoenix_after, phoenix_before,
        "discarded Phoenix tx must not create a Phoenix refund"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn disabled_phoenix_errors_transfer_convert_to_phoenix() -> Result<()>
{
    logger();

    let tc = convert_state().await?;
    let withdraw = withdraw_to_phoenix(TRANSFER_CONTRACT);
    let call = ContractCall::new(TRANSFER_CONTRACT, "convert")
        .with_args(&withdraw)
        .expect("serializing withdraw should succeed");
    let tx = moonlight_call(&tc, call);

    assert_executed_with_phoenix_error(&tc, tx, BLOCK_HEIGHT);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn disabled_phoenix_errors_transfer_withdraw_to_phoenix() -> Result<()>
{
    logger();

    let tc = convert_state().await?;
    let withdraw = withdraw_to_phoenix(TRANSFER_CONTRACT);
    let call = ContractCall::new(TRANSFER_CONTRACT, "withdraw")
        .with_args(&withdraw)
        .expect("serializing withdraw should succeed");
    let tx = moonlight_call(&tc, call);

    assert_executed_with_phoenix_error(&tc, tx, BLOCK_HEIGHT);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn disabled_phoenix_errors_transfer_mint_to_phoenix() -> Result<()> {
    logger();

    let tc = convert_state().await?;
    let withdraw = withdraw_to_phoenix(STAKE_CONTRACT);
    let call = ContractCall::new(TRANSFER_CONTRACT, "mint")
        .with_args(&withdraw)
        .expect("serializing withdraw should succeed");
    let tx = moonlight_call(&tc, call);

    assert_executed_with_phoenix_error(&tc, tx, BLOCK_HEIGHT);

    Ok(())
}
