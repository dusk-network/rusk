// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::{ParseHexStr, Serializable};
use execution_core::BlsScalar;
use phoenix_core::{Fee, PublicKey, SecretKey, Transaction};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use node::database::DBViewer;
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::pow_verifier::{PoW, POW_DIFFICULTY};
use rusk::{Result, Rusk};
use rusk_abi::{ContractData, ContractId, EconomicMode, TRANSFER_CONTRACT};
use rusk_recovery_tools::state;
use tempfile::tempdir;
use test_wallet::{self as wallet};
use tokio::sync::broadcast;
use tracing::info;

use crate::common::logger;
use crate::common::state::{generator_procedure, ExecuteResult};
use crate::common::wallet::{TestProverClient, TestStateClient, TestStore};

const BLOCK_HEIGHT: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 1_000_000_000_000;
const WALLET_BALANCE: u64 = 10_000_000_000;
const GAS_LIMIT: u64 = 200_000_000;
const GAS_PRICE: u64 = 2;
const CHARLIES_BALANCE: u64 = 140_000_000;
const POINT_LIMIT: u64 = 0x10000000;
const SENDER_INDEX: u64 = 0;

const CHARLIE_CONTRACT_ID: ContractId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0xFC;
    ContractId::from_bytes(bytes)
};
const ALICE_CONTRACT_ID: ContractId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0xFA;
    ContractId::from_bytes(bytes)
};
const ALICE_OWNER: [u8; 32] = [0; 32];

const CHARLIE_FREE_LIMIT: u64 = 20_000_000;
const CHARLIE_FREE_PRICE_HINT: (u64, u64) = (200, 1);

struct DBMock;

const TOP_HEIGHT: u64 = 2;
const USER_HEIGHT: u64 = TOP_HEIGHT - 1;
const BLOCK_HASH: [u8; 32] = [3u8; 32];

impl DBViewer for DBMock {
    fn fetch_block_hash(
        &self,
        _block_height: u64,
    ) -> Result<Option<[u8; 32]>, anyhow::Error> {
        Ok(Some(BLOCK_HASH))
    }
    fn fetch_tip_height(&self) -> anyhow::Result<u64, anyhow::Error> {
        Ok(TOP_HEIGHT)
    }
}

fn initial_state<P: AsRef<Path>>(dir: P, charlies_funds: u64) -> Result<Rusk> {
    let snapshot = toml::from_str(include_str!("../config/contract_pays.toml"))
        .expect("Cannot deserialize config");

    let dir = dir.as_ref();

    let (_vm, _commit_id) = state::deploy(dir, &snapshot, |session| {
        let alice_bytecode = include_bytes!(
            "../../../target/dusk/wasm32-unknown-unknown/release/alice.wasm"
        );
        let charlie_bytecode = include_bytes!(
            "../../../target/dusk/wasm32-unknown-unknown/release/charlie.wasm"
        );

        let mut rng = StdRng::seed_from_u64(0xcafe);
        let charlie_owner_ssk = SecretKey::random(&mut rng);
        let charlie_owner_psk = PublicKey::from(&charlie_owner_ssk);

        session
            .deploy(
                charlie_bytecode,
                ContractData::builder()
                    .owner(charlie_owner_psk.to_bytes())
                    .contract_id(CHARLIE_CONTRACT_ID)
                    .free_limit(CHARLIE_FREE_LIMIT)
                    .free_price_hint(CHARLIE_FREE_PRICE_HINT),
                POINT_LIMIT,
            )
            .expect("Deploying the charlie contract should succeed");

        session
            .call::<_, ()>(
                TRANSFER_CONTRACT,
                "add_module_balance",
                &(CHARLIE_CONTRACT_ID, charlies_funds),
                u64::MAX,
            )
            .expect("stake contract balance to be set with provisioner stakes");

        session
            .deploy(
                alice_bytecode,
                ContractData::builder()
                    .owner(ALICE_OWNER)
                    .contract_id(ALICE_CONTRACT_ID),
                POINT_LIMIT,
            )
            .expect("Deploying the alice contract should succeed");
    })
    .expect("Deploying initial state should succeed");

    let (sender, _) = broadcast::channel(10);

    let rusk = Rusk::new(dir, None, u64::MAX, sender, DBMock {})
        .expect("Instantiating rusk should succeed");
    Ok(rusk)
}

