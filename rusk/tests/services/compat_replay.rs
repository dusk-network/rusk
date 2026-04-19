// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fs;

use anyhow::{Context, Result, bail};
use dusk_bytes::Serializable;
use dusk_core::abi::ContractId;
use dusk_core::transfer::Transaction;
use dusk_core::transfer::data::{BlobData, ContractCall};
use dusk_core::transfer::moonlight::Transaction as MoonlightTransaction;
use dusk_vm::gen_contract_id;
use rand::SeedableRng;
use rand::rngs::StdRng;
use wallet_core::keys::derive_bls_sk;

use super::compat_replay_fixture::{
    FIXTURE_SEED, ReplayFixture, ReplayStep, apply_tx_step,
    assert_fixture_replays, bob_wasm_path, empty_blocks_step, fixture_path,
    instantiate_config_context, load_fixture, load_fixture_config,
    moonlight_public_key, phoenix_public_key, sync_fixture_expectations,
    tx_step, write_fixture,
};
use crate::common::logger;

const GAS_LIMIT_TRANSFER: u64 = 1_000_000_000;
const GAS_LIMIT_CALL: u64 = 1_000_000_000;
const GAS_LIMIT_DEPLOY: u64 = 200_000_000;
const GAS_PRICE: u64 = 1;
const DEPLOY_GAS_PRICE: u64 = 2_000;
const BLOB_BYTES: usize = 4096 * 32;
const DEPLOY_NONCE: u64 = 7;
const AEGIS_DEPLOY_NONCE: u64 = 8;
const BOREAS_DEPLOY_NONCE: u64 = 9;
const BOB_INIT_VALUE: u8 = 5;
const BOB_RESET_VALUE: u8 = 11;
const AEGIS_BOB_INIT_VALUE: u8 = 13;
const AEGIS_BOB_RESET_VALUE: u8 = 17;
const BOREAS_BOB_INIT_VALUE: u8 = 19;
const BOREAS_BOB_RESET_VALUE: u8 = 23;

// This suite keeps a checked-in replay fixture so we can cheaply prove that
// the same historical transaction bytes still produce the same post-state
// roots across pre-hardfork, Aegis, and Boreas transitions.
//
// The expensive part is fixture generation. Day-to-day replay just loads the
// checked-in raw tx bytes and proves they still commit to the same roots.

fn seeded_rng(seed: u64) -> StdRng {
    StdRng::seed_from_u64(seed)
}

fn deterministic_blob_datapart(seed: u8) -> Vec<u8> {
    let mut bytes = vec![0u8; BLOB_BYTES];
    for (idx, chunk) in bytes.chunks_exact_mut(32).enumerate() {
        chunk[31] = seed.wrapping_add((idx % 251) as u8);
    }
    bytes
}

fn deterministic_blob(seed: u8) -> Result<BlobData> {
    let bytes = deterministic_blob_datapart(seed);
    BlobData::from_datapart(&bytes, None).map_err(|err| {
        anyhow::anyhow!("failed to build deterministic blob: {err}")
    })
}

fn deterministic_blobs(seed: u8) -> Result<Vec<BlobData>> {
    // `BlobData` embeds a full 128 KiB sidecar. Build it on a dedicated
    // large-stack thread and move it into a heap-backed Vec before the async
    // replay machinery touches it.
    std::thread::Builder::new()
        .name("compat-replay-blob-builder".to_string())
        .stack_size(8 * 1024 * 1024)
        .spawn(move || deterministic_blob(seed).map(|blob| vec![blob]))
        .map_err(|err| {
            anyhow::anyhow!("failed to spawn blob builder thread: {err}")
        })?
        .join()
        .map_err(|err| {
            anyhow::anyhow!("blob builder thread panicked: {err:?}")
        })?
}

fn into_anyhow<T, E>(result: std::result::Result<T, E>) -> Result<T>
where
    E: std::fmt::Debug,
{
    result.map_err(|err| anyhow::anyhow!("{err:?}"))
}

