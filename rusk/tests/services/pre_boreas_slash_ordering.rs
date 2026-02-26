// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use dusk_core::transfer::Transaction as ProtocolTransaction;
use dusk_rusk_test::{Result, RuskVmConfig, TestContext};
use node_data::bls::PublicKeyBytes;
use node_data::ledger::{
    Attestation, Block, Header, IterationsInfo, Transaction as NodeTransaction,
};
use node_data::message::payload::{RatificationResult, Vote};
use rusk::DUSK_CONSENSUS_KEY;
use rusk::node::FEATURE_HARDFORK_BOREAS;

use crate::common::*;

const BLOCK_HEIGHT: u64 = 1;
const BOREAS_ACTIVATION_HEIGHT: u64 = 2;
const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;
const GAS_LIMIT: u64 = 10_000_000_000;
const GAS_PRICE: u64 = 1;

fn resign_moonlight_insecure(
    tx: ProtocolTransaction,
    signer: &dusk_core::signatures::bls::SecretKey,
) -> ProtocolTransaction {
    let ProtocolTransaction::Moonlight(tx) = tx else {
        panic!("expected moonlight transaction");
    };
    let mut bytes = tx.to_var_bytes();
    let sig = signer.sign_insecure(&tx.signature_message()).to_bytes();
    let sig_start = bytes
        .len()
        .checked_sub(sig.len())
        .expect("moonlight tx must include signature bytes");
    bytes[sig_start..].copy_from_slice(&sig);

    ProtocolTransaction::Moonlight(
        dusk_core::transfer::moonlight::Transaction::from_slice(&bytes)
            .expect("re-signed moonlight transaction must deserialize"),
    )
}

async fn stake_state_pre_boreas() -> Result<TestContext> {
    let state = include_str!("../config/stake.toml");
    let mut vm_config =
        RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);
    vm_config.with_feature(FEATURE_HARDFORK_BOREAS, BOREAS_ACTIVATION_HEIGHT);
    TestContext::instantiate(state, vm_config).await
}

/// Pre-Boreas compatibility: slashes are applied after tx execution.
/// We assert execute-time event ordering keeps tx-origin events before
/// hard-slash events at heights below activation.
#[tokio::test(flavor = "multi_thread")]
pub async fn slash_events_follow_tx_events_pre_boreas() -> Result<()> {
    logger();

    let tc = stake_state_pre_boreas().await?;
    let wallet = tc.wallet();
    let rusk = tc.rusk();

    let staker_pk = wallet
        .account_public_key(1)
        .expect("staker pubkey to be available");
    let receiver_pk = wallet
        .account_public_key(2)
        .expect("receiver pubkey to be available");

    let sender_sk = wallet
        .account_secret_key(0)
        .expect("sender account secret key to be available");

    let transfer_tx = wallet
        .moonlight_transfer(0, receiver_pk, 1, GAS_LIMIT, GAS_PRICE)
        .expect("Failed to create transfer tx");
    let transfer_tx = resign_moonlight_insecure(transfer_tx, &sender_sk);
    let node_tx: NodeTransaction = transfer_tx.into();

    let prev_state = rusk.state_root();
    let generator = node_data::bls::PublicKey::new(*DUSK_CONSENSUS_KEY);
    let mut failed_iterations = IterationsInfo::default();
    failed_iterations.att_list.push(Some((
        Attestation {
            result: RatificationResult::Fail(Vote::Invalid([0; 32])),
            ..Default::default()
        },
        PublicKeyBytes(staker_pk.to_bytes()),
    )));
    failed_iterations.att_list.push(Some((
        Attestation {
            result: RatificationResult::Fail(Vote::Invalid([1; 32])),
            ..Default::default()
        },
        PublicKeyBytes(staker_pk.to_bytes()),
    )));

    let block = Block::new(
        Header {
            height: BLOCK_HEIGHT,
            gas_limit: BLOCK_GAS_LIMIT,
            generator_bls_pubkey: *generator.bytes(),
            state_hash: prev_state,
            failed_iterations,
            ..Default::default()
        },
        vec![node_tx],
        vec![],
    )
    .expect("valid block");

    let block_hash = block.header().hash;

    let (_spent, _transition_result, events, _session) = rusk
        .execute_state_transition(prev_state, &block, &[])
        .expect("executing pre-Boreas transition should succeed");

    let first_tx_event = events
        .iter()
        .position(|event| event.origin != block_hash)
        .expect("expected at least one tx-origin event");
    let first_slash_event = events
        .iter()
        .position(|event| {
            event.origin == block_hash
                && (event.event.topic == "slash"
                    || event.event.topic == "hard_slash")
        })
        .expect("expected a slash event");

    assert!(
        first_tx_event < first_slash_event,
        "pre-Boreas compatibility requires slash events after tx events"
    );

    Ok(())
}