fn make_and_execute_transaction(
    rusk: &Rusk,
    wallet: &wallet::Wallet<TestStore, TestStateClient, TestProverClient>,
    contract_id: &ContractId,
    method: impl AsRef<str>,
    gas_limit: u64,
) -> (EconomicMode, u64) {
    // We will refund the transaction to ourselves.
    let refund = wallet
        .public_key(SENDER_INDEX)
        .expect("Getting a public spend key should succeed");

    let initial_balance = wallet
        .get_balance(SENDER_INDEX)
        .expect("Getting initial balance should succeed")
        .value;

    assert_eq!(
        initial_balance, WALLET_BALANCE,
        "The sender should have the given initial balance"
    );

    let mut rng = StdRng::seed_from_u64(0xcafe);

    let tx = wallet
        .execute(
            &mut rng,
            contract_id.to_bytes().into(),
            String::from(method.as_ref()),
            (),
            SENDER_INDEX,
            &refund,
            gas_limit,
            GAS_PRICE,
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

    let after_balance = wallet
        .get_balance(SENDER_INDEX)
        .expect("Getting initial balance should succeed")
        .value;

    let mut spent_transactions = spent_transactions.into_iter();
    let tx = spent_transactions
        .next()
        .expect("There should be one spent transactions");

    assert!(tx.err.is_none(), "Transaction should succeed");
    (tx.economic_mode, after_balance)
}

fn make_and_execute_transaction_no_deposit(
    rusk: &Rusk,
    contract_id: &ContractId,
    method: impl AsRef<str>,
    should_discard: bool,
) -> EconomicMode {
    // we just need any psk, it won't be used as there won't be any refund
    const DUMMY_PSK: &str = "8ebcaed21b0dd87eb7ca0b1cc1cd3e2e3df85a737037f475f9f7c65176f9ad3f8ebcaed21b0dd87eb7ca0b1cc1cd3e2e3df85a737037f475f9f7c65176f9ad3f";
    let dummy_refund_psk: PublicKey = PublicKey::from_hex_str(DUMMY_PSK)
        .expect("public key creation should succeed");

    let mut rng = StdRng::seed_from_u64(0xcafe);

    // note: gas price zero means that this is a free call
    // we use dummy psk and gas price zero until we change the format of the Fee
    // struct to accommodate for the free calls
    let fee = Fee::new(&mut rng, GAS_LIMIT, 0, &dummy_refund_psk);

    let call = Some((
        contract_id.to_bytes(),
        String::from(method.as_ref()),
        vec![],
    ));

    let mut tx = Transaction {
        anchor: BlsScalar::zero(),
        nullifiers: vec![],
        outputs: vec![],
        fee,
        crossover: None,
        proof: vec![],
        call,
    };

    let expected = ExecuteResult {
        discarded: if should_discard { 1 } else { 0 },
        executed: if should_discard { 0 } else { 1 },
    };

    let mut pow_input = tx.to_hash_input_bytes();
    pow_input.extend(BLOCK_HASH);
    let mut proof = USER_HEIGHT.to_le_bytes().to_vec();
    proof.extend(PoW::generate(pow_input, POW_DIFFICULTY));
    tx.proof = proof;

    let spent_transactions = generator_procedure(
        rusk,
        &[tx],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![],
        Some(expected),
    )
    .expect("generator procedure should succeed");

    if !should_discard {
        let mut spent_transactions = spent_transactions.into_iter();
        let tx = spent_transactions
            .next()
            .expect("There should be one spent transactions");

        assert!(tx.err.is_none(), "Transaction should succeed");
        tx.economic_mode
    } else {
        EconomicMode::None
    }
}

/// We call method 'pay' of a Charlie contract and make sure the gas
/// spent by us is zero as it is the Charlie contract who pays for gas.
/// Note that deposit is needed to execute the transaction.
#[tokio::test(flavor = "multi_thread")]
pub async fn contract_pays_with_deposit() -> Result<()> {
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp, CHARLIES_BALANCE)?;

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

    let (economic_mode, after_balance) = make_and_execute_transaction(
        &rusk,
        &wallet,
        &CHARLIE_CONTRACT_ID,
        "pay",
        GAS_LIMIT,
    );
    assert!(
        match economic_mode {
            EconomicMode::Allowance(allowance) if allowance > 0 => true,
            _ => false,
        },
        "Transaction should be free"
    );
    assert_eq!(after_balance, WALLET_BALANCE); // transaction was free

    // Check the state's root is changed from the original one
    let new_root = rusk.state_root();
    info!(
        "New root after the 1st transfer: {:?}",
        hex::encode(new_root)
    );
    assert_ne!(original_root, new_root, "Root should have changed");

    Ok(())
}

