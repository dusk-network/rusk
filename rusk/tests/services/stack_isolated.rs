// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use dusk_consensus::operations::StateTransitionData;
use dusk_core::abi::ContractId;
use dusk_core::signatures::bls;
use dusk_core::stake::{
    Stake, StakeAmount, StakeData, StakeKeys, DEFAULT_MINIMUM_STAKE,
    STAKE_CONTRACT,
};
use dusk_core::transfer::data::ContractCall;
use dusk_core::transfer::TRANSFER_CONTRACT;
use dusk_vm::ContractData;
use node_data::ledger::Transaction as NodeTransaction;
use rusk::node::{DriverStore, RuskVmConfig, FEATURE_ABI_PUBLIC_SENDER};
use rusk::{Error, Result, Rusk, DUSK_CONSENSUS_KEY};
use rusk_recovery_tools::state::restore_state;
use tempfile::TempDir;
use tokio::sync::broadcast;
use tracing::info;
use wallet_core::transaction::{moonlight, moonlight_stake_reward};

use crate::common::logger;
use crate::common::state::DEFAULT_MIN_GAS_LIMIT;
use crate::common::wallet::{
    test_wallet as wallet, test_wallet::Wallet, TestStateClient, TestStore,
};

const GAS_LIMIT: u64 = 0x10000000;

const NON_BLS_OWNER: [u8; 32] = [1; 32];

const ALICE_ID: ContractId = ContractId::from_bytes([3; 32]);
const CHARLIE_ID: ContractId = ContractId::from_bytes([4; 32]);

const CHAIN_ID: u8 = 0x01;

async fn initial_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let dir = dir.as_ref();

    let (_vm, _commit_id) = restore_state(dir)?;

    let (sender, _) = broadcast::channel(10);

    #[cfg(feature = "archive")]
    let archive_dir =
        tempdir().expect("Should be able to create temporary directory");
    #[cfg(feature = "archive")]
    let archive =
        node::archive::Archive::create_or_open(archive_dir.path()).await;

    let mut vm_config =
        RuskVmConfig::new().with_block_gas_limit(10_000_000_000);
    vm_config.with_feature(FEATURE_ABI_PUBLIC_SENDER, 1);

    let rusk = Rusk::new(
        dir,
        CHAIN_ID,
        vm_config,
        DEFAULT_MIN_GAS_LIMIT,
        u64::MAX,
        sender,
        #[cfg(feature = "archive")]
        archive,
        DriverStore::new(None::<PathBuf>),
    )
    .expect("Instantiating rusk should succeed");
    Ok(rusk)
}

#[allow(dead_code)]
struct Fixture {
    pub rusk: Rusk,
    pub wallet: Wallet<TestStore, TestStateClient>,
    pub tmpdir: TempDir,
}

