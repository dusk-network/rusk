// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use anyhow::{Result, anyhow};
use dusk_consensus::operations::StateTransitionData;
use dusk_core::abi::ContractId;
use dusk_core::transfer::data::ContractCall;
use dusk_rusk_test::common::state::{ExecuteResult, generator_procedure};
use dusk_rusk_test::{RuskVmConfig, TestContext, Transaction};
use dusk_vm::ContractData;
use node_data::ledger::LedgerTransaction;
use rusk::DUSK_CONSENSUS_KEY;

const HOST_FN_CONTRACT_ID: ContractId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0x48;
    ContractId::from_bytes(bytes)
};

const BOB_CONTRACT_ID: ContractId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0x42;
    ContractId::from_bytes(bytes)
};

const SECOND_BOB_CONTRACT_ID: ContractId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0x43;
    ContractId::from_bytes(bytes)
};

const OWNER: [u8; 32] = [1; 32];

const GAS_LIMIT: u64 = 1_000_000_000;
const GAS_PRICE: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 1_000_000_000_000;

fn query_height(tc: &TestContext) -> Result<u64> {
    tc.rusk()
        .query(HOST_FN_CONTRACT_ID, "block_height", &())
        .map_err(|err| anyhow!("{err}"))
}

fn query_chain_id(tc: &TestContext) -> Result<u8> {
    tc.rusk()
        .query(HOST_FN_CONTRACT_ID, "chain_id", &())
        .map_err(|err| anyhow!("{err}"))
}

fn query_contract_value(
    tc: &TestContext,
    contract_id: ContractId,
) -> Result<u8> {
    tc.rusk()
        .query(contract_id, "value", &())
        .map_err(|err| anyhow!("{err}"))
}

fn query_bob_value(tc: &TestContext) -> Result<u8> {
    query_contract_value(tc, BOB_CONTRACT_ID)
}

fn reset_tx(
    tc: &TestContext,
    contract_id: ContractId,
    value: u8,
    nonce: u64,
) -> Result<Transaction> {
    let call = ContractCall::new(contract_id, "reset")
        .with_args(&value)
        .map_err(|err| anyhow!("{err}"))?;
    tc.wallet()
        .moonlight_transaction(
            0,
            None,
            0,
            0,
            GAS_LIMIT,
            GAS_PRICE,
            Some(call),
            Some(nonce),
        )
        .map_err(|err| anyhow!("{err:?}"))
}

fn unknown_method_tx(
    tc: &TestContext,
    contract_id: ContractId,
    nonce: u64,
) -> Result<Transaction> {
    let call = ContractCall::new(contract_id, "unknown_method");
    tc.wallet()
        .moonlight_transaction(
            0,
            None,
            0,
            0,
            GAS_LIMIT,
            GAS_PRICE,
            Some(call),
            Some(nonce),
        )
        .map_err(|err| anyhow!("{err:?}"))
}

fn execute_block(
    tc: &TestContext,
    txs: &[Transaction],
    block_height: u64,
) -> Result<Vec<dusk_rusk_test::SpentTransaction>> {
    generator_procedure(
        tc.rusk(),
        txs,
        block_height,
        BLOCK_GAS_LIMIT,
        vec![],
        Some(ExecuteResult {
            executed: txs.len(),
            discarded: 0,
        }),
    )
}

