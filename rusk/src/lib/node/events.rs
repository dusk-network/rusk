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
use tracing::error;

use crate::http::RuesEvent;

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
                if let Err(e) = self.rues_sender.send(msg.into()) {
                    // NB: This service receives all events and forwards them to
                    // RUES. We can forward them here to the
                    // ArchivistSrv too and directly be able to store all
                    // events.
                    error!("Cannot send to rues {e:?}");
                }
            }
        }
    }

    /// Returns service name.
    fn name(&self) -> &'static str {
        "chain event streamer"
    }
}
