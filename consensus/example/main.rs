// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381_sign::SecretKey;
use rand::rngs::StdRng;
use rand_core::SeedableRng;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, oneshot, Mutex};

use dusk_consensus::commons::RoundUpdate;
use dusk_consensus::consensus::Consensus;
use dusk_consensus::user::provisioners::{Provisioners, DUSK};
use node_data::message::{AsyncQueue, Message};

use node_data::bls::PublicKey;
use node_data::ledger;
use tokio::time;

mod mocks;

const MOCKED_PROVISIONERS_NUM: u64 = 10;

fn generate_keys(n: u64) -> Vec<(SecretKey, PublicKey)> {
    let mut keys = vec![];

    for i in 0..n {
        let rng = &mut StdRng::seed_from_u64(i);
        let sk = dusk_bls12_381_sign::SecretKey::random(rng);
        keys.push((
            sk,
            PublicKey::new(dusk_bls12_381_sign::PublicKey::from(&sk)),
        ));
    }

    keys
}

fn generate_provisioners_from_keys(
    keys: Vec<(SecretKey, PublicKey)>,
) -> Provisioners {
    let mut p = Provisioners::default();

    for (pos, (_, pk)) in keys.into_iter().enumerate() {
        p.add_member_with_value(pk, 1000 * (pos as u64) * DUSK);
    }

    p
}

async fn perform_basic_run() {
    let mut all_to_inbound = vec![];
    let mut agr_to_inbound = vec![];

    // TODO: Use here broadcast
    let (sender_bridge, mut recv_bridge) = mpsc::channel::<Message>(1000);
    let (aggr_sender_bridge, mut aggr_recv_bridge) =
        mpsc::channel::<Message>(1000);

    // Initialize N dummy provisioners
    let keys = generate_keys(MOCKED_PROVISIONERS_NUM);
    let provisioners = generate_provisioners_from_keys(keys.clone());

    // Spawn N virtual nodes
    for key in keys.into_iter() {
        let inbound = AsyncQueue::default();
        let outbound = AsyncQueue::default();

        let aggr_inbound = AsyncQueue::default();
        let aggr_outbound = AsyncQueue::default();

        // Spawn a node which simulates a provisioner running its own consensus
        // instance.
        spawn_node(
            key,
            provisioners.clone(),
            inbound.clone(),
            outbound.clone(),
            aggr_inbound.clone(),
            aggr_outbound.clone(),
        );

        // Bridge all so that provisioners can exchange messages in a
        // single-process setup.
        all_to_inbound.push(inbound);
        agr_to_inbound.push(aggr_inbound);

        let bridge = sender_bridge.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(msg) = outbound.recv().await {
                    let _ = bridge.send(msg.clone()).await;
                }
            }
        });

        let aggr_bridge = aggr_sender_bridge.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(msg) = aggr_outbound.recv().await {
                    let _ = aggr_bridge.send(msg.clone()).await;
                }
            }
        });
    }

    // clone bridge-ed messages to all provisioners.
    tokio::spawn(async move {
        loop {
            if let Some(msg) = recv_bridge.recv().await {
                for to_inbound in all_to_inbound.iter_mut() {
                    /* TODO
                    if to_inbound.is_duplicate(&msg) {
                        continue;
                    };

                     */

                    let _ = to_inbound.send(msg.clone()).await;
                }
            }
        }
    });

    tokio::spawn(async move {
        loop {
            if let Some(msg) = aggr_recv_bridge.recv().await {
                for to_inbound in agr_to_inbound.iter_mut() {
                    /* TODO
                    if to_inbound.is_duplicate(&msg) {
                        continue;
                    };
                     */

                    let _ = to_inbound.send(msg.clone()).await;
                }
            }
        }
    });

    time::sleep(Duration::from_secs(12000)).await;
}

/// spawn_node runs a separate thread-pool (tokio::runtime) that drives a single
/// instance of consensus.
fn spawn_node(
    keys: (SecretKey, PublicKey),
    p: Provisioners,
    inbound_msgs: AsyncQueue<Message>,
    outbound_msgs: AsyncQueue<Message>,
    aggr_inbound_queue: AsyncQueue<Message>,
    aggr_outbound_queue: AsyncQueue<Message>,
) {
    let _ = thread::spawn(move || {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let mut c = Consensus::new(
                    inbound_msgs,
                    outbound_msgs,
                    aggr_inbound_queue,
                    aggr_outbound_queue,
                    Arc::new(Mutex::new(crate::mocks::Executor {})),
                    Arc::new(Mutex::new(crate::mocks::SimpleDB::default())),
                );

                let mut cumulative_block_time = 0f64;
                // Run consensus for N rounds
                for i in 0..1000 {
                    let (_cancel_tx, cancel_rx) = oneshot::channel::<i32>();

                    let before = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();

                    let blk = ledger::Block::new(
                        ledger::Header {
                            version: 0,
                            height: i,
                            timestamp: 0,
                            prev_block_hash: [0u8; 32],
                            seed: Default::default(),
                            state_hash: [0u8; 32],
                            event_hash: [0u8; 32],
                            generator_bls_pubkey: Default::default(),
                            txroot: [0u8; 32],
                            gas_limit: 0,
                            iteration: 0,
                            hash: [0u8; 32],
                            ..Default::default()
                        },
                        vec![],
                    )
                    .unwrap();

                    let _ = c
                        .spin(
                            RoundUpdate::new(keys.1.clone(), keys.0, blk),
                            p.clone(),
                            cancel_rx,
                        )
                        .await;

                    // Calc block time
                    let block_time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        - before;
                    cumulative_block_time += block_time as f64;
                    let average_block_time =
                        cumulative_block_time / ((i + 1) as f64);
                    let average_block_time =
                        (average_block_time * 100f64).round() / 100f64;
                    tracing::info!(
                        bls_key = keys.1.to_bs58(),
                        round = i,
                        block_time,
                        average_block_time,
                    );
                }
            });
    });
}

fn main() {
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("failed");

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(perform_basic_run());
}
