// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_consensus::commons::TimeoutSet;
use dusk_consensus::config::EMERGENCY_MODE_ITERATION_THRESHOLD;
use dusk_consensus::errors::ConsensusError;

use super::TestNetwork;

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
    let last_iter = (
        network.tip_header.hash,
        EMERGENCY_MODE_ITERATION_THRESHOLD,
    );
    for node in &network.nodes {
        let mut db = node.db.lock().await;
        db.last_iter = last_iter;
    }
}

