// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::data::Topics;
use crate::utils::PendingQueue;
use crate::{data, Network};
use crate::{LongLivedService, Message};
use async_trait::async_trait;
use tokio::sync::RwLock;

use std::sync::Arc;

const TOPICS: &[u8] = &[data::Topics::Tx as u8];

#[derive(Default)]
pub struct MempoolSrv {
    inbound: PendingQueue<Message>,
}

#[async_trait]
impl<N: Network> LongLivedService<N> for MempoolSrv {
    async fn execute(
        &mut self,
        network: Arc<RwLock<N>>,
    ) -> anyhow::Result<usize> {
        self.add_routes(TOPICS, self.inbound.clone(), &network)
            .await?;

        loop {
            if let Ok(msg) = self.inbound.recv().await {
                match msg.topic {
                    Topics::Tx => {
                        if self.handle_tx(&msg).is_ok() {
                            _ = network.read().await.repropagate(&msg, 0);
                        }
                    }
                    _ => todo!(),
                };
            }
        }
    }

    /// Returns service name.
    fn name(&self) -> &'static str {
        "mempool"
    }
}

impl MempoolSrv {
    fn handle_tx(&mut self, msg: &Message) -> anyhow::Result<()> {
        // TODO: Verify

        // TODO: Put in mempool storage
        Ok(())
    }
}
