// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::{HashMap, HashSet};
use std::env;
use std::net::TcpStream;
use std::time::Duration;

use rusk_wallet::GraphQL;
use serde::Deserialize;
use tempfile::{tempdir, TempDir};
use tokio::sync::{Mutex, OnceCell};
use tracing_subscriber::EnvFilter;
use url::Url;

use super::*;
use crate::command::history::TransactionDirection;
use crate::settings::{LogLevel, Logging};
use crate::{connect, status, LogFormat};

#[derive(Default)]
struct FakePrompter {
    text_answer: String,
}

impl Prompt for FakePrompter {
    fn create_new_password(
        &self,
    ) -> anyhow::Result<String, inquire::InquireError> {
        Ok("password".to_string())
    }

    fn prompt_text(&self, _msg: &str) -> inquire::error::InquireResult<String> {
        return Ok(self.text_answer.clone());
    }
}

#[derive(Debug, PartialEq)]
pub struct StrippedTxHistoryItem {
    pub direction: TransactionDirection,
    pub amount: f64,
    pub fee: u64,
}

impl Into<StrippedTxHistoryItem> for TransactionHistory {
    fn into(self) -> StrippedTxHistoryItem {
        StrippedTxHistoryItem {
            direction: self.direction,
            amount: self.amount,
            fee: self.fee,
        }
    }
}

