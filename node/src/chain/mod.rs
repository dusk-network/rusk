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

const TOPICS: &[u8] = &[data::Topics::Block as u8];

#[derive(Default)]
pub struct ChainSrv {
    inbound: PendingQueue<Message>,
}

#[async_trait]
impl<N: Network> LongLivedService<N> for ChainSrv {
    async fn execute(
        &mut self,
        network: Arc<RwLock<N>>,
    ) -> anyhow::Result<usize> {
        self.add_routes(TOPICS, self.inbound.clone(), &network)
            .await?;

        loop {
            if let Ok(msg) = self.inbound.recv().await {
                match msg.topic {
                    Topics::Block => {
                        // Try to validate message
                        if self.is_valid(&msg).is_ok() {
                            _ = network.read().await.repropagate(&msg, 0);

                            self.handle_block_msg(&msg);
                        }
                    }
                    _ => todo!(),
                }
            }
        }
    }

    /// Returns service name.
    fn name(&self) -> &'static str {
        "chain"
    }
}

impl ChainSrv {
    fn is_valid(&self, msg: &Message) -> anyhow::Result<()> {
        // TODO:
        Ok(())
    }

    fn handle_block_msg(&mut self, msg: &Message) {
        // TODO:
    }
}
