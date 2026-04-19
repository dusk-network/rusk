// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use dusk_bytes::Serializable;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::transfer::Transaction;
use dusk_core::transfer::phoenix::PublicKey as PhoenixPublicKey;
use dusk_rusk_test::common::state::{ExecuteResult, generator_procedure2};
use dusk_rusk_test::{Rusk, RuskVmConfig, SpentTransaction, TestContext};
use rusk::node::{
    FEATURE_ABI_PUBLIC_SENDER, FEATURE_BLOB, FEATURE_HARDFORK_AEGIS,
    FEATURE_HARDFORK_BOREAS, FEATURE_PLONK_V2,
};
use serde::{Deserialize, Serialize};
use wallet_core::keys::{derive_bls_pk, derive_phoenix_pk};

pub(super) const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;
pub(super) const FIXTURE_SEED: [u8; 64] = [0; 64];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ReplayFixture {
    pub suite: String,
    pub initial_state_root: String,
    pub config: FixtureConfig,
    pub steps: Vec<ReplayStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct FixtureConfig {
    pub suite: String,
    pub features: FeatureConfig,
    pub moonlight_accounts: Vec<MoonlightAccountConfig>,
    pub phoenix_balances: Vec<PhoenixBalanceConfig>,
    pub stakes: Vec<StakeConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct MoonlightAccountConfig {
    pub index: u8,
    pub balance: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct PhoenixBalanceConfig {
    pub index: u8,
    pub seed: u64,
    pub notes: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StakeConfig {
    pub index: u8,
    pub amount: u64,
    pub reward: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct FeatureConfig {
    pub abi_public_sender: u64,
    pub plonk_v2: u64,
    pub blob: u64,
    pub hardfork_aegis: u64,
    pub hardfork_boreas: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum ReplayStepKind {
    Tx,
    EmptyBlocks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ReplayStep {
    pub name: String,
    pub kind: ReplayStepKind,
    pub height: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub state_root: String,
}

pub(super) fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/assets/compat_replay_v1.json")
}

fn fixture_config_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/config/compat_replay.toml")
}

pub(super) fn bob_wasm_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../contracts/bin/bob.wasm")
}

pub(super) fn load_fixture_config() -> Result<FixtureConfig> {
    let path = fixture_config_path();
    let data = fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&data)
        .with_context(|| format!("failed to parse {}", path.display()))
}

pub(super) fn load_fixture(path: &Path) -> Result<ReplayFixture> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&data)
        .with_context(|| format!("failed to parse {}", path.display()))
}

pub(super) fn write_fixture(
    path: &Path,
    fixture: &ReplayFixture,
) -> Result<()> {
    let parent = path.parent().context("fixture path has no parent")?;
    fs::create_dir_all(parent)?;
    fs::write(path, serde_json::to_string_pretty(fixture)?)
        .with_context(|| format!("failed to write {}", path.display()))
}

pub(super) fn moonlight_public_key(index: u8) -> BlsPublicKey {
    derive_bls_pk(&FIXTURE_SEED, index)
}

pub(super) fn phoenix_public_key(index: u8) -> PhoenixPublicKey {
    derive_phoenix_pk(&FIXTURE_SEED, index)
}

pub(super) async fn instantiate_config_context(
    config: &FixtureConfig,
) -> Result<TestContext> {
    let vm_config = compat_vm_config(&config.features);
    TestContext::instantiate(&state_toml(config), vm_config).await
}

pub(super) async fn instantiate_fixture_context(
    fixture: &ReplayFixture,
) -> Result<TestContext> {
    instantiate_config_context(&fixture.config).await
}

pub(super) fn tx_step(
    name: impl Into<String>,
    height: u64,
    raw: String,
) -> ReplayStep {
    ReplayStep {
        name: name.into(),
        kind: ReplayStepKind::Tx,
        height,
        count: None,
        raw: Some(raw),
        error: None,
        state_root: String::new(),
    }
}

pub(super) fn empty_blocks_step(
    name: impl Into<String>,
    start_height: u64,
    count: u64,
) -> ReplayStep {
    ReplayStep {
        name: name.into(),
        kind: ReplayStepKind::EmptyBlocks,
        height: start_height,
        count: Some(count),
        raw: None,
        error: None,
        state_root: String::new(),
    }
}

pub(super) async fn sync_fixture_expectations(
    fixture: &mut ReplayFixture,
) -> Result<()> {
    let tc = instantiate_fixture_context(fixture).await?;

    fixture.initial_state_root = hex::encode(tc.state_root());

    for step in &mut fixture.steps {
        let (error, state_root) = execute_recorded_step(&tc, step)?;
        step.error = error;
        step.state_root = state_root;
    }

    Ok(())
}

pub(super) async fn assert_fixture_replays(
    fixture: &ReplayFixture,
) -> Result<()> {
    let tc = instantiate_fixture_context(fixture).await?;

    assert_eq!(
        hex::encode(tc.state_root()),
        fixture.initial_state_root,
        "initial state root drifted before replay started"
    );

    for step in &fixture.steps {
        let (error, state_root) = execute_recorded_step(&tc, step)?;

        assert_eq!(
            error, step.error,
            "{} emitted an unexpected execution error",
            step.name
        );
        assert_eq!(
            state_root, step.state_root,
            "{} produced a different state root",
            step.name
        );
        assert_eq!(
            hex::encode(tc.state_root()),
            step.state_root,
            "{} committed a different state root than reported",
            step.name
        );
    }

    Ok(())
}

pub(super) fn apply_tx_step(
    tc: &TestContext,
    height: u64,
    raw: Vec<u8>,
) -> Result<(Vec<SpentTransaction>, [u8; 32])> {
    execute_raw_replay_tx(tc.rusk().clone(), raw, height)
}

fn compat_vm_config(features: &FeatureConfig) -> RuskVmConfig {
    let mut vm_config =
        RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);
    vm_config
        .with_feature(FEATURE_ABI_PUBLIC_SENDER, features.abi_public_sender);
    vm_config.with_feature(FEATURE_PLONK_V2, features.plonk_v2);
    vm_config.with_feature(FEATURE_BLOB, features.blob);
    vm_config.with_feature(FEATURE_HARDFORK_AEGIS, features.hardfork_aegis);
    vm_config.with_feature(FEATURE_HARDFORK_BOREAS, features.hardfork_boreas);
    vm_config
}

