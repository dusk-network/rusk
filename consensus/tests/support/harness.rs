// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::time::Duration;

use dusk_consensus::commons::TimeoutSet;
use dusk_consensus::config::EMERGENCY_MODE_ITERATION_THRESHOLD;
use dusk_consensus::errors::ConsensusError;

use node_data::message::payload::RatificationResult;
use node_data::message::Message;

use super::assertions::{
    assert_quorum_batch_invariants_with_network,
    assert_quorum_batch_invariants_with_network_for_round,
    assert_quorum_message_ok,
};
use super::{deliver_all, find_quorum, BufferedRouter, Envelope, TestNetwork};

pub type ConsensusCancel = tokio::sync::oneshot::Sender<i32>;
pub type ConsensusHandle = tokio::task::JoinHandle<Result<(), ConsensusError>>;

pub fn spawn_all(
    network: &TestNetwork,
    timeouts: TimeoutSet,
) -> (Vec<ConsensusCancel>, Vec<ConsensusHandle>) {
    let mut cancels = Vec::new();
    let mut handles = Vec::new();

    for node in &network.nodes {
        let ru = node.round_update(&network.tip_header, timeouts.clone());
        let (cancel, handle) =
            node.spawn_consensus(ru, network.provisioners.clone());
        cancels.push(cancel);
        handles.push(handle);
    }

    (cancels, handles)
}

pub async fn shutdown_all(
    cancels: Vec<ConsensusCancel>,
    handles: Vec<ConsensusHandle>,
) {
    for cancel in cancels {
        let _ = cancel.send(0);
    }
    for handle in handles {
        let _ = handle.await;
    }
}

pub async fn set_emergency_iteration(network: &TestNetwork) {
    let last_iter =
        (network.tip_header.hash, EMERGENCY_MODE_ITERATION_THRESHOLD);
    for node in &network.nodes {
        let mut db = node.db.lock().await;
        db.last_iter = last_iter;
    }
}

pub fn drain_pending(
    pending: &mut Vec<(Envelope, u32)>,
    tick: u32,
) -> Vec<Envelope> {
    let mut delivery = Vec::new();
    let mut keep = Vec::with_capacity(pending.len());
    for (env, deliver_at) in pending.drain(..) {
        if deliver_at <= tick {
            delivery.push(env);
        } else {
            keep.push((env, deliver_at));
        }
    }
    *pending = keep;
    delivery
}

#[derive(Clone, Copy, Debug)]
pub enum QuorumObservation {
    Batch,
    Delivery,
}

pub struct RunningNetwork {
    pub network: TestNetwork,
    router: BufferedRouter,
    cancels: Vec<ConsensusCancel>,
    handles: Vec<ConsensusHandle>,
    seen_quorums: HashMap<(u64, u8), RatificationResult>,
}

impl RunningNetwork {
    pub fn new(nodes: usize, seed: u64, timeouts: TimeoutSet) -> Self {
        Self::start(TestNetwork::new(nodes, seed), timeouts)
    }

    pub fn start(network: TestNetwork, timeouts: TimeoutSet) -> Self {
        let (cancels, handles) = spawn_all(&network, timeouts);
        let router = BufferedRouter::start(&network.nodes);
        Self {
            network,
            router,
            cancels,
            handles,
            seen_quorums: HashMap::new(),
        }
    }

    pub async fn shutdown(self) {
        shutdown_all(self.cancels, self.handles).await;
        self.router.stop();
    }

    pub async fn recv_batch(&self, timeout: Duration) -> Vec<Envelope> {
        self.router.recv_batch(timeout).await
    }

    pub fn deliver_all(&self, envelopes: &[Envelope]) {
        deliver_all(&self.network.nodes, envelopes);
    }

    pub fn assert_quorum_batch_invariants(&mut self, envelopes: &[Envelope]) {
        assert_quorum_batch_invariants_with_network(
            envelopes,
            &mut self.seen_quorums,
            &self.network,
        );
    }

    pub fn assert_quorum_batch_invariants_for_round(
        &mut self,
        envelopes: &[Envelope],
        verify_round: u64,
    ) {
        assert_quorum_batch_invariants_with_network_for_round(
            envelopes,
            &mut self.seen_quorums,
            &self.network,
            Some(verify_round),
        );
    }

    pub async fn drive_until<F>(
        &mut self,
        deadline: tokio::time::Instant,
        recv_timeout: Duration,
        mut deliver: F,
    ) where
        F: FnMut(Vec<Envelope>) -> Vec<Envelope>,
    {
        while tokio::time::Instant::now() < deadline {
            let batch = self.router.recv_batch(recv_timeout).await;
            if !batch.is_empty() {
                self.assert_quorum_batch_invariants(&batch);
            }

            let delivery = deliver(batch);
            if !delivery.is_empty() {
                self.deliver_all(&delivery);
            }
        }
    }

    pub async fn run_until_quorum<F>(
        &mut self,
        deadline: tokio::time::Instant,
        recv_timeout: Duration,
        observation: QuorumObservation,
        mut deliver: F,
    ) -> Option<Message>
    where
        F: FnMut(Vec<Envelope>) -> Vec<Envelope>,
    {
        while tokio::time::Instant::now() < deadline {
            let batch = self.router.recv_batch(recv_timeout).await;
            // Validate the raw outbound batch, but do not validate any
            // injected/faulty deliveries.
            let quorum_in_batch = if batch.is_empty() {
                None
            } else {
                self.assert_quorum_batch_invariants(&batch);
                find_quorum(&batch)
            };

            let delivery = deliver(batch);

            match observation {
                QuorumObservation::Batch => {
                    if let Some(q) = quorum_in_batch {
                        return Some(q);
                    }
                }
                QuorumObservation::Delivery => {
                    if let Some(q) = find_quorum(&delivery) {
                        return Some(q);
                    }
                }
            }

            if !delivery.is_empty() {
                self.deliver_all(&delivery);
            }
        }
        None
    }

    pub async fn expect_verified_quorum<F>(
        &mut self,
        deadline: tokio::time::Instant,
        recv_timeout: Duration,
        observation: QuorumObservation,
        deliver: F,
        expect_msg: &'static str,
    ) -> Message
    where
        F: FnMut(Vec<Envelope>) -> Vec<Envelope>,
    {
        let msg = self
            .run_until_quorum(deadline, recv_timeout, observation, deliver)
            .await
            .unwrap_or_else(|| panic!("{expect_msg}"));
        assert_quorum_message_ok(&self.network, &msg);
        msg
    }
}
