// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg(all(feature = "chain", feature = "recovery-state"))]

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use async_trait::async_trait;
use dusk_core::transfer::data::TransactionData;
use dusk_rusk_test::{RuskVmConfig, TestContext};
use hex::encode as hex_encode;
use node::database::{DB, DatabaseOptions, Ledger, Mempool};
use node::mempool::MempoolSrv;
use node::mempool::conf::Params as MempoolParams;
use node::{BoxedFilter, LongLivedService, Network};
use node_data::events::Event;
use node_data::ledger::{Header, Label, LedgerTransaction, SpendingId};
use node_data::message::{AsyncQueue, Message, Payload, Topics, payload};
use tempfile::TempDir;
use tokio::task::JoinHandle;
use tokio::time::sleep;

const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;
const GAS_LIMIT: u64 = 75_000;
const GAS_PRICE: u64 = 1;
const QUIESCENT_WAIT: Duration = Duration::from_millis(200);
const ROUTE_WAIT_TIMEOUT: Duration = Duration::from_secs(2);
const OBSERVE_TIMEOUT: Duration = Duration::from_secs(2);
const DROP_WAIT: Duration = Duration::from_secs(6);

#[derive(Clone, Default)]
struct TestNetwork {
    tx_route: Arc<Mutex<Option<AsyncQueue<Message>>>>,
    broadcasts: Arc<Mutex<Vec<[u8; 32]>>>,
}

impl TestNetwork {
    async fn route_tx(&self, tx: LedgerTransaction) -> Result<()> {
        let msg: Message = tx.into();
        let deadline = Instant::now() + ROUTE_WAIT_TIMEOUT;

        loop {
            let maybe_queue: Option<AsyncQueue<Message>> =
                self.tx_route.lock().unwrap().clone();
            if let Some(queue) = maybe_queue {
                queue.try_send(msg.clone());
                return Ok(());
            }

            if Instant::now() >= deadline {
                bail!("timed out waiting for mempool tx route");
            }

            sleep(Duration::from_millis(10)).await;
        }
    }

    fn broadcast_count(&self) -> usize {
        self.broadcasts.lock().unwrap().len()
    }
}

#[async_trait]
impl Network for TestNetwork {
    async fn broadcast(&self, msg: &Message) -> anyhow::Result<()> {
        if let Payload::Transaction(tx) = &msg.payload {
            self.broadcasts.lock().unwrap().push(tx.id());
        }
        Ok(())
    }