async fn instantiate() -> Result<TestContext> {
    let state = include_str!("../config/contract_deployment.toml");
    let host_fn_bytecode =
        include_bytes!("../../../contracts/bin/host_fn.wasm");
    let bob_bytecode = include_bytes!("../../../contracts/bin/bob.wasm");
    TestContext::instantiate_with(state, RuskVmConfig::new(), |session| {
        session
            .deploy::<_, (), _>(
                host_fn_bytecode,
                ContractData::builder()
                    .owner(OWNER)
                    .contract_id(HOST_FN_CONTRACT_ID),
                u64::MAX,
            )
            .expect("deploy host_fn contract");
        session
            .deploy::<_, (), _>(
                bob_bytecode,
                ContractData::builder()
                    .owner(OWNER)
                    .contract_id(BOB_CONTRACT_ID)
                    .init_arg(&0u8),
                u64::MAX,
            )
            .expect("deploy bob contract");
        session
            .deploy::<_, (), _>(
                bob_bytecode,
                ContractData::builder()
                    .owner(OWNER)
                    .contract_id(SECOND_BOB_CONTRACT_ID)
                    .init_arg(&0u8),
                u64::MAX,
            )
            .expect("deploy second bob contract");
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
pub async fn current_query_height_tracks_empty_accepted_blocks() -> Result<()> {
    let tc = instantiate().await?;

    assert_eq!(query_height(&tc)?, 0);
    assert_eq!(query_chain_id(&tc)?, 0xfa);

    tc.empty_block(1)?;
    assert_eq!(query_height(&tc)?, 1);
    assert_eq!(query_chain_id(&tc)?, 0xfa);

    tc.empty_block(2)?;
    assert_eq!(query_height(&tc)?, 2);
    assert_eq!(query_chain_id(&tc)?, 0xfa);

    tc.empty_block(42)?;
    assert_eq!(query_height(&tc)?, 42);
    assert_eq!(query_height(&tc)?, 42);
    assert_eq!(query_chain_id(&tc)?, 0xfa);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn current_query_height_and_state_track_accepted_transaction_blocks()
-> Result<()> {
    let tc = instantiate().await?;
    assert_eq!(query_bob_value(&tc)?, 0);
    assert_eq!(query_height(&tc)?, 0);

    let call = ContractCall::new(BOB_CONTRACT_ID, "reset")
        .with_args(&7u8)
        .expect("reset call");
    let tx = tc
        .wallet()
        .moonlight_execute(0, 0, 0, GAS_LIMIT, GAS_PRICE, Some(call))
        .expect("reset transaction");
    tc.execute_transaction(tx, 1, None);

    assert_eq!(query_bob_value(&tc)?, 7);
    assert_eq!(query_height(&tc)?, 1);
    assert_eq!(query_chain_id(&tc)?, 0xfa);

    let call = ContractCall::new(BOB_CONTRACT_ID, "reset")
        .with_args(&9u8)
        .expect("reset call");
    let tx = tc
        .wallet()
        .moonlight_execute(0, 0, 1, GAS_LIMIT, GAS_PRICE, Some(call))
        .expect("reset transaction");
    tc.execute_transaction(tx, 2, None);

    assert_eq!(query_bob_value(&tc)?, 9);
    assert_eq!(query_height(&tc)?, 2);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn current_query_context_tracks_multi_contract_multi_tx_blocks()
-> Result<()> {
    let tc = instantiate().await?;
    assert_eq!(query_contract_value(&tc, BOB_CONTRACT_ID)?, 0);
    assert_eq!(query_contract_value(&tc, SECOND_BOB_CONTRACT_ID)?, 0);
    assert_eq!(query_height(&tc)?, 0);

    let txs = vec![
        reset_tx(&tc, BOB_CONTRACT_ID, 3, 1)?,
        reset_tx(&tc, SECOND_BOB_CONTRACT_ID, 5, 2)?,
        reset_tx(&tc, BOB_CONTRACT_ID, 7, 3)?,
    ];
    let spent = execute_block(&tc, &txs, 1)?;
    assert_eq!(spent.len(), 3);
    assert!(spent.iter().all(|tx| tx.err.is_none()));

    assert_eq!(query_contract_value(&tc, BOB_CONTRACT_ID)?, 7);
    assert_eq!(query_contract_value(&tc, SECOND_BOB_CONTRACT_ID)?, 5);
    assert_eq!(query_height(&tc)?, 1);
    assert_eq!(query_chain_id(&tc)?, 0xfa);

    let txs = vec![
        reset_tx(&tc, SECOND_BOB_CONTRACT_ID, 11, 4)?,
        reset_tx(&tc, BOB_CONTRACT_ID, 13, 5)?,
        reset_tx(&tc, SECOND_BOB_CONTRACT_ID, 17, 6)?,
    ];
    let spent = execute_block(&tc, &txs, 2)?;
    assert_eq!(spent.len(), 3);
    assert!(spent.iter().all(|tx| tx.err.is_none()));

    assert_eq!(query_contract_value(&tc, BOB_CONTRACT_ID)?, 13);
    assert_eq!(query_contract_value(&tc, SECOND_BOB_CONTRACT_ID)?, 17);
    assert_eq!(query_height(&tc)?, 2);
    assert_eq!(query_chain_id(&tc)?, 0xfa);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn current_query_context_tracks_accepted_block_with_mixed_success_and_failure()
-> Result<()> {
    let tc = instantiate().await?;
    assert_eq!(query_contract_value(&tc, BOB_CONTRACT_ID)?, 0);
    assert_eq!(query_contract_value(&tc, SECOND_BOB_CONTRACT_ID)?, 0);
    assert_eq!(query_height(&tc)?, 0);

    let txs = vec![
        reset_tx(&tc, BOB_CONTRACT_ID, 21, 1)?,
        unknown_method_tx(&tc, SECOND_BOB_CONTRACT_ID, 2)?,
        reset_tx(&tc, SECOND_BOB_CONTRACT_ID, 34, 3)?,
    ];
    let spent = execute_block(&tc, &txs, 1)?;
    assert_eq!(spent.len(), 3);
    assert_eq!(spent[0].err, None);
    assert_eq!(spent[1].err.as_deref(), Some("Unknown"));
    assert_eq!(spent[2].err, None);

    assert_eq!(query_contract_value(&tc, BOB_CONTRACT_ID)?, 21);
    assert_eq!(query_contract_value(&tc, SECOND_BOB_CONTRACT_ID)?, 34);
    assert_eq!(query_height(&tc)?, 1);
    assert_eq!(query_chain_id(&tc)?, 0xfa);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn failed_transaction_in_accepted_block_still_advances_query_height()
-> Result<()> {
    let tc = instantiate().await?;
    assert_eq!(query_height(&tc)?, 0);

    let call = ContractCall::new(BOB_CONTRACT_ID, "unknown_method");
    let tx = tc
        .wallet()
        .moonlight_execute(0, 0, 0, GAS_LIMIT, GAS_PRICE, Some(call))
        .expect("failing transaction");
    tc.execute_transaction(tx, 1, "Unknown");

    assert_eq!(query_bob_value(&tc)?, 0);
    assert_eq!(query_height(&tc)?, 1);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn transition_proposal_does_not_advance_current_query_height()
-> Result<()> {
    let tc = instantiate().await?;
    assert_eq!(query_bob_value(&tc)?, 0);
    assert_eq!(query_height(&tc)?, 0);

    let call = ContractCall::new(BOB_CONTRACT_ID, "reset")
        .with_args(&11u8)
        .expect("reset call");
    let tx = tc
        .wallet()
        .moonlight_execute(0, 0, 0, GAS_LIMIT, GAS_PRICE, Some(call))
        .expect("reset transaction");
    let ledger_tx = LedgerTransaction::from_protocol_for_ledger(tx, 1);
    let generator = node_data::bls::PublicKey::new(*DUSK_CONSENSUS_KEY);
    let transition_data = StateTransitionData {
        round: 1,
        generator: generator.clone(),
        slashes: vec![],
        cert_voters: vec![(generator, 1)],
        max_txs_bytes: usize::MAX,
        prev_state_root: tc.state_root(),
    };

    let (spent, discarded, _) = tc
        .rusk()
        .create_state_transition(&transition_data, [ledger_tx].into_iter())
        .expect("transition proposal should execute");
    assert_eq!(spent.len(), 1);
    assert!(discarded.is_empty());

    assert_eq!(query_bob_value(&tc)?, 0);
    assert_eq!(query_height(&tc)?, 0);
    assert_eq!(query_chain_id(&tc)?, 0xfa);

    Ok(())
}
