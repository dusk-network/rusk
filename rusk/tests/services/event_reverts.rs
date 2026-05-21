// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use anyhow::Result;
use dusk_bytes::Serializable;
use dusk_core::abi::ContractId;
use dusk_core::stake::{DEFAULT_MINIMUM_STAKE, STAKE_CONTRACT, Stake};
use dusk_core::transfer::data::ContractCall;
use dusk_rusk_test::TestContext;
use dusk_vm::{FeatureActivation, gen_contract_id};
use node_data::ledger::{Block, CanonicalTransaction, Header};
use rusk::DUSK_CONSENSUS_KEY;
use rusk::node::{FEATURE_HARDFORK_BOREAS, RuskVmConfig};

use crate::common::*;

const AEGIS_HEIGHT: u64 = 1;
const BOREAS_HEIGHT: u64 = 2;
const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;

const GAS_LIMIT: u64 = 10_000_000_000;
const GAS_PRICE: u64 = 1;
const BLOOM_BYTE_LEN: usize = 256;

// Creates the Rusk initial state for the tests below

async fn aegis_stake_state() -> Result<TestContext> {
    let state = include_str!("../config/stake_from_contract.toml");
    let mut vm_config =
        RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);
    vm_config.with_feature(
        FEATURE_HARDFORK_BOREAS,
        FeatureActivation::Height(BOREAS_HEIGHT),
    );

    TestContext::instantiate(state, vm_config).await
}