pub fn configure_logger() {
    let directive =
        std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    let filter = EnvFilter::new(directive);
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

// Prior to running the tests, a node compiled with
// prover and archiver features must be running and its
// port should be in the env variable `NODE_PORT`.
fn node_address() -> String {
    let port = env::var("NODE_PORT").expect(
        "env variable NODE_PORT must be set before running these tests",
    );
    format!("127.0.0.1:{port}")
}

fn wallet_settings(wallet_dir: &TempDir) -> Settings {
    let addr = format!("http://{}", node_address());
    Settings {
        state: Url::parse(&addr).unwrap(),
        prover: Url::parse(&addr).unwrap(),
        archiver: Url::parse(&addr).unwrap(),
        explorer: None,
        logging: Logging {
            level: LogLevel::Trace,
            format: LogFormat::Coloured,
        },
        wallet_dir: wallet_dir.path().to_path_buf(),
        password: None,
    }
}

pub async fn wait_for_nodes_to_start() -> anyhow::Result<()> {
    tracing::info!("Waiting for nodes to start");
    let timeout = Duration::from_secs(3);
    let count = 5;
    let node_addr = node_address();
    for _ in 0..count {
        let node_status =
            TcpStream::connect_timeout(&node_addr.parse().unwrap(), timeout);
        if node_status.is_ok() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    Err(anyhow::anyhow!("Nodes never started"))
}

async fn faucet_wallet(
) -> anyhow::Result<&'static Mutex<(Wallet<WalletFile>, Settings)>> {
    // Faucet wallet has to be in a mutex because most tests have to
    // transfer from it to another wallet and they have to be done
    // one by one to avoid reusing nonces.
    static FAUCET_WALLET: OnceCell<Mutex<(Wallet<WalletFile>, Settings)>> =
        OnceCell::const_new();
    FAUCET_WALLET.get_or_try_init(|| async {
        let wallet_dir = tempdir().unwrap();
        let wallet_path = WalletPath::from(wallet_dir.path().join("wallet.dat"));
        let prompter = FakePrompter { text_answer: "auction tribe type torch domain caution lyrics mouse alert fabric snake ticket".to_string() };
        let wallet =
            Command::run_restore_from_seed(&wallet_path, &prompter).unwrap();
        let settings = wallet_settings(&wallet_dir);
        let wallet = connect(wallet, &settings, status::headless).await.unwrap();
        Ok(Mutex::new((wallet, settings)))
    }).await
}

pub async fn create_wallet() -> anyhow::Result<(Wallet<WalletFile>, Settings)> {
    let wallet_dir = tempdir().unwrap();
    let wallet_path = WalletPath::from(wallet_dir.path().join("wallet.dat"));
    let wallet = Command::run_create(
        true,
        &None,
        &None,
        &wallet_path,
        &FakePrompter::default(),
    )
    .unwrap();
    let settings = wallet_settings(&wallet_dir);
    Ok((
        connect(wallet, &settings, status::headless).await.unwrap(),
        settings,
    ))
}

pub async fn rcv_moonlight_from_faucet(
    rcvr_addr: Address,
    amount: u64,
    gas_price: u64,
) -> anyhow::Result<String> {
    let (ref mut faucet_wallet, ref settings) =
        *faucet_wallet().await.unwrap().lock().await;
    let id = transfer_moonlight(
        faucet_wallet,
        rcvr_addr,
        &settings,
        amount,
        gas_price,
    )
    .await?;
    let gql = GraphQL::new(
        settings.state.clone(),
        settings.archiver.clone(),
        status::headless,
    )
    .unwrap();
    gql.wait_for(&id).await.unwrap();
    Ok(id)
}

pub async fn transfer_moonlight(
    wallet: &mut Wallet<WalletFile>,
    to: Address,
    settings: &Settings,
    amount: u64,
    gas_price: u64,
) -> anyhow::Result<String> {
    let cmd = Command::Transfer {
        sender: Some(wallet.default_address()),
        rcvr: to,
        amt: Dusk::new(amount),
        gas_limit: 3_000_000_000,
        gas_price,
        memo: None,
    };
    let run_result = cmd.run(wallet, settings).await.unwrap();
    let RunResult::Tx(tx_hash) = run_result else {
        unreachable!()
    };
    let tx_id = hex::encode(&tx_hash.to_bytes());
    Ok(tx_id)
}

pub async fn transfer_phoenix(
    wallet: &mut Wallet<WalletFile>,
    to: Address,
    settings: &Settings,
    amount: u64,
    gas_price: u64,
) -> anyhow::Result<String> {
    let cmd = Command::Transfer {
        sender: Some(wallet.default_shielded_account()),
        rcvr: to,
        amt: Dusk::new(amount),
        gas_limit: 3_000_000_000,
        gas_price,
        memo: None,
    };
    let run_result = cmd.run(wallet, settings).await.unwrap();
    let RunResult::Tx(tx_hash) = run_result else {
        unreachable!()
    };
    let tx_id = hex::encode(&tx_hash.to_bytes());
    Ok(tx_id)
}

pub async fn rcv_phoenix_from_faucet(
    rcvr_addr: Address,
    amount: u64,
    gas_price: u64,
) -> anyhow::Result<String> {
    let (ref mut faucet_wallet, ref settings) =
        *faucet_wallet().await.unwrap().lock().await;
    let id = transfer_phoenix(
        faucet_wallet,
        rcvr_addr,
        &settings,
        amount,
        gas_price,
    )
    .await?;
    let gql = GraphQL::new(
        settings.state.clone(),
        settings.archiver.clone(),
        status::headless,
    )
    .unwrap();
    gql.wait_for(&id).await.unwrap();
    Ok(id)
}

pub async fn convert_phoenix_to_moonlight(
    wallet: &mut Wallet<WalletFile>,
    settings: &Settings,
    amount: Dusk,
    gas_price: u64,
) -> anyhow::Result<String> {
    let cmd = Command::Unshield {
        profile_idx: None,
        amt: amount,
        gas_limit: 3_000_000_000,
        gas_price,
    };
    let run_result = cmd.run(wallet, &settings).await.unwrap();
    let RunResult::Tx(tx_hash) = run_result else {
        unreachable!()
    };
    let tx_id = hex::encode(&tx_hash.to_bytes());
    Ok(tx_id)
}

pub async fn convert_moonlight_to_phoenix(
    wallet: &mut Wallet<WalletFile>,
    settings: &Settings,
    amount: Dusk,
    gas_price: u64,
) -> anyhow::Result<String> {
    let cmd = Command::Shield {
        profile_idx: None,
        amt: amount,
        gas_limit: 3_000_000_000,
        gas_price,
    };
    let run_result = cmd.run(wallet, &settings).await.unwrap();
    let RunResult::Tx(tx_hash) = run_result else {
        unreachable!()
    };
    let tx_id = hex::encode(&tx_hash.to_bytes());
    Ok(tx_id)
}

pub async fn stake_moonlight(
    wallet: &mut Wallet<WalletFile>,
    settings: &Settings,
    amount: Dusk,
    gas_price: u64,
) -> anyhow::Result<String> {
    let cmd = Command::Stake {
        address: Some(wallet.default_address()),
        owner: Some(wallet.default_address()),
        amt: amount,
        gas_limit: 3_000_000_000,
        gas_price,
    };
    let run_result = cmd.run(wallet, &settings).await.unwrap();
    let RunResult::Tx(tx_hash) = run_result else {
        unreachable!()
    };
    let tx_id = hex::encode(&tx_hash.to_bytes());
    Ok(tx_id)
}

pub async fn stake_phoenix(
    wallet: &mut Wallet<WalletFile>,
    settings: &Settings,
    amount: Dusk,
    gas_price: u64,
) -> anyhow::Result<String> {
    let cmd = Command::Stake {
        address: Some(wallet.default_shielded_account()),
        owner: Some(wallet.default_shielded_account()),
        amt: amount,
        gas_limit: 3_000_000_000,
        gas_price,
    };
    let run_result = cmd.run(wallet, &settings).await.unwrap();
    let RunResult::Tx(tx_hash) = run_result else {
        unreachable!()
    };
    let tx_id = hex::encode(&tx_hash.to_bytes());
    Ok(tx_id)
}

pub async fn unstake_moonlight(
    wallet: &mut Wallet<WalletFile>,
    settings: &Settings,
    gas_price: u64,
) -> anyhow::Result<String> {
    let cmd = Command::Unstake {
        address: Some(wallet.default_address()),
        gas_limit: 3_000_000_000,
        gas_price,
    };
    let run_result = cmd.run(wallet, &settings).await.unwrap();
    let RunResult::Tx(tx_hash) = run_result else {
        unreachable!()
    };
    let tx_id = hex::encode(&tx_hash.to_bytes());
    Ok(tx_id)
}

pub async fn unstake_phoenix(
    wallet: &mut Wallet<WalletFile>,
    settings: &Settings,
    gas_price: u64,
) -> anyhow::Result<String> {
    let cmd = Command::Unstake {
        address: Some(wallet.default_shielded_account()),
        gas_limit: 3_000_000_000,
        gas_price,
    };
    let run_result = cmd.run(wallet, &settings).await.unwrap();
    let RunResult::Tx(tx_hash) = run_result else {
        unreachable!()
    };
    let tx_id = hex::encode(&tx_hash.to_bytes());
    Ok(tx_id)
}

async fn block_is_finalized(
    gql: &GraphQL,
    block_height: u64,
    block_hash: &str,
) -> anyhow::Result<bool> {
    #[derive(Deserialize)]
    struct CheckBlockResponse {
        #[serde(alias = "checkBlock", default)]
        pub is_finalized: bool,
    }
    let query = format!("query {{ checkBlock(height: {block_height}, hash: \"{block_hash}\", onlyFinalized: true) }}");
    let resp = gql.query_archiver(&query).await?;
    let CheckBlockResponse { is_finalized } = serde_json::from_slice(&resp)?;
    Ok(is_finalized)
}

#[derive(Deserialize)]
pub struct TxInfo {
    #[serde(alias = "blockHeight")]
    pub block_height: u64,
    #[serde(alias = "blockHash")]
    pub block_hash: String,
    #[serde(alias = "gasSpent")]
    pub gas_spent: u64,
}

async fn get_tx_info(tx_id: &str, gql: &GraphQL) -> anyhow::Result<TxInfo> {
    #[derive(Deserialize)]
    struct SpentTxResponse {
        tx: TxInfo,
    }
    let query = format!("query {{ tx(hash: \"{tx_id}\") {{ blockHash, blockHeight, gasSpent }} }}");
    let resp = gql.query_archiver(&query).await?;
    let SpentTxResponse { tx } =
        serde_json::from_slice::<SpentTxResponse>(&resp)?;
    Ok(tx)
}

pub async fn wait_for_tx_blocks_to_finalize(
    gql: &GraphQL,
    tx_ids: Vec<&str>,
) -> anyhow::Result<HashMap<String, TxInfo>> {
    let mut txs_info = HashMap::with_capacity(tx_ids.len());
    // Wait for transactions to be included in blocks
    for id in tx_ids {
        gql.wait_for(&id).await?;
        let tx_info = get_tx_info(&id, gql).await?;
        txs_info.insert(id.to_string(), tx_info);
    }

    let mut finalized = HashSet::new();
    // Wait for blocks to finalize
    let max_count = 20;
    for i in 0..max_count {
        tracing::info!("Wait for blocks to finalize for the {i}th time");
        for (tx_id, tx_info) in txs_info.iter() {
            if block_is_finalized(
                gql,
                tx_info.block_height,
                &tx_info.block_hash,
            )
            .await?
            {
                finalized.insert(tx_id);
            }
        }
        if txs_info.len() == finalized.len() {
            return Ok(txs_info);
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Err(anyhow::anyhow!(
        "Some transaction blocks were never finalized"
    ))
}
