// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg(all(feature = "chain", feature = "recovery-state"))]

use dusk_bytes::Serializable;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::transfer::Transaction as ProtocolTransaction;
use dusk_core::transfer::data::TransactionData;
use dusk_core::transfer::moonlight::Transaction as MoonlightTransaction;
use dusk_rusk_test::RuskVmConfig;
use dusk_rusk_test::common::state::{
    DEFAULT_MIN_GAS_LIMIT, LOCAL_TEST_CHAIN_ID,
};
use dusk_vm::FeatureActivation;
#[cfg(feature = "archive")]
use node::archive::Archive;
use node::database::{DB, DatabaseOptions, Ledger, Mempool};
use node::mempool::conf::Params as MempoolParams;
use rusk::http::{
    HttpHandlers, HttpPolicyConfig, HttpServer, HttpServerConfig,
};
use rusk::node::driverstore::DriverStore;
use rusk::node::{FEATURE_HARDFORK_BOREAS, RuskNode, WellKnownVmConfig};
use rusk::{DUSK_CONSENSUS_KEY, Rusk};
use rusk_recovery_tools::state::{self, Snapshot};
use tempfile::tempdir;
use tokio::sync::broadcast;
use tokio::time::{Duration, timeout};
use wallet_core::keys::derive_bls_sk;

const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;
const MOONLIGHT_BALANCE: u64 = 10_000_000_000_000;
const GAS_LIMIT: u64 = 75_000;
const GAS_PRICE: u64 = 1;

#[tokio::test(flavor = "multi_thread")]
async fn future_nonce_http_propagate_accepts_aegis_bytes_after_boreas() {
    let seed = [0u8; 64];
    let sender_sk = derive_bls_sk(&seed, 0);
    let sender_pk = BlsPublicKey::from(&sender_sk);

    let sender_addr = bs58::encode(sender_pk.to_bytes()).into_string();
    let snapshot_toml = format!(
        "[[moonlight_account]]\naddress = \"{sender_addr}\"\nbalance = {MOONLIGHT_BALANCE}\n"
    );
    let snapshot: Snapshot =
        toml::from_str(&snapshot_toml).expect("snapshot should parse");
    let chain_id = LOCAL_TEST_CHAIN_ID;
    let mut vm_config =
        RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);
    let known_conf = WellKnownVmConfig::from_chain_id(chain_id);
    for (feature, activation) in known_conf.features {
        if vm_config.feature(feature).is_none() {
            vm_config.with_feature(feature, activation);
        }
    }
    vm_config
        .with_feature(FEATURE_HARDFORK_BOREAS, FeatureActivation::Height(1));

    let state_dir = tempdir().expect("creating state tempdir should succeed");
    let (_session, state_root) =
        state::deploy(state_dir.path(), &snapshot, *DUSK_CONSENSUS_KEY, |_| {})
            .expect("deploying test state should succeed");

    let (event_sender, _event_receiver) = broadcast::channel(16);
    #[cfg(feature = "archive")]
    let archive_dir =
        tempdir().expect("creating archive tempdir should succeed");
    let rusk = Rusk::new(
        state_dir.path(),
        chain_id,
        vm_config,
        DEFAULT_MIN_GAS_LIMIT,
        u64::MAX,
        event_sender.clone(),
        #[cfg(feature = "archive")]
        Archive::create_or_open(archive_dir.path()).await,
        DriverStore::new(None::<std::path::PathBuf>),
    )
    .expect("instantiating rusk should succeed");
    let sender_account = rusk
        .account(&sender_pk)
        .expect("account should exist in genesis snapshot");

    let db_dir = tempdir().expect("creating DB tempdir should succeed");
    let backend = node::database::rocksdb::Backend::create_or_open(
        db_dir.path(),
        DatabaseOptions::default(),
    );
    backend
        .update(|db| {
            let header = node_data::ledger::Header {
                height: 0,
                state_hash: state_root,
                hash: [1u8; 32],
                ..Default::default()
            };
            db.store_block(
                &header,
                &[],
                &[],
                node_data::ledger::Label::Final(0),
            )?;

            Ok(())
        })
        .expect("storing genesis block should succeed");

    let kadcast_conf = kadcast::config::Config {
        public_address: "127.0.0.1:0".to_string(),
        listen_address: Some("127.0.0.1:0".to_string()),
        ..Default::default()
    };
    let network =
        node::network::Kadcast::<255>::new(kadcast_conf).expect("valid config");
    #[cfg(feature = "archive")]
    let _archive_dir =
        tempdir().expect("creating archive tempdir should succeed");
    #[cfg(feature = "archive")]
    let archive = Archive::create_or_open(_archive_dir.path()).await;
    let mempool_conf = MempoolParams::default();
    let node = RuskNode::new(
        node::Node::new(network, backend, rusk.clone()),
        node::mempool::FutureNonceRetryHandle::new(
            mempool_conf.max_queue_size,
            mempool_conf.max_moonlight_future_nonce_per_account,
        ),
        #[cfg(feature = "archive")]
        archive,
    );

    let mut event_receiver = event_sender.subscribe();
    let mut handlers = HttpHandlers::default();
    handlers.set_chain_handler(node.clone());
    handlers.set_graphql_handler(node.clone());
    let (_server, local_addr) = HttpServer::bind(
        handlers,
        event_sender.clone(),
        HttpServerConfig {
            address: "127.0.0.1:0".to_string(),
            cert: None,
            key: None,
            enable_docs: false,
            headers: Default::default(),
            ws_event_channel_cap: 16,
            policy: HttpPolicyConfig::default(),
        },
    )
    .await
    .expect("binding test HTTP server should succeed");
    drop(event_sender);

    let future_tx = MoonlightTransaction::new(
        &sender_sk,
        None,
        1,
        0,
        GAS_LIMIT,
        GAS_PRICE,
        sender_account.nonce + 2,
        chain_id,
        None::<TransactionData>,
    )
    .expect("creating tx should succeed");
    let future_tx = ProtocolTransaction::Moonlight(future_tx);
    let expected_entity = hex::encode(future_tx.hash().to_bytes());

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://{local_addr}/on/transactions/propagate"))
        .header("Content-Type", "application/octet-stream")
        .body(future_tx.to_var_bytes())
        .send()
        .await
        .expect("requesting should succeed");
    assert_eq!(response.status(), reqwest::StatusCode::ACCEPTED);

    assert_eq!(
        node.db().read().await.view(|db| db.mempool_txs_count()),
        0,
        "future nonce tx should not be inserted into the real mempool by HTTP ingress alone"
    );

    let deferred_event = timeout(Duration::from_secs(2), event_receiver.recv())
        .await
        .expect("future nonce HTTP propagate should emit a deferred event")
        .expect("RUES receiver should stay alive");
    assert_eq!(deferred_event.uri.component, "transactions");
    assert_eq!(deferred_event.uri.topic, "deferred");
    assert_eq!(
        deferred_event.uri.entity.as_deref(),
        Some(expected_entity.as_str())
    );
    let payload = serde_json::to_value(&deferred_event.data)
        .expect("RUES event payload should be serializable");
    assert_eq!(payload["reason"], "missing_intermediate_nonce");
}