fn execute_recorded_step(
    tc: &TestContext,
    step: &ReplayStep,
) -> Result<(Option<String>, String)> {
    match step.kind {
        ReplayStepKind::Tx => {
            let raw_hex = step.raw.as_ref().with_context(|| {
                format!("{} is missing raw tx bytes", step.name)
            })?;
            let raw = hex::decode(raw_hex).with_context(|| {
                format!("{} has invalid raw tx hex", step.name)
            })?;
            let (spent_txs, root) = apply_tx_step(tc, step.height, raw)?;
            let spent = spent_txs
                .into_iter()
                .next()
                .context("expected one executed tx during replay")?;
            Ok((spent.err, hex::encode(root)))
        }
        ReplayStepKind::EmptyBlocks => {
            let count = step.count.with_context(|| {
                format!("{} is missing empty-block count", step.name)
            })?;
            let mut last_root = tc.state_root();
            for height in step.height..step.height + count {
                last_root = tc.empty_block(height)?;
            }
            Ok((None, hex::encode(last_root)))
        }
    }
}

fn execute_replay_tx(
    rusk: Rusk,
    tx: Transaction,
    height: u64,
) -> Result<(Vec<SpentTransaction>, [u8; 32])> {
    let has_blob = tx.blob().is_some();
    let execute = move || {
        generator_procedure2(
            &rusk,
            &[tx],
            height,
            BLOCK_GAS_LIMIT,
            vec![],
            Some(ExecuteResult {
                executed: 1,
                discarded: 0,
            }),
            None,
        )
        .map_err(|err| anyhow::anyhow!("{err:?}"))
    };

    if has_blob {
        std::thread::Builder::new()
            .name("compat-replay-blob-exec".to_string())
            .stack_size(8 * 1024 * 1024)
            .spawn(execute)
            .map_err(|err| {
                anyhow::anyhow!("failed to spawn blob exec thread: {err}")
            })?
            .join()
            .map_err(|err| {
                anyhow::anyhow!("blob exec thread panicked: {err:?}")
            })?
    } else {
        execute()
    }
}

fn execute_raw_replay_tx(
    rusk: Rusk,
    raw: Vec<u8>,
    height: u64,
) -> Result<(Vec<SpentTransaction>, [u8; 32])> {
    if raw.len() < 64 * 1024 {
        let tx = Transaction::from_slice(&raw).map_err(|err| {
            anyhow::anyhow!("failed to decode replay tx bytes: {err:?}")
        })?;
        return execute_replay_tx(rusk, tx, height);
    }

    let execute = move || {
        let tx = Transaction::from_slice(&raw).map_err(|err| {
            anyhow::anyhow!("failed to decode replay tx bytes: {err:?}")
        })?;
        generator_procedure2(
            &rusk,
            &[tx],
            height,
            BLOCK_GAS_LIMIT,
            vec![],
            Some(ExecuteResult {
                executed: 1,
                discarded: 0,
            }),
            None,
        )
        .map_err(|err| anyhow::anyhow!("{err:?}"))
    };

    std::thread::Builder::new()
        .name("compat-replay-raw-exec".to_string())
        .stack_size(8 * 1024 * 1024)
        .spawn(execute)
        .map_err(|err| {
            anyhow::anyhow!("failed to spawn raw exec thread: {err}")
        })?
        .join()
        .map_err(|err| anyhow::anyhow!("raw exec thread panicked: {err:?}"))?
}

fn state_toml(config: &FixtureConfig) -> String {
    let mut toml = String::new();

    for phoenix_balance in &config.phoenix_balances {
        toml.push_str("[[phoenix_balance]]\n");
        toml.push_str(&format!(
            "address = \"{}\"\n",
            phoenix_address(phoenix_balance.index)
        ));
        toml.push_str(&format!("seed = {}\n", phoenix_balance.seed));
        toml.push_str("notes = [");
        for (idx, note) in phoenix_balance.notes.iter().enumerate() {
            if idx > 0 {
                toml.push_str(", ");
            }
            toml.push_str(&note.to_string());
        }
        toml.push_str("]\n\n");
    }

    for account in &config.moonlight_accounts {
        toml.push_str("[[moonlight_account]]\n");
        toml.push_str(&format!(
            "address = \"{}\"\n",
            moonlight_address(account.index)
        ));
        toml.push_str(&format!("balance = {}\n\n", account.balance));
    }

    for stake in &config.stakes {
        toml.push_str("[[stake]]\n");
        toml.push_str(&format!(
            "address = \"{}\"\n",
            moonlight_address(stake.index)
        ));
        toml.push_str(&format!("amount = {}\n", stake.amount));
        if let Some(reward) = stake.reward {
            toml.push_str(&format!("reward = {}\n", reward));
        }
        toml.push('\n');
    }

    toml
}

fn moonlight_address(index: u8) -> String {
    let pk = derive_bls_pk(&FIXTURE_SEED, index);
    bs58::encode(pk.to_bytes()).into_string()
}

fn phoenix_address(index: u8) -> String {
    let pk = derive_phoenix_pk(&FIXTURE_SEED, index);
    bs58::encode(pk.to_bytes()).into_string()
}
