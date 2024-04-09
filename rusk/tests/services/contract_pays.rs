// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use dusk_pki::{PublicSpendKey, SecretSpendKey};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use dusk_wallet_core::{self as wallet};
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::{Result, Rusk};
use rusk_abi::{ContractData, ContractId, TRANSFER_CONTRACT};
use rusk_recovery_tools::state;
use tempfile::tempdir;
use tracing::info;

use crate::common::logger;
use crate::common::state::{generator_procedure, ExecuteResult};
use crate::common::wallet::{TestProverClient, TestStateClient, TestStore};

const BLOCK_HEIGHT: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 1_000_000_000_000;
const INITIAL_BALANCE: u64 = 10_000_000_000;
const GAS_LIMIT: u64 = 200_000_000;
const CHARLIES_FUNDS: u64 = 140_000_000;
const POINT_LIMIT: u64 = 0x10000000;

const CHARLIE_CONTRACT_ID: ContractId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0xFC;
    ContractId::from_bytes(bytes)
};

fn initial_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot = toml::from_str(include_str!("../config/contract_pays.toml"))
        .expect("Cannot deserialize config");

    let dir = dir.as_ref();

    let (_vm, _commit_id) = state::deploy(dir, &snapshot, |session| {
        let charlie_bytecode = include_bytes!(
            "../../../target/wasm32-unknown-unknown/release/charlie.wasm"
        );

        let mut rng = StdRng::seed_from_u64(0xcafe);
        let charlie_owner_ssk = SecretSpendKey::random(&mut rng);
        let charlie_owner_psk = PublicSpendKey::from(&charlie_owner_ssk);

        session
            .deploy(
                charlie_bytecode,
                ContractData::builder()
                    .owner(charlie_owner_psk.to_bytes())
                    .contract_id(CHARLIE_CONTRACT_ID),
                POINT_LIMIT,
            )
            .expect("Deploying the charlie contract should succeed");

        session
            .call::<_, ()>(
                TRANSFER_CONTRACT,
                "add_module_balance",
                &(CHARLIE_CONTRACT_ID, CHARLIES_FUNDS),
                u64::MAX,
            )
            .expect("stake contract balance to be set with provisioner stakes");
    })
    .expect("Deploying initial state should succeed");

    let rusk = Rusk::new(dir, None).expect("Instantiating rusk should succeed");
    Ok(rusk)
}

const SENDER_INDEX: u64 = 0;

fn make_and_execute_transaction(
    rusk: &Rusk,
    wallet: &wallet::Wallet<TestStore, TestStateClient, TestProverClient>,
) {
    // We will refund the transaction to ourselves.
    let refund = wallet
        .public_spend_key(SENDER_INDEX)
        .expect("Getting a public spend key should succeed");

    let initial_balance = wallet
        .get_balance(SENDER_INDEX)
        .expect("Getting initial balance should succeed")
        .value;

    assert_eq!(
        initial_balance, INITIAL_BALANCE,
        "The sender should have the given initial balance"
    );

    let mut rng = StdRng::seed_from_u64(0xdead);

    // This transaction should be free for the caller
    let tx = wallet
        .execute(
            &mut rng,
            CHARLIE_CONTRACT_ID.to_bytes().into(),
            String::from("ping"),
            (),
            SENDER_INDEX,
            &refund,
            GAS_LIMIT,
            1,
        )
        .expect("Making the transaction should succeed");

    let expected = ExecuteResult {
        discarded: 0,
        executed: 1,
    };

    let spent_transactions = generator_procedure(
        rusk,
        &[tx],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![],
        Some(expected),
    )
    .expect("generator procedure should succeed");

    let mut spent_transactions = spent_transactions.into_iter();
    let tx = spent_transactions
        .next()
        .expect("There should be one spent transactions");

    assert!(tx.err.is_none(), "Transaction should succeed");
    assert_eq!(tx.gas_spent, 0, "Transaction should be free");
}

#[tokio::test(flavor = "multi_thread")]
pub async fn contract_pays() -> Result<()> {
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp)?;

    let cache = Arc::new(RwLock::new(HashMap::new()));

    let wallet = wallet::Wallet::new(
        TestStore,
        TestStateClient {
            rusk: rusk.clone(),
            cache,
        },
        TestProverClient::default(),
    );

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    make_and_execute_transaction(&rusk, &wallet);

    // Check the state's root is changed from the original one
    let new_root = rusk.state_root();
    info!(
        "New root after the 1st transfer: {:?}",
        hex::encode(new_root)
    );
    assert_ne!(original_root, new_root, "Root should have changed");

    Ok(())
}
