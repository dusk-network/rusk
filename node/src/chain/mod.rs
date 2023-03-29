// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod consensus;
mod genesis;

use crate::database::{Candidate, Ledger, Mempool};
use crate::{database, Network};
use crate::{LongLivedService, Message};
use async_trait::async_trait;
use dusk_consensus::commons::{ConsensusError, Database, RoundUpdate};
use dusk_consensus::consensus::Consensus;
use dusk_consensus::contract_state::{
    CallParams, Error, Operations, Output, StateRoot,
};
use dusk_consensus::user::provisioners::Provisioners;
use node_data::ledger::{Block, Hash};
use node_data::message::AsyncQueue;
use node_data::message::{Payload, Topics};
use node_data::Serializable;
use tokio::sync::{oneshot, Mutex, RwLock};
use tokio::task::JoinHandle;

use std::any;
use std::sync::Arc;

use self::consensus::Task;

const TOPICS: &[u8] = &[
    Topics::Block as u8,
    Topics::NewBlock as u8,
    Topics::Reduction as u8,
    Topics::AggrAgreement as u8,
    Topics::Agreement as u8,
];

pub struct ChainSrv {
    /// Inbound wire messages queue
    inbound: AsyncQueue<Message>,

    /// Most recently accepted block
    most_recent_block: Block,

    /// List of eligible provisioners of actual round
    eligible_provisioners: Provisioners,

    /// Upper layer consensus task
    upper: consensus::Task,
}

impl Drop for ChainSrv {
    fn drop(&mut self) {
        self.upper.abort();
    }
}

#[async_trait]
impl<N: Network, DB: database::DB> LongLivedService<N, DB> for ChainSrv {
    async fn execute(
        &mut self,
        network: Arc<RwLock<N>>,
        db: Arc<RwLock<DB>>,
    ) -> anyhow::Result<usize> {
        // Register routes
        LongLivedService::<N, DB>::add_routes(
            self,
            TOPICS,
            self.inbound.clone(),
            &network,
        )
        .await?;

        self.init::<N, DB>(&network, &db).await?;

        loop {
            tokio::select! {
                // Receives results from the upper layer
                recv = &mut self.upper.result.recv() => {
                    if let Ok(res) = recv {
                        match res {
                            Ok(blk) => {
                                if let Err(e) = self.accept_block::<DB>( &db, &blk).await {
                                    tracing::error!("failed to accept block: {}", e);
                                } else {
                                    network.read().await.
                                        broadcast(&Message::new_with_block(Box::new(blk))).await;
                                }
                            }
                            Err(e) => {
                                tracing::error!("consensus halted due to: {:?}", e);
                            }
                        }
                    }
                },
                // Receives inbound wire messages.
                // Component should either process it or re-route it to the next upper layer
                recv = &mut self.inbound.recv() => {
                    if let Ok(mut msg) = recv {
                        match &msg.payload {
                            Payload::Block(b) => {
                                if let Err(e) = self.accept_block::<DB>(&db, b).await {
                                    tracing::error!("failed to accept block: {}", e);
                                } else {
                                    network.read().await.broadcast(&msg).await;
                                }
                            }

                            // Re-route message to upper layer (in this case it is the Consensus layer)
                            Payload::NewBlock(_) |  Payload::Reduction(_) => {
                                self.upper.main_inbound.try_send(msg);
                            }
                            Payload::Agreement(_) | Payload::AggrAgreement(_) => {
                                self.upper.agreement_inbound.try_send(msg);
                            }
                            _ => tracing::warn!("invalid inbound message"),
                        }
                    }
                },
                // Re-routes messages originated from Consensus (upper) layer to the network layer.
                recv = &mut self.upper.outbound.recv() => {
                    if let Ok(msg) = recv {
                        network.read().await.broadcast(&msg).await;
                    }
                },
            }
        }
    }

    /// Returns service name.
    fn name(&self) -> &'static str {
        "chain"
    }
}

impl ChainSrv {
    pub fn new(keys_path: String) -> Self {
        Self {
            inbound: Default::default(),
            upper: Task::new_with_keys(keys_path),
            most_recent_block: Block::default(),
            eligible_provisioners: Provisioners::default(),
        }
    }

    async fn init<N: Network, DB: database::DB>(
        &mut self,
        network: &Arc<RwLock<N>>,
        db: &Arc<RwLock<DB>>,
    ) -> anyhow::Result<usize> {
        (self.most_recent_block, self.eligible_provisioners) =
            genesis::generate_state();

        self.upper.spawn(
            &self.most_recent_block.header,
            &self.eligible_provisioners,
            db,
        );

        anyhow::Ok(0)
    }

    async fn accept_block<DB: database::DB>(
        &mut self,
        db: &Arc<RwLock<DB>>,
        blk: &Block,
    ) -> anyhow::Result<()> {
        // Reset Consensus
        self.upper.abort();

        // TODO: Ensure block is valid
        // TODO: Call ExecuteStateTransition

        // Persist block
        db.read().await.update(|t| t.store_block(blk, true))?;
        self.most_recent_block = blk.clone();

        // Delete from mempool any transaction already included in the block
        db.read().await.update(|u| {
            for tx in blk.txs.iter() {
                u.delete_tx(tx.hash());
            }
            Ok(())
        })?;

        tracing::info!(
            "block accepted height:{} hash:{} txs_count: {}",
            blk.header.height,
            hex::encode(blk.header.hash),
            blk.txs.len(),
        );

        // Restart Consensus.
        // NB. This will be moved out of accept_block when Synchronizer is
        // implemented.
        self.upper.spawn(
            &self.most_recent_block.header,
            &self.eligible_provisioners,
            db,
        );

        Ok(())
    }
}