impl Fixture {
    async fn build(/*owner: impl AsRef<[u8]>*/) -> Self {
        let tmpdir: TempDir = tempfile::tempdir().expect("tempdir() to work");
        let state_dir = tmpdir.path().join("state");
        let data = include_bytes!("../assets/2710377_state.tar.gz");

        rusk_recovery_tools::state::tar::unarchive(
            &data[..],
            state_dir.as_path(),
        )
        .expect("unarchive should work");

        let rusk = initial_state(&state_dir)
            .await
            .expect("Initializing should succeed");

        let cache = Arc::new(RwLock::new(HashMap::new()));

        let wallet = wallet::Wallet::new(
            TestStore,
            TestStateClient {
                rusk: rusk.clone(),
                cache,
            },
        );

        let original_root = rusk.state_root();

        info!("Original Root: {:?}", hex::encode(original_root));

        Self {
            rusk,
            wallet,
            tmpdir,
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
pub async fn test_isolated() -> Result<(), Error> {
    logger();

    // start rusk instance from state "../assets/2710377_state.tar.gz"
    let f = Fixture::build().await;
    println!("root={}", hex::encode(f.rusk.state_root()));

    // move rusk to block 2,710,376
    let base = hex::decode(
        "53de818894cf665f1131edda3c5579ccb8736fd05c993ecb5cd16677974b088b", // Block 2,710,376
    )
    .unwrap();
    let mut base_a: [u8; 32] = [0u8; 32];
    base_a.copy_from_slice(&base);
    let mut lock = f.rusk.tip.write();
    lock.current = base_a;
    drop(lock);

    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(64);

    let sk_1 = bls::SecretKey::random(&mut rng);
    let pk_1 = bls::PublicKey::from(&sk_1);
    let mut nonce_1 = 0;
    let sk_2 = bls::SecretKey::random(&mut rng);
    let pk_2 = bls::PublicKey::from(&sk_2);
    let mut nonce_2 = 0;

    //
    // inject rusk session with the isolated contracts and test accounts
    //

    f.rusk.tip.write().current = base_a;
    let mut session = f
        .rusk
        .new_block_session(1, f.rusk.tip.read().current)
        .expect("creating a session should be possible");

    // deploy alice
    let alice_bytecode = include_bytes!(
        "../../../target/wasm32-unknown-unknown/release/alice.wasm"
    );
    session
        .deploy(
            alice_bytecode,
            ContractData::builder()
                .owner(NON_BLS_OWNER.as_ref())
                .init_arg(&())
                .contract_id(ALICE_ID),
            GAS_LIMIT,
        )
        .expect("Deploying the alice contract should succeed");

    // deploy charlie
    let charlie_bytecode = include_bytes!(
        "../../../target/wasm32-unknown-unknown/release/charlie.wasm"
    );
    session
        .deploy(
            charlie_bytecode,
            ContractData::builder()
                .owner(NON_BLS_OWNER.as_ref())
                .init_arg(&())
                .contract_id(CHARLIE_ID),
            GAS_LIMIT,
        )
        .expect("Deploying the charlie contract should succeed");

    // transfer funds to the two addresses
    session
        .call::<(_, u64), ()>(
            TRANSFER_CONTRACT,
            "add_account_balance",
            &(pk_1, 100_000_000_000_000),
            GAS_LIMIT,
        )
        .expect("Adding balance to first key should succeed");
    session
        .call::<(_, u64), ()>(
            TRANSFER_CONTRACT,
            "add_account_balance",
            &(pk_2, 100_000_000_000_000),
            GAS_LIMIT,
        )
        .expect("Adding balance to second key should succeed");

    // insert stake and reward to second key
    let stake_keys = StakeKeys {
        owner: dusk_core::stake::StakeFundOwner::Account(pk_2),
        account: pk_2,
    };
    let stake_amount = StakeAmount {
        value: DEFAULT_MINIMUM_STAKE,
        locked: 0,
        eligibility: 0,
    };
    let reward = 1000u64;
    let stake_data = StakeData {
        amount: Some(stake_amount),
        reward,
        faults: 0,
        hard_faults: 0,
    };
    session
        .call::<_, ()>(
            STAKE_CONTRACT,
            "insert_stake",
            &(stake_keys, stake_data),
            GAS_LIMIT,
        )
        .expect("Adding a stake to second key should succeed");

    // transfer funds to alice
    session
        .call::<(_, u64), ()>(
            TRANSFER_CONTRACT,
            "add_contract_balance",
            &(ALICE_ID, DEFAULT_MINIMUM_STAKE),
            GAS_LIMIT,
        )
        .expect("Adding balance should succeed");

    // commit the session
    f.rusk.commit_session(session)?;

    //
    // generate the transaction
    //

    // create stake_activate tx
    let stake_amount = DEFAULT_MINIMUM_STAKE - 1;
    let stake = Stake::new_from_contract(
        &f.wallet
            .account_secret_key(0)
            .expect("default secret key should be available"),
        CHARLIE_ID,
        stake_amount,
        CHAIN_ID,
    );
    let stake_activate_call = ContractCall::new(ALICE_ID, "stake_activate")
        .with_args(&stake)
        .expect("Should serialize fn args correctly");
    nonce_1 += 1;
    let stake_activate_tx = moonlight(
        &sk_1,
        None,
        0,
        0,
        GAS_LIMIT,
        1,
        nonce_1,
        CHAIN_ID,
        Some(stake_activate_call),
    )
    .expect("creating the stake activate tx should succeed");

    // create reward withdrawal tx
    nonce_2 += 1;
    let withdraw_tx = moonlight_stake_reward(
        &mut rng,
        &sk_2,
        &sk_2,
        &sk_2,
        reward + 1,
        GAS_LIMIT,
        1,
        nonce_2,
        CHAIN_ID,
    )
    .expect("creating an unstake tx should succeed");

    //
    // Execute the transactions
    //

    let mut voters = vec![];
    for i in 0..10 {
        let sk = bls::SecretKey::random(&mut rng);
        let pk = bls::PublicKey::from(&sk);
        voters.push((node_data::bls::PublicKey::new(pk), i))
    }
    let data = StateTransitionData {
        round: 2710377,
        generator: node_data::bls::PublicKey::new(*DUSK_CONSENSUS_KEY),
        slashes: vec![],
        cert_voters: voters,
        max_txs_bytes: 5000,
        prev_state_root: f.rusk.tip.read().current,
    };

    let txs = vec![
        NodeTransaction::from(stake_activate_tx),
        NodeTransaction::from(withdraw_tx),
    ];

    let (spent, _discarded, _) = f
        .rusk
        .create_state_transition(&data, txs.into_iter())
        .expect("State transition to be executed");
    assert!(spent.len() == 2, "Both txs should be spent");

    Ok(())
}