/// We call method 'pay' of a Charlie contract and make sure the gas
/// spent by us is zero as it is the Charlie contract who pays for gas.
/// No deposit and no wallet is needed to execute the transaction.
#[tokio::test(flavor = "multi_thread")]
pub async fn contract_pays() -> Result<()> {
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp, CHARLIES_BALANCE)?;

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    let economic_mode = make_and_execute_transaction_no_deposit(
        &rusk,
        &CHARLIE_CONTRACT_ID,
        "pay",
        false,
    );
    assert!(
        match economic_mode {
            EconomicMode::Allowance(allowance) if allowance > 0 => true,
            _ => false,
        },
        "Transaction should be free"
    );

    // Check the state's root is changed from the original one
    let new_root = rusk.state_root();
    info!(
        "New root after the 1st transfer: {:?}",
        hex::encode(new_root)
    );
    assert_ne!(original_root, new_root, "Root should have changed");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[should_panic(expected = "Allowance not sufficient!")]
pub async fn contract_pays_insufficient_allowance() {
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp, CHARLIES_BALANCE)
        .expect("Initializing should succeed");

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    let economic_mode = make_and_execute_transaction_no_deposit(
        &rusk,
        &CHARLIE_CONTRACT_ID,
        "pay_and_fail",
        false,
    );
    println!("eco mode={:?}", economic_mode);
    assert!(
        match economic_mode {
            EconomicMode::Allowance(allowance) if allowance > 0 => true,
            _ => false,
        },
        "Transaction should be free"
    );

    // Check the state's root is changed from the original one
    let new_root = rusk.state_root();
    info!(
        "New root after the 1st transfer: {:?}",
        hex::encode(new_root)
    );
    assert_ne!(original_root, new_root, "Root should have changed");
}

#[tokio::test(flavor = "multi_thread")]
#[should_panic(expected = "Balance for allowance not sufficient!")]
pub async fn contract_pays_insufficient_balance() {
    logger();

    const INSUFFICIENT_BALANCE: u64 = 1000;
    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp, INSUFFICIENT_BALANCE)
        .expect("Initializing should succeed");

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    let economic_mode = make_and_execute_transaction_no_deposit(
        &rusk,
        &CHARLIE_CONTRACT_ID,
        "pay",
        false,
    );
    println!("eco mode={:?}", economic_mode);
    assert!(
        match economic_mode {
            EconomicMode::Allowance(allowance) if allowance > 0 => true,
            _ => false,
        },
        "Transaction should be free"
    );

    // Check the state's root is changed from the original one
    let new_root = rusk.state_root();
    info!(
        "New root after the 1st transfer: {:?}",
        hex::encode(new_root)
    );
    assert_ne!(original_root, new_root, "Root should have changed");
}

/// We call a "normal" contract via a free transaction.
#[tokio::test(flavor = "multi_thread")]
pub async fn free_tx_calls_not_paying_contract() {
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp, CHARLIES_BALANCE)
        .expect("Initializing should succeed");

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    let _economic_mode = make_and_execute_transaction_no_deposit(
        &rusk,
        &ALICE_CONTRACT_ID,
        "ping",
        true, // should discard the transaction
    );
}
