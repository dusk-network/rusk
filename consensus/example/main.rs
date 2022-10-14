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
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex};

use consensus::commons::RoundUpdate;
use consensus::consensus::Consensus;
use consensus::messages::Message;
use consensus::user::provisioners::{Provisioners, DUSK};

use consensus::util::pending_queue::PendingQueue;
use consensus::util::pubkey::PublicKey;
use tokio::time;

mod mocks;

const MOCKED_PROVISIONERS_NUM: u64 = 5;

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

fn generate_provisioners_from_keys(keys: Vec<(SecretKey, PublicKey)>) -> Provisioners {
    let mut p = Provisioners::new();

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
    let (aggr_sender_bridge, mut aggr_recv_bridge) = mpsc::channel::<Message>(1000);

    // Initialize N dummy provisioners
    let keys = generate_keys(MOCKED_PROVISIONERS_NUM);
    let provisioners = generate_provisioners_from_keys(keys.clone());

    // Spawn N virtual nodes
    for key in keys.into_iter() {
        let inbound = PendingQueue::new("inbound_main_loop");
        let outbound = PendingQueue::new("outbound_main_loop");

        let aggr_inbound = PendingQueue::new("inbound_agreement");
        let aggr_outbound = PendingQueue::new("outbound_agreement");

        // Spawn a node which simulates a provisioner running its own consensus instance.
        spawn_node(
            key,
            provisioners.clone(),
            inbound.clone(),
            outbound.clone(),
            aggr_inbound.clone(),
            aggr_outbound.clone(),
        );

        // Bridge all so that provisioners can exchange messages in a single-process setup.
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
                    if to_inbound.is_duplicate(&msg) {
                        continue;
                    };

                    let _ = to_inbound.send(msg.clone()).await;
                }
            }
        }
    });

    tokio::spawn(async move {
        loop {
            if let Some(msg) = aggr_recv_bridge.recv().await {
                for to_inbound in agr_to_inbound.iter_mut() {
                    if to_inbound.is_duplicate(&msg) {
                        continue;
                    };

                    let _ = to_inbound.send(msg.clone()).await;
                }
            }
        }
    });

    time::sleep(Duration::from_secs(12000)).await;
}

/// spawn_node runs a separate thread-pool (tokio::runtime) that drives a single instance of consensus.
fn spawn_node(
    keys: (SecretKey, PublicKey),
    p: Provisioners,
    inbound_msgs: PendingQueue,
    outbound_msgs: PendingQueue,
    aggr_inbound_queue: PendingQueue,
    aggr_outbound_queue: PendingQueue,
) {
    let _ = thread::spawn(move || {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(3)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let mut c = Consensus::new(
                    inbound_msgs,
                    outbound_msgs,
                    aggr_inbound_queue,
                    aggr_outbound_queue,
                    Arc::new(Mutex::new(mocks::Executor {})),
                );

                // Run consensus for N rounds
                for i in 0..1000 {
                    let (_cancel_tx, cancel_rx) = oneshot::channel::<i32>();
                    let _ = c
                        .spin(RoundUpdate::new(i, keys.1, keys.0), p.clone(), cancel_rx)
                        .await;
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