fn with_plonk_prove_mode<T>(
    mode: Option<&str>,
    f: impl FnOnce() -> Result<T>,
) -> Result<T> {
    let previous = std::env::var("RUSK_PLONK_PROVE_MODE").ok();

    unsafe {
        match mode {
            Some(mode) => std::env::set_var("RUSK_PLONK_PROVE_MODE", mode),
            None => std::env::remove_var("RUSK_PLONK_PROVE_MODE"),
        }
    }

    let result = f();

    unsafe {
        match previous {
            Some(mode) => std::env::set_var("RUSK_PLONK_PROVE_MODE", mode),
            None => std::env::remove_var("RUSK_PLONK_PROVE_MODE"),
        }
    }

    result
}

fn resign_moonlight_insecure(
    tx: Transaction,
    sender_index: u8,
) -> Result<Transaction> {
    let Transaction::Moonlight(tx) = tx else {
        bail!("expected a moonlight transaction");
    };
    let signer = derive_bls_sk(&FIXTURE_SEED, sender_index);
    let mut bytes = tx.to_var_bytes();
    let sig = signer.sign_insecure(&tx.signature_message()).to_bytes();
    let sig_start = bytes
        .len()
        .checked_sub(sig.len())
        .context("moonlight tx must contain a signature")?;
    bytes[sig_start..].copy_from_slice(&sig);

    Ok(Transaction::Moonlight(
        MoonlightTransaction::from_slice(&bytes).map_err(|err| {
            anyhow::anyhow!("failed to decode re-signed tx: {err:?}")
        })?,
    ))
}

fn reset_call(
    contract_id: ContractId,
    value: u8,
    label: &str,
) -> Result<ContractCall> {
    ContractCall::new(contract_id, "reset")
        .with_args(&value)
        .map_err(|err| {
            anyhow::anyhow!(
                "failed to build {label} reset contract call: {err:?}"
            )
        })
}

fn record_tx(
    tc: &dusk_rusk_test::TestContext,
    steps: &mut Vec<ReplayStep>,
    name: &'static str,
    height: u64,
    tx: Transaction,
) -> Result<()> {
    let raw = tx.to_var_bytes();
    apply_tx_step(tc, height, raw.clone())?;
    steps.push(tx_step(name, height, hex::encode(raw)));
    Ok(())
}

fn record_empty_blocks(
    tc: &dusk_rusk_test::TestContext,
    steps: &mut Vec<ReplayStep>,
    name: &'static str,
    start_height: u64,
    count: u64,
) -> Result<()> {
    for height in start_height..start_height + count {
        tc.empty_block(height)?;
    }
    steps.push(empty_blocks_step(name, start_height, count));
    Ok(())
}

