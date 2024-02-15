// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use dusk_wallet_core::{self as wallet};
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::{Result, Rusk};
use rusk_abi::{
    ContractId, TRANSFER_CONTRACT, TRANSFER_DATA_CONTRACT,
    TRANSFER_LOGIC_CONTRACT,
};
use tempfile::tempdir;

use crate::common::logger;
use crate::common::state::{generator_procedure, new_state};
use crate::common::wallet::{TestProverClient, TestStateClient, TestStore};

const BLOCK_HEIGHT: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 1_000_000_000_000;

const GAS_LIMIT: u64 = 200_000_000;

fn initial_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot = toml::from_str(include_str!("../config/protected.toml"))
        .expect("Cannot deserialize config");

    new_state(dir, &snapshot)
}

const SENDER_INDEX: u64 = 0;

fn verify_protected_method(
    contract: ContractId,
    protected_method: &str,
    rusk: &Rusk,
    wallet: &wallet::Wallet<TestStore, TestStateClient, TestProverClient>,
) {
    let mut rng = StdRng::seed_from_u64(0xcafe);

    let refund = wallet
        .public_spend_key(SENDER_INDEX)
        .expect("Getting a public spend key should succeed");

    let tx = wallet
        .execute(
            &mut rng,
            contract.to_bytes().into(),
            String::from(protected_method),
            (),
            SENDER_INDEX,
            &refund,
            GAS_LIMIT,
            1,
        )
        .expect("Making the transaction should succeed");

    let spent_transactions = generator_procedure(
        rusk,
        &[tx],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![],
        None,
    )
    .expect("generator procedure should succeed");

    assert!(spent_transactions
        .first()
        .expect("Transaction should exist")
        .err
        .is_some());
}

pub fn test_protected_method(method: &str, contract: ContractId) -> Result<()> {
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

    verify_protected_method(contract, method, &rusk, &wallet);

    Ok(())
}

const TRANSFER_DATA_PROTECTED_METHODS: &'static [&'static str] = &[
    "root",
    "num_notes",
    "module_balance",
    "message",
    "opening",
    "existing_nullifiers",
    "any_nullifier_exists",
    "extend_nullifiers",
    "take_message_from_address_key",
    "root_exists",
    "push_message",
    "take_crossover",
    "set_crossover",
    "get_crossover",
    "extend_notes",
    "sub_balance",
    "leaves_from_height",
    "leaves_from_pos",
    "push_note",
    "get_note",
    "update_root",
    "add_module_balance",
    "get_module_balance",
];

const TRANSFER_LOGIC_PROTECTED_METHODS: &'static [&'static str] = &[
    "mint",
    "stct",
    "wfct",
    "wfct_raw",
    "stco",
    "wfco",
    "wfco_raw",
    "wfctc",
    "root",
    "num_notes",
    "module_balance",
    "message",
    "opening",
    "existing_nullifiers",
    "leaves_from_height",
    "leaves_from_pos",
    "spend",
    "execute",
    "refund",
    "push_note",
    "update_root",
    "add_module_balance",
    "sub_module_balance",
];

const TRANSFER_PROXY_PROTECTED_METHODS: &'static [&'static str] = &[
    "spend_and_execute",
    "refund",
    "push_note",
    "update_root",
    "add_module_balance",
];

#[tokio::test(flavor = "multi_thread")]
pub async fn protected_transfer_methods() -> Result<()> {
    for method in TRANSFER_DATA_PROTECTED_METHODS {
        test_protected_method(*method, TRANSFER_DATA_CONTRACT)?;
    }
    for method in TRANSFER_LOGIC_PROTECTED_METHODS {
        test_protected_method(*method, TRANSFER_LOGIC_CONTRACT)?;
    }
    for method in TRANSFER_PROXY_PROTECTED_METHODS {
        test_protected_method(*method, TRANSFER_CONTRACT)?;
    }
    Ok(())
}
