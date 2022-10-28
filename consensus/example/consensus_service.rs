// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use consensus::commons::RoundUpdate;
use consensus::consensus::Consensus;
use consensus::contract_state::{CallParams, Error, Operations, Output, StateRoot};
use consensus::user::provisioners::{Provisioners, DUSK};
use consensus::util::pending_queue::PendingQueue;
use consensus::util::pubkey::PublicKey;
use dusk_bls12_381_sign::SecretKey;
use rand::SeedableRng;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{oneshot, Mutex};

pub fn run_main_loop(
    provisioners_num: usize,
    prov_id: usize,
    inbound: PendingQueue,
    outbound: PendingQueue,
    agr_inbound: PendingQueue,
    agr_outbound: PendingQueue,
) {
    // Initialize N hard-coded provisioners
    let keys = generate_keys(provisioners_num as u64);
    let provisioners = generate_provisioners_from_keys(keys.clone());

    spawn_consensus_in_thread_pool(
        keys[prov_id],
        provisioners,
        inbound,
        outbound,
        agr_inbound,
        agr_outbound,
    );
}

/// spawn_node runs a separate thread-pool (tokio::runtime) that drives a single instance of consensus.
fn spawn_consensus_in_thread_pool(
    keys: (SecretKey, PublicKey),
    p: Provisioners,
    inbound_msgs: PendingQueue,
    outbound_msgs: PendingQueue,
    agr_inbound_queue: PendingQueue,
    agr_outbound_queue: PendingQueue,
) {
    let _ = std::thread::spawn(move || {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2 + consensus::config::ACCUMULATOR_WORKERS_AMOUNT)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let mut c = Consensus::new(
                    inbound_msgs,
                    outbound_msgs,
                    agr_inbound_queue,
                    agr_outbound_queue,
                    Arc::new(Mutex::new(Executor {})),
                );

                let mut cumulative_block_time = 0f64;
                // Run consensus for N rounds
                for i in 0..1000 {
                    let (_cancel_tx, cancel_rx) = oneshot::channel::<i32>();

                    let before = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();

                    let _ = c
                        .spin(RoundUpdate::new(i, keys.1, keys.0), p.clone(), cancel_rx)
                        .await;

                    // Calc block time
                    let block_time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        - before;
                    cumulative_block_time += block_time as f64;
                    tracing::info!(
                        "bls_key={}, round={}, block_time={} average_block_time={:.2}",
                        keys.1.encode_short_hex(),
                        i,
                        block_time,
                        cumulative_block_time / ((i + 1) as f64)
                    );
                }
            });
    });
}

pub struct Executor {}
impl Operations for Executor {
    fn verify_state_transition(&self, _params: CallParams) -> Result<StateRoot, Error> {
        Ok([0; 32])
    }

    fn execute_state_transition(&self, _params: CallParams) -> Result<Output, Error> {
        Ok(Output::default())
    }

    fn accept(&self, _params: CallParams) -> Result<Output, Error> {
        Ok(Output::default())
    }

    fn finalize(&self, _params: CallParams) -> Result<Output, Error> {
        Ok(Output::default())
    }

    fn get_state_root(&self) -> Result<StateRoot, Error> {
        Ok([0; 32])
    }
}

fn generate_keys(n: u64) -> Vec<(SecretKey, PublicKey)> {
    let mut keys = vec![];

    for i in 0..n {
        let rng = &mut rand::rngs::StdRng::seed_from_u64(i);
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