async fn build_fixture() -> Result<ReplayFixture> {
    let config = load_fixture_config()?;
    let tc = instantiate_config_context(&config).await?;
    let wallet = tc.wallet();
    let initial_state_root = hex::encode(tc.state_root());
    let bob_bytecode = fs::read(bob_wasm_path()).with_context(|| {
        format!("failed to read {}", bob_wasm_path().display())
    })?;
    let owner = into_anyhow(wallet.account_public_key(0))?;
    let prefork_contract_id =
        gen_contract_id(&bob_bytecode, DEPLOY_NONCE, owner.to_bytes());
    let aegis_contract_id =
        gen_contract_id(&bob_bytecode, AEGIS_DEPLOY_NONCE, owner.to_bytes());
    let boreas_contract_id =
        gen_contract_id(&bob_bytecode, BOREAS_DEPLOY_NONCE, owner.to_bytes());
    let mut steps = Vec::new();

    record_tx(
        &tc,
        &mut steps,
        "prefork_moonlight_transfer",
        5,
        resign_moonlight_insecure(
            into_anyhow(wallet.moonlight_transfer(
                0,
                moonlight_public_key(1),
                25_000_000_000,
                GAS_LIMIT_TRANSFER,
                GAS_PRICE,
            ))?,
            0,
        )?,
    )?;

    record_tx(
        &tc,
        &mut steps,
        "prefork_contract_deploy",
        6,
        resign_moonlight_insecure(
            into_anyhow(wallet.moonlight_deployment(
                0,
                bob_bytecode.clone(),
                &owner,
                vec![BOB_INIT_VALUE],
                GAS_LIMIT_DEPLOY,
                DEPLOY_GAS_PRICE,
                DEPLOY_NONCE,
            ))?,
            0,
        )?,
    )?;

    record_tx(
        &tc,
        &mut steps,
        "prefork_contract_call",
        7,
        resign_moonlight_insecure(
            into_anyhow(wallet.moonlight_execute(
                0,
                0,
                0,
                GAS_LIMIT_CALL,
                GAS_PRICE,
                Some(reset_call(
                    prefork_contract_id,
                    BOB_RESET_VALUE,
                    "prefork",
                )?),
            ))?,
            0,
        )?,
    )?;

    record_empty_blocks(&tc, &mut steps, "advance_prefork", 8, 17)?;

    record_tx(
        &tc,
        &mut steps,
        "prefork_phoenix_transfer",
        25,
        with_plonk_prove_mode(Some("v2"), || {
            into_anyhow(wallet.phoenix_transfer(
                &mut seeded_rng(0x12),
                0,
                &phoenix_public_key(1),
                20_000_000_000,
                GAS_LIMIT_TRANSFER,
                GAS_PRICE,
            ))
        })?,
    )?;

    record_empty_blocks(&tc, &mut steps, "advance_to_aegis_blob", 26, 24)?;

    record_tx(
        &tc,
        &mut steps,
        "aegis_blob_transaction",
        50,
        into_anyhow(tc.wallet().moonlight_execute(
            0,
            0,
            0,
            GAS_LIMIT_TRANSFER,
            GAS_PRICE,
            Some(deterministic_blobs(0x42)?),
        ))?,
    )?;

    record_tx(
        &tc,
        &mut steps,
        "aegis_moonlight_transfer",
        51,
        into_anyhow(wallet.moonlight_transfer(
            2,
            moonlight_public_key(3),
            9_000_000_000,
            GAS_LIMIT_TRANSFER,
            GAS_PRICE,
        ))?,
    )?;

    record_tx(
        &tc,
        &mut steps,
        "aegis_contract_deploy",
        52,
        into_anyhow(wallet.moonlight_deployment(
            0,
            bob_bytecode.clone(),
            &owner,
            vec![AEGIS_BOB_INIT_VALUE],
            GAS_LIMIT_DEPLOY,
            DEPLOY_GAS_PRICE,
            AEGIS_DEPLOY_NONCE,
        ))?,
    )?;

    record_tx(
        &tc,
        &mut steps,
        "aegis_contract_call",
        53,
        into_anyhow(wallet.moonlight_execute(
            0,
            0,
            0,
            GAS_LIMIT_CALL,
            GAS_PRICE,
            Some(reset_call(
                aegis_contract_id,
                AEGIS_BOB_RESET_VALUE,
                "Aegis",
            )?),
        ))?,
    )?;

    record_tx(
        &tc,
        &mut steps,
        "aegis_phoenix_transfer",
        54,
        into_anyhow(wallet.phoenix_transfer(
            &mut seeded_rng(0x18),
            0,
            &phoenix_public_key(2),
            12_000_000_000,
            GAS_LIMIT_TRANSFER,
            GAS_PRICE,
        ))?,
    )?;

    record_tx(
        &tc,
        &mut steps,
        "aegis_unshield_prefork_note",
        55,
        into_anyhow(wallet.phoenix_to_moonlight(
            &mut seeded_rng(0x13),
            1,
            1,
            10_000_000_000,
            GAS_LIMIT_TRANSFER,
            GAS_PRICE,
        ))?,
    )?;

    record_tx(
        &tc,
        &mut steps,
        "aegis_shield_note",
        56,
        into_anyhow(wallet.moonlight_to_phoenix(
            &mut seeded_rng(0x14),
            0,
            3,
            15_000_000_000,
            GAS_LIMIT_TRANSFER,
            GAS_PRICE,
        ))?,
    )?;

    record_tx(
        &tc,
        &mut steps,
        "aegis_reward_withdraw",
        57,
        into_anyhow(wallet.moonlight_stake_withdraw(
            &mut seeded_rng(0x15),
            1,
            1,
            GAS_LIMIT_TRANSFER,
            GAS_PRICE,
        ))?,
    )?;

    record_empty_blocks(&tc, &mut steps, "advance_to_boreas", 58, 42)?;

    record_tx(
        &tc,
        &mut steps,
        "boreas_blob_transaction",
        100,
        into_anyhow(tc.wallet().moonlight_execute(
            0,
            0,
            0,
            GAS_LIMIT_TRANSFER,
            GAS_PRICE,
            Some(deterministic_blobs(0x43)?),
        ))?,
    )?;

    record_tx(
        &tc,
        &mut steps,
        "boreas_moonlight_transfer",
        101,
        into_anyhow(wallet.moonlight_transfer(
            3,
            moonlight_public_key(1),
            7_000_000_000,
            GAS_LIMIT_TRANSFER,
            GAS_PRICE,
        ))?,
    )?;

    record_tx(
        &tc,
        &mut steps,
        "boreas_contract_deploy",
        102,
        into_anyhow(wallet.moonlight_deployment(
            0,
            bob_bytecode.clone(),
            &owner,
            vec![BOREAS_BOB_INIT_VALUE],
            GAS_LIMIT_DEPLOY,
            DEPLOY_GAS_PRICE,
            BOREAS_DEPLOY_NONCE,
        ))?,
    )?;

    record_tx(
        &tc,
        &mut steps,
        "boreas_contract_call",
        103,
        into_anyhow(wallet.moonlight_execute(
            0,
            0,
            0,
            GAS_LIMIT_CALL,
            GAS_PRICE,
            Some(reset_call(
                boreas_contract_id,
                BOREAS_BOB_RESET_VALUE,
                "Boreas",
            )?),
        ))?,
    )?;

    record_tx(
        &tc,
        &mut steps,
        "boreas_phoenix_transfer",
        104,
        into_anyhow(wallet.phoenix_transfer(
            &mut seeded_rng(0x19),
            2,
            &phoenix_public_key(1),
            6_000_000_000,
            GAS_LIMIT_TRANSFER,
            GAS_PRICE,
        ))?,
    )?;

    record_tx(
        &tc,
        &mut steps,
        "boreas_unshield_aegis_note",
        105,
        into_anyhow(wallet.phoenix_to_moonlight(
            &mut seeded_rng(0x16),
            3,
            3,
            8_000_000_000,
            GAS_LIMIT_TRANSFER,
            GAS_PRICE,
        ))?,
    )?;

    record_tx(
        &tc,
        &mut steps,
        "boreas_reward_withdraw",
        106,
        into_anyhow(wallet.moonlight_stake_withdraw(
            &mut seeded_rng(0x17),
            2,
            2,
            GAS_LIMIT_TRANSFER,
            GAS_PRICE,
        ))?,
    )?;

    Ok(ReplayFixture {
        suite: config.suite.clone(),
        initial_state_root,
        config,
        steps,
    })
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "writes deterministic replay fixtures"]
async fn generate_compat_replay_v1_fixture() -> Result<()> {
    logger();

    let mut fixture = build_fixture().await?;
    sync_fixture_expectations(&mut fixture).await?;
    write_fixture(&fixture_path(), &fixture)?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "replays the existing fixture and rewrites expected roots"]
async fn sync_compat_replay_v1_fixture_expectations() -> Result<()> {
    logger();

    let path = fixture_path();
    let mut fixture = load_fixture(&path)?;
    sync_fixture_expectations(&mut fixture).await?;
    write_fixture(&path, &fixture)?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn compat_replay_fixture_replays_deterministically() -> Result<()> {
    logger();

    let path = fixture_path();
    if !path.exists() {
        bail!(
            "{} is missing, run the ignored fixture generator first",
            path.display()
        );
    }

    let fixture = load_fixture(&path)?;
    assert_fixture_replays(&fixture).await
}
