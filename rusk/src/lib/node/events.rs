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
#[cfg(feature = "archive")]
use {
    node_data::archive::ArchivalData, node_data::events::BLOCK_FINALIZED,
    serde_json::Value, tokio::sync::mpsc::Sender,
};

use crate::http::RuesEvent;

pub(crate) struct ChainEventStreamer {
    pub node_receiver: Receiver<ChainEvent>,
    pub rues_sender: broadcast::Sender<RuesEvent>,
    #[cfg(feature = "archive")]
    pub archivist_sender: Sender<ArchivalData>,
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
                if let Err(e) = self.rues_sender.send(msg.clone().into()) {
                    error!("Cannot send to rues {e:?}");
                }

                #[cfg(feature = "archive")]
                {
                    // NB: This is a temporary solution to send finalized and
                    // deleted blocks to the archivist in a decoupled way.
                    // We can remove this once the consensus acceptor can send
                    // these events directly to the archivist service.
                    match msg.topic {
                        // "statechange" & "deleted" are only in msg.component
                        // == "blocks"
                        "statechange" => {
                            if let Some(json_val) = msg.data {
                                let state = json_val
                                    .get("state")
                                    .and_then(Value::as_str)
                                    .unwrap_or_default();
                                let at_height = json_val
                                    .get("atHeight")
                                    .and_then(Value::as_u64)
                                    .unwrap_or_default();

                                if state == BLOCK_FINALIZED {
                                    if let Err(e) = self
                                        .archivist_sender
                                        .try_send(ArchivalData::FinalizedBlock(
                                            at_height,
                                            msg.entity.clone(),
                                        ))
                                    {
                                        error!(
                                            "Cannot send to archivist {e:?}"
                                        );
                                    };
                                }
                            };
                        }
                        "deleted" => {
                            if let Some(json_val) = msg.data {
                                let at_height = json_val
                                    .get("atHeight")
                                    .and_then(Value::as_u64)
                                    .unwrap_or_default();

                                if let Err(e) = self.archivist_sender.try_send(
                                    ArchivalData::DeletedBlock(
                                        at_height,
                                        msg.entity.clone(),
                                    ),
                                ) {
                                    error!("Cannot send to archivist {e:?}");
                                };
                            };
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    /// Returns service name.
    fn name(&self) -> &'static str {
        "chain event streamer"
    }
}
