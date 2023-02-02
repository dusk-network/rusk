// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::data::Topics;
use crate::utils::PendingQueue;
use crate::{data, database, Network};
use crate::{LongLivedService, Message};
use async_trait::async_trait;
use tokio::sync::RwLock;

use std::sync::Arc;

const TOPICS: &[u8] = &[data::Topics::Tx as u8];

#[derive(Default)]
pub struct MempoolSrv {
    inbound: PendingQueue<Message>,
}

pub struct TxFilter {}
impl crate::Filter for TxFilter {
    fn filter(&mut self, msg: &Message) -> anyhow::Result<()> {
        // TODO: Ensure transaction does not exist in the mempool state
        // TODO: Ensure transaction does not exist in blockchain
        // TODO: Check  Nullifier
        Ok(())
    }
}

#[async_trait]
impl<N: Network, DB: database::DB> LongLivedService<N, DB> for MempoolSrv {
    async fn execute(
        &mut self,
        network: Arc<RwLock<N>>,
        db: Arc<RwLock<DB>>,
    ) -> anyhow::Result<usize> {
        LongLivedService::<N, DB>::add_routes(
            self,
            TOPICS,
            self.inbound.clone(),
            &network,
        )
        .await?;

        // Add a filter that will discard any transactions invalid to the actual
        // mempool, blockchain state.
        LongLivedService::<N, DB>::add_filter(
            self,
            data::Topics::Tx.into(),
            Box::new(TxFilter {}),
            &network,
        )
        .await?;

        loop {
            if let Ok(msg) = self.inbound.recv().await {
                match msg.topic {
                    Topics::Tx => {
                        if self.handle_tx(&msg).is_ok() {
                            network.read().await.repropagate(&msg, 0).await;
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
        // TODO: Preverify

        // TODO: Put in mempool storage
        Ok(())
    }
}
