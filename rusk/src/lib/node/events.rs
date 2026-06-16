// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::Arc;

use async_trait::async_trait;
use node::database::{self};
use node::{LongLivedService, Network};
use node_data::events::Event as ChainEvent;
use tokio::sync::broadcast;
use tokio::sync::mpsc::Receiver;

use crate::http::RuesEvent;

pub(crate) fn forward_event_to_rues(
    rues_sender: &broadcast::Sender<RuesEvent>,
    msg: ChainEvent,
) {
    // RUES receivers are created only while WebSocket clients are connected,
    // so having no receivers is an expected idle-state and not an error.
    if rues_sender.receiver_count() > 0 {
        let _ = rues_sender.send(msg.into());
    }
}

pub(crate) struct ChainEventStreamer {
    pub node_receiver: Receiver<ChainEvent>,
    pub rues_sender: broadcast::Sender<RuesEvent>,
}

#[async_trait]
impl<N: Network, DB: database::DB, VM: node::vm::VMExecution>
    LongLivedService<N, DB, VM> for ChainEventStreamer
{
    async fn execute(
        &mut self,
        _: Arc<tokio::sync::RwLock<N>>,
        _: Arc<tokio::sync::RwLock<DB>>,
        _: Arc<tokio::sync::RwLock<VM>>,
    ) -> anyhow::Result<usize> {
        loop {
            if let Some(msg) = self.node_receiver.recv().await {
                forward_event_to_rues(&self.rues_sender, msg);
            }
        }
    }

    /// Returns service name.
    fn name(&self) -> &'static str {
        "chain event streamer"
    }
}
