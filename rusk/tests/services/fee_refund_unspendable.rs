// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::transfer::data::TransactionData;
use dusk_core::transfer::moonlight::Transaction as MoonlightTransaction;
use dusk_rusk_test::common::state::{ExecuteResult, generator_procedure};
use dusk_rusk_test::{Result, RuskVmConfig, TestContext};
use wallet_core::keys::derive_bls_sk;

use crate::common::logger;

const BLOCK_HEIGHT: u64 = 2;
const TX_GAS_LIMIT: u64 = 1_000_000;
const TX_GAS_PRICE: u64 = 1;
const TRANSFER_VALUE: u64 = 17;

#[tokio::test(flavor = "multi_thread")]
async fn refund_failure_discards_tx_and_preserves_state() -> Result<()> {
    logger();

    let seed = [0u8; 64];
    let sender_sk = derive_bls_sk(&seed, 0);
    let sender_pk = BlsPublicKey::from(&sender_sk);
    let refund_sk = derive_bls_sk(&seed, 1);
    let refund_pk = BlsPublicKey::from(&refund_sk);
    let receiver_sk = derive_bls_sk(&seed, 2);
    let receiver_pk = BlsPublicKey::from(&receiver_sk);

    let state_toml = include_str!("../config/fee_refund_overflow.toml");

    let vm_config = RuskVmConfig::new();
    let block_gas_limit = vm_config.block_gas_limit;
    let tc = TestContext::instantiate(&state_toml, vm_config).await?;
    let rusk = tc.rusk();

    let sender_before = rusk
        .account(&sender_pk)
        .expect("Querying sender account should succeed");
    let refund_before = rusk
        .account(&refund_pk)
        .expect("Querying refund account should succeed");
    let receiver_before = rusk
        .account(&receiver_pk)
        .expect("Querying receiver account should succeed");

    let tx = MoonlightTransaction::new_with_refund(
        &sender_sk,
        &refund_pk,
        Some(receiver_pk),
        TRANSFER_VALUE,
        0,
        TX_GAS_LIMIT,
        TX_GAS_PRICE,
        sender_before.nonce + 1,
        rusk.chain_id().expect("Querying chain id should succeed"),
        None::<TransactionData>,
    )
    .expect("Creating moonlight transaction should succeed")
    .into();

    generator_procedure(
        rusk,
        &[tx],
        BLOCK_HEIGHT,
        block_gas_limit,
        vec![],
        Some(ExecuteResult {
            executed: 0,
            discarded: 1,
        }),
    )?;

    let sender_after = rusk
        .account(&sender_pk)
        .expect("Querying sender account should succeed");
    let refund_after = rusk
        .account(&refund_pk)
        .expect("Querying refund account should succeed");
    let receiver_after = rusk
        .account(&receiver_pk)
        .expect("Querying receiver account should succeed");

    assert_eq!(sender_after, sender_before);
    assert_eq!(refund_after, refund_before);
    assert_eq!(receiver_after, receiver_before);

    Ok(())
}
