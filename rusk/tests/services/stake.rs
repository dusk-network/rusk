// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::Path;
use std::sync::{Arc, LazyLock, RwLock};

use dusk_pki::SecretSpendKey;
use dusk_wallet_core::{self as wallet, Store};
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::{Result, Rusk};
use rusk_recovery_tools::state::MINIMUM_STAKE;
use std::collections::HashMap;
use tempfile::tempdir;
use tracing::info;

use crate::common::state::{generator_procedure, new_state};
use crate::common::wallet::{TestProverClient, TestStateClient, TestStore};
use crate::common::*;

const BLOCK_HEIGHT: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;
const GAS_LIMIT: u64 = 10_000_000_000;

// Creates the Rusk initial state for the tests below
fn initial_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot = toml::from_str(include_str!("../config/stake.toml"))
        .expect("Cannot deserialize config");

    new_state(dir, &snapshot)
}

static SSK: LazyLock<SecretSpendKey> = LazyLock::new(|| {
    info!("Generating SecretSpendKey");
    TestStore.retrieve_ssk(0).expect("Should not fail in test")
});

// static SK: LazyLock<SecretKey> = LazyLock::new(|| {
//     info!("Generating BLS SecretKey");
//     TestStore.retrieve_sk(0).expect("Should not fail in test")
// });

/// Stakes an amount Dusk and produces a block with this single transaction,
/// checking the stake is set successfully. It then proceeds to withdraw the
/// stake and checking it is correctly withdrawn.
fn wallet_stake(
    rusk: &Rusk,
    wallet: &wallet::Wallet<TestStore, TestStateClient, TestProverClient>,
    value: u64,
) {
    // Sender psk
    let psk = SSK.public_spend_key();

    let mut rng = StdRng::seed_from_u64(0xdead);

    wallet.get_stake(0).expect("stake to not be found");

    let tx = wallet
        .stake(&mut rng, 0, 2, &psk, value, GAS_LIMIT, 1)
        .expect("Failed to stake");
    generator_procedure(rusk, &[tx], BLOCK_HEIGHT, BLOCK_GAS_LIMIT)
        .expect("generator procedure to succeed");

    let stake = wallet.get_stake(2).expect("stake to be found");
    let stake_value = stake.amount.expect("stake should have an amount").0;

    assert_eq!(stake_value, value);

    let _ = wallet.get_stake(0).expect("stake to be found");

    let tx = wallet
        .unstake(&mut rng, 0, 0, &psk, GAS_LIMIT, 1)
        .expect("Failed to unstake");
    generator_procedure(rusk, &[tx], BLOCK_HEIGHT, BLOCK_GAS_LIMIT)
        .expect("generator procedure to succeed");

    let stake = wallet.get_stake(0).expect("stake should still be state");
    assert_eq!(stake.amount, None);

    let tx = wallet
        .withdraw(&mut rng, 0, 1, &psk, GAS_LIMIT, 1)
        .expect("failed to withdraw reward");
    generator_procedure(rusk, &[tx], BLOCK_HEIGHT, BLOCK_GAS_LIMIT)
        .expect("generator procedure to succeed");

    let stake = wallet.get_stake(1).expect("stake should still be state");
    assert_eq!(stake.reward, 0);
}

#[tokio::test(flavor = "multi_thread")]
pub async fn stake() -> Result<()> {
    // Setup the logger
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp)?;

    let cache = Arc::new(RwLock::new(HashMap::new()));

    // Create a wallet
    let wallet = wallet::Wallet::new(
        TestStore,
        TestStateClient {
            rusk: rusk.clone(),
            cache: cache.clone(),
        },
        TestProverClient::default(),
    );

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    // Perform some staking actions.
    wallet_stake(&rusk, &wallet, MINIMUM_STAKE);

    // Check the state's root is changed from the original one
    let new_root = rusk.state_root();
    info!(
        "New root after the 1st transfer: {:?}",
        hex::encode(new_root)
    );
    assert_ne!(original_root, new_root, "Root should have changed");

    // let recv = kadcast_recv.try_recv();
    // let (_, _, h) = recv.expect("Transaction has not been locally
    // propagated"); assert_eq!(h, 0, "Transaction locally propagated with
    // wrong height");

    Ok(())
}