#[tokio::test(flavor = "multi_thread")]
pub async fn boreas_discard_reverted_stake_events() -> Result<()> {
    logger();

    let tc = aegis_stake_state().await?;
    let rusk = tc.rusk();
    let wallet = tc.wallet();
    let contract_id = deploy_proxy_contract(&tc);

    let sk = wallet.account_secret_key(0).unwrap();
    let stake = Stake::new_from_contract(
        &sk,
        contract_id,
        DEFAULT_MINIMUM_STAKE,
        rusk.chain_id().unwrap(),
    );
    let call = ContractCall::new(contract_id, "stake_then_panic")
        .with_args(&stake)
        .expect("call to be successful");
    let stake_from_contract = wallet
        .moonlight_execute(
            0,
            0,
            DEFAULT_MINIMUM_STAKE,
            GAS_LIMIT,
            GAS_PRICE,
            Some(call),
        )
        .expect("stake to be successful");

    let node_tx = CanonicalTransaction::canonicalize_for_ledger(
        stake_from_contract,
        BOREAS_HEIGHT,
    )
    .into();
    let prev_state = rusk.state_root();
    let generator = node_data::bls::PublicKey::new(*DUSK_CONSENSUS_KEY);
    let block = Block::new(
        Header {
            height: BOREAS_HEIGHT,
            gas_limit: BLOCK_GAS_LIMIT,
            generator_bls_pubkey: *generator.bytes(),
            state_hash: prev_state,
            ..Default::default()
        },
        vec![node_tx],
        vec![],
    )
    .expect("valid block");

    let cert_voters = vec![(generator, 1)];
    let (spent, transition_result, events, _session) = rusk
        .execute_state_transition(prev_state, &block, &cert_voters)
        .expect("executing transition should succeed");

    assert_eq!(spent.len(), 1);
    assert_eq!(
        spent[0].err.as_deref(),
        Some("Panic: revert after stake_from_contract")
    );
    assert!(
        events.iter().all(|event| !event.event.reverted),
        "reverted events should not be returned"
    );
    assert!(
        !events.iter().any(|event| {
            event.event.target == STAKE_CONTRACT && event.event.topic == "stake"
        }),
        "reverted stake event should not be returned"
    );
    assert!(
        !event_bloom_contains(
            &transition_result.event_bloom,
            STAKE_CONTRACT,
            "stake"
        ),
        "reverted stake event should not be included in bloom"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn aegis_include_reverted_stake_events() -> Result<()> {
    logger();

    let tc = aegis_stake_state().await?;
    let rusk = tc.rusk();
    let wallet = tc.wallet();
    let contract_id = deploy_proxy_contract(&tc);

    let sk = wallet.account_secret_key(0).unwrap();
    let stake = Stake::new_from_contract(
        &sk,
        contract_id,
        DEFAULT_MINIMUM_STAKE,
        rusk.chain_id().unwrap(),
    );
    let call = ContractCall::new(contract_id, "stake_then_panic")
        .with_args(&stake)
        .expect("call to be successful");
    let stake_from_contract = wallet
        .moonlight_execute(
            0,
            0,
            DEFAULT_MINIMUM_STAKE,
            GAS_LIMIT,
            GAS_PRICE,
            Some(call),
        )
        .expect("stake to be successful");

    let node_tx = CanonicalTransaction::canonicalize_for_ledger(
        stake_from_contract,
        AEGIS_HEIGHT,
    )
    .into();
    let prev_state = rusk.state_root();
    let generator = node_data::bls::PublicKey::new(*DUSK_CONSENSUS_KEY);
    let block = Block::new(
        Header {
            height: AEGIS_HEIGHT,
            gas_limit: BLOCK_GAS_LIMIT,
            generator_bls_pubkey: *generator.bytes(),
            state_hash: prev_state,
            ..Default::default()
        },
        vec![node_tx],
        vec![],
    )
    .expect("valid block");

    let cert_voters = vec![(generator, 1)];
    let (spent, transition_result, events, _session) = rusk
        .execute_state_transition(prev_state, &block, &cert_voters)
        .expect("executing transition should succeed");

    assert_eq!(spent.len(), 1);
    assert_eq!(
        spent[0].err.as_deref(),
        Some("Panic: revert after stake_from_contract")
    );
    assert!(
        events.iter().any(|event| {
            event.event.target == STAKE_CONTRACT && event.event.topic == "stake"
        }),
        "reverted stake event should be returned before BOREAS"
    );
    assert!(
        event_bloom_contains(
            &transition_result.event_bloom,
            STAKE_CONTRACT,
            "stake"
        ),
        "reverted stake event should be included in bloom before BOREAS"
    );

    Ok(())
}

fn deploy_proxy_contract(tc: &TestContext) -> ContractId {
    let wallet = tc.wallet();

    let deploy_nonce = 0u64;
    let owner = wallet.account_public_key(0).unwrap();
    let charlie_byte_code =
        include_bytes!("../../../contracts/bin/charlie.wasm");
    let contract_id =
        gen_contract_id(charlie_byte_code, deploy_nonce, owner.to_bytes());
    let tx = wallet
        .moonlight_deployment(
            0,
            charlie_byte_code,
            &owner,
            vec![],
            GAS_LIMIT,
            20000,
            0,
        )
        .expect("Failed to create a deploy transaction");

    tc.execute_transaction(tx, AEGIS_HEIGHT, None);
    contract_id
}

fn event_bloom_contains(
    bloom: &[u8; BLOOM_BYTE_LEN],
    contract: ContractId,
    topic: &str,
) -> bool {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&contract.to_bytes());
    hasher.update(topic.as_bytes());
    let hash = hasher.finalize().into();
    let (i0, v0, i1, v1, i2, v2) = bloom_values(&hash);

    v0 == v0 & bloom[i0] && v1 == v1 & bloom[i1] && v2 == v2 & bloom[i2]
}

fn bloom_values(
    hash: &[u8; blake3::OUT_LEN],
) -> (usize, u8, usize, u8, usize, u8) {
    let v0 = 1 << (hash[1] & 0x7);
    let v1 = 1 << (hash[3] & 0x7);
    let v2 = 1 << (hash[5] & 0x7);

    let i0 = BLOOM_BYTE_LEN
        - ((u16::from_be_bytes([hash[0], hash[1]]) & 0x7ff) >> 3) as usize
        - 1;
    let i1 = BLOOM_BYTE_LEN
        - ((u16::from_be_bytes([hash[2], hash[3]]) & 0x7ff) >> 3) as usize
        - 1;
    let i2 = BLOOM_BYTE_LEN
        - ((u16::from_be_bytes([hash[4], hash[5]]) & 0x7ff) >> 3) as usize
        - 1;

    (i0, v0, i1, v1, i2, v2)
}