    async fn flood_request(
        &self,
        _msg_inv: &payload::Inv,
        _ttl_as_sec: Option<u64>,
        _hops_limit: u16,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn send_to_peer(
        &self,
        _msg: Message,
        _peer_addr: SocketAddr,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn send_to_alive_peers(
        &self,
        _msg: Message,
        _amount: usize,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn add_route(
        &mut self,
        msg_type: u8,
        queue: AsyncQueue<Message>,
    ) -> anyhow::Result<()> {
        if msg_type == Topics::Tx as u8 {
            *self.tx_route.lock().unwrap() = Some(queue);
        }
        Ok(())
    }

    async fn add_filter(
        &mut self,
        _msg_type: u8,
        _filter: BoxedFilter,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn get_info(&self) -> anyhow::Result<String> {
        Ok(String::new())
    }

    fn public_addr(&self) -> &SocketAddr {
        static LOCAL_ADDR: SocketAddr =
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0));
        &LOCAL_ADDR
    }

    async fn alive_nodes_count(&self) -> usize {
        0
    }
}

struct MempoolHarness {
    _db_dir: TempDir,
    test_context: TestContext,
    node: node::Node<TestNetwork, node::database::rocksdb::Backend, rusk::Rusk>,
    network: TestNetwork,
    event_receiver: tokio::sync::mpsc::Receiver<Event>,
    service_task: JoinHandle<anyhow::Result<usize>>,
}

impl MempoolHarness {
    async fn new() -> Result<Self> {
        let vm_config =
            RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);
        let test_context = TestContext::instantiate(
            include_str!("config/sequential_nonce.toml"),
            vm_config,
        )
        .await?;

        let db_dir = tempfile::tempdir()?;
        let backend = node::database::rocksdb::Backend::create_or_open(
            db_dir.path(),
            DatabaseOptions::default(),
        );
        let state_root = test_context.state_root();
        backend.update(|db| {
            let header = Header {
                height: 0,
                state_hash: state_root,
                hash: [1u8; 32],
                ..Default::default()
            };
            db.store_block(&header, &[], &[], Label::Final(0))?;
            Ok(())
        })?;

        let network = TestNetwork::default();
        let node = node::Node::new(
            network.clone(),
            backend,
            test_context.rusk().clone(),
        );

        let (event_sender, event_receiver) =
            tokio::sync::mpsc::channel::<Event>(32);
        let conf = MempoolParams {
            mempool_download_redundancy: Some(0),
            idle_interval: Some(Duration::from_secs(60)),
            ..Default::default()
        };

        let mut mempool = MempoolSrv::new(conf, event_sender);
        let network_handle = node.network();
        let db_handle = node.database();
        let vm_handle = node.vm_handler();
        let service_task = tokio::spawn(async move {
            mempool.execute(network_handle, db_handle, vm_handle).await
        });

        let harness = Self {
            _db_dir: db_dir,
            test_context,
            node,
            network,
            event_receiver,
            service_task,
        };
        Ok(harness)
    }

    fn make_tx(&self, nonce: u64) -> LedgerTransaction {
        self.make_tx_with_fee(nonce, GAS_LIMIT, GAS_PRICE)
    }

    fn make_tx_with_fee(
        &self,
        nonce: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> LedgerTransaction {
        let wallet = self.test_context.wallet();
        let receiver = wallet
            .account_public_key(1)
            .expect("receiver key should exist");
        let tx = wallet
            .moonlight_transaction(
                0,
                Some(receiver),
                1,
                0,
                gas_limit,
                gas_price,
                None::<TransactionData>,
                Some(nonce),
            )
            .expect("creating moonlight tx should succeed");

        LedgerTransaction::from_protocol_for_ledger(tx, 1)
    }

    async fn route_tx(&self, tx: LedgerTransaction) -> Result<()> {
        self.network.route_tx(tx).await
    }

    async fn route_nonce(&self, nonce: u64) -> Result<()> {
        self.network.route_tx(self.make_tx(nonce)).await
    }

    async fn mempool_count(&self) -> usize {
        self.node
            .database()
            .read()
            .await
            .view(|db| db.mempool_txs_count())
    }

    fn broadcast_count(&self) -> usize {
        self.network.broadcast_count()
    }

    async fn mempool_tx_id_for_nonce(
        &self,
        nonce: u64,
    ) -> Result<Option<[u8; 32]>> {
        let sender = self
            .test_context
            .wallet()
            .account_public_key(0)
            .expect("sender key should exist");

        self.node.database().read().await.view(|db| {
            let ids =
                db.mempool_txs_by_spendable_ids(&[SpendingId::AccountNonce(
                    sender, nonce,
                )]);
            Ok(ids.into_iter().next())
        })
    }

    fn drain_events(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        while let Ok(event) = self.event_receiver.try_recv() {
            events.push(event);
        }
        events
    }

    async fn wait_for_counts(
        &self,
        expected_mempool: usize,
        expected_broadcasts: usize,
    ) -> Result<()> {
        let deadline = Instant::now() + OBSERVE_TIMEOUT;
        loop {
            let mempool_count = self.mempool_count().await;
            let broadcast_count = self.broadcast_count();
            if mempool_count == expected_mempool
                && broadcast_count == expected_broadcasts
            {
                return Ok(());
            }

            if Instant::now() >= deadline {
                bail!(
                    "timed out waiting for mempool/broadcast counts {expected_mempool}/{expected_broadcasts}, last observed {mempool_count}/{broadcast_count}"
                );
            }

            sleep(Duration::from_millis(20)).await;
        }
    }
}

impl Drop for MempoolHarness {
    fn drop(&mut self) {
        self.service_task.abort();
    }
}

fn tx_topics(
    events: &[Event],
    tx: &LedgerTransaction,
) -> Vec<(&'static str, Option<String>)> {
    let entity = hex_encode(tx.id());
    events
        .iter()
        .filter(|event| {
            event.component == "transactions" && event.entity == entity
        })
        .map(|event| {
            let reason = event
                .data
                .as_ref()
                .and_then(|data| data.get("reason"))
                .and_then(|value| value.as_str())
                .map(str::to_owned);
            (event.topic, reason)
        })
        .collect()
}

fn assert_tx_topics(
    events: &[Event],
    tx: &LedgerTransaction,
    expected: &[(&'static str, Option<&str>)],
) {
    let expected = expected
        .iter()
        .map(|(topic, reason)| (*topic, reason.map(str::to_owned)))
        .collect::<Vec<_>>();
    assert_eq!(tx_topics(events, tx), expected);
}

#[tokio::test(flavor = "multi_thread")]
async fn future_nonce_prequeue_admits_gap_fill_and_drops_late_gap_fill()
-> Result<()> {
    let harness = MempoolHarness::new().await?;

    harness.route_nonce(2).await?;
    harness.route_nonce(3).await?;

    sleep(QUIESCENT_WAIT).await;
    assert_eq!(harness.mempool_count().await, 0);
    assert_eq!(harness.broadcast_count(), 0);

    harness.route_nonce(1).await?;
    harness.wait_for_counts(3, 3).await?;

    harness.route_nonce(5).await?;
    sleep(DROP_WAIT).await;
    assert_eq!(harness.mempool_count().await, 3);
    assert_eq!(harness.broadcast_count(), 3);

    harness.route_nonce(4).await?;
    harness.wait_for_counts(4, 4).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn future_nonce_prequeue_replaces_staged_tx_before_gap_fill() -> Result<()>
{
    let mut harness = MempoolHarness::new().await?;

    let original = harness.make_tx_with_fee(2, GAS_LIMIT, GAS_PRICE);
    let replacement = harness.make_tx_with_fee(2, GAS_LIMIT + 1, GAS_PRICE);

    harness.route_tx(original.clone()).await?;
    harness.route_tx(replacement.clone()).await?;

    sleep(QUIESCENT_WAIT).await;
    assert_eq!(harness.mempool_count().await, 0);
    assert_eq!(harness.broadcast_count(), 0);

    harness.route_nonce(1).await?;
    harness.wait_for_counts(2, 2).await?;
    sleep(QUIESCENT_WAIT).await;

    assert_eq!(
        harness.mempool_tx_id_for_nonce(2).await?,
        Some(replacement.id())
    );
    assert_ne!(replacement.id(), original.id());
    let events = harness.drain_events();
    assert_tx_topics(
        &events,
        &original,
        &[
            ("deferred", Some("missing_intermediate_nonce")),
            ("dropped", Some("replaced_in_prequeue")),
        ],
    );
    assert_tx_topics(
        &events,
        &replacement,
        &[
            ("deferred", Some("missing_intermediate_nonce")),
            ("included", None),
        ],
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn future_nonce_then_mempool_replacement_prefers_later_better_tx()
-> Result<()> {
    let mut harness = MempoolHarness::new().await?;

    let deferred = harness.make_tx_with_fee(2, GAS_LIMIT, GAS_PRICE);
    let mempool_replacement =
        harness.make_tx_with_fee(2, GAS_LIMIT + 2, GAS_PRICE);

    harness.route_tx(deferred.clone()).await?;
    sleep(QUIESCENT_WAIT).await;
    assert_eq!(harness.mempool_count().await, 0);

    harness.route_nonce(1).await?;
    harness.wait_for_counts(2, 2).await?;
    assert_eq!(
        harness.mempool_tx_id_for_nonce(2).await?,
        Some(deferred.id())
    );

    harness.route_tx(mempool_replacement.clone()).await?;
    harness.wait_for_counts(2, 3).await?;
    sleep(QUIESCENT_WAIT).await;

    assert_eq!(
        harness.mempool_tx_id_for_nonce(2).await?,
        Some(mempool_replacement.id())
    );
    assert_ne!(mempool_replacement.id(), deferred.id());
    let events = harness.drain_events();
    assert_tx_topics(
        &events,
        &deferred,
        &[
            ("deferred", Some("missing_intermediate_nonce")),
            ("included", None),
            ("removed", None),
        ],
    );
    assert_tx_topics(&events, &mempool_replacement, &[("included", None)]);

    Ok(())
}
