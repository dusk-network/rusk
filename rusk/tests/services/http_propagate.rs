// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg(all(feature = "chain", feature = "recovery-state"))]

use std::path::PathBuf;

use dusk_bytes::Serializable;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::transfer::Transaction as ProtocolTransaction;
use dusk_core::transfer::data::TransactionData;
use dusk_core::transfer::moonlight::Transaction as MoonlightTransaction;
use dusk_rusk_test::{RuskVmConfig, TestContext};
use hyper::HeaderMap;
use node::database::{DB, DatabaseOptions, Ledger};
use rusk::http::HttpServer;
use rusk::node::RuskNode;
use tempfile::tempdir;
use tokio::sync::broadcast;
use wallet_core::keys::derive_bls_sk;

const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;
const MOONLIGHT_BALANCE: u64 = 10_000_000_000_000;
const GAS_LIMIT: u64 = 75_000;
const GAS_PRICE: u64 = 1;

#[tokio::test(flavor = "multi_thread")]
async fn propagate_rejects_tx_that_fails_preverify() {
    let seed = [0u8; 64];
    let sender_sk = derive_bls_sk(&seed, 0);
    let sender_pk = BlsPublicKey::from(&sender_sk);

    let sender_addr = bs58::encode(sender_pk.to_bytes()).into_string();
    let snapshot_toml = format!(
        "[[moonlight_account]]\naddress = \"{sender_addr}\"\nbalance = {MOONLIGHT_BALANCE}\n"
    );
    let vm_config = RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);
    let test_context = TestContext::instantiate(&snapshot_toml, vm_config)
        .await
        .expect("instantiating test context should succeed");

    let rusk = test_context.rusk().clone();
    let chain_id = rusk.chain_id().expect("chain id should be available");
    let sender_account = rusk
        .account(&sender_pk)
        .expect("account should exist in genesis snapshot");
    let state_root = test_context.state_root();

    let db_dir = tempdir().expect("creating DB tempdir should succeed");
    let backend = node::database::rocksdb::Backend::create_or_open(
        db_dir.path(),
        DatabaseOptions::default(),
    );
    backend
        .update(|db| {
            let mut header = node_data::ledger::Header::default();
            header.height = 0;
            header.state_hash = state_root;
            header.hash = [1u8; 32];
            db.store_block(
                &header,
                &[],
                &[],
                node_data::ledger::Label::Final(0),
            )?;

            Ok(())
        })
        .expect("storing genesis block should succeed");

    let mut kadcast_conf = kadcast::config::Config::default();
    kadcast_conf.public_address = "127.0.0.1:0".to_string();
    kadcast_conf.listen_address = Some("127.0.0.1:0".to_string());
    let network =
        node::network::Kadcast::<255>::new(kadcast_conf).expect("valid config");
    let node = RuskNode::new(node::Node::new(network, backend, rusk));

    let (event_sender, event_receiver) = broadcast::channel(1);
    let (_server, local_addr) = HttpServer::bind(
        node,
        event_receiver,
        16,
        "127.0.0.1:0",
        HeaderMap::new(),
        None::<(PathBuf, PathBuf)>,
    )
    .await
    .expect("binding test HTTP server should succeed");
    drop(event_sender);

    let tx = MoonlightTransaction::new(
        &sender_sk,
        None,
        1,
        0,
        GAS_LIMIT,
        GAS_PRICE,
        sender_account.nonce + 1,
        chain_id,
        None::<TransactionData>,
    )
    .expect("creating tx should succeed");
    let tx = ProtocolTransaction::Moonlight(tx);

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://{local_addr}/on/transactions/propagate"))
        .header("Content-Type", "application/octet-stream")
        .body(tx.to_var_bytes())
        .send()
        .await
        .expect("requesting should succeed");
    assert_eq!(response.status(), reqwest::StatusCode::ACCEPTED);

    let invalid_tx = MoonlightTransaction::new(
        &sender_sk,
        None,
        1,
        0,
        GAS_LIMIT,
        GAS_PRICE,
        sender_account.nonce,
        chain_id,
        None::<TransactionData>,
    )
    .expect("creating invalid tx should succeed");
    let invalid_bytes =
        ProtocolTransaction::Moonlight(invalid_tx).to_var_bytes();

    let response = client
        .post(format!("http://{local_addr}/on/transactions/propagate"))
        .header("Content-Type", "application/octet-stream")
        .body(invalid_bytes)
        .send()
        .await
        .expect("requesting should succeed");

    assert_eq!(
        response.status(),
        reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        "invalid tx should be rejected by preverify"
    );

    let body = response
        .text()
        .await
        .expect("reading error response should succeed");
    assert!(
        body.contains("not accepted"),
        "expected preverify failure in response, got {body}"
    );
}
