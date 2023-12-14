// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod acceptor;
mod consensus;
mod fallback;
mod fsm;
mod genesis;

use crate::database::Ledger;
use crate::{database, vm, Network};
use crate::{LongLivedService, Message};
use anyhow::Result;

use std::sync::Arc;
use tracing::{error, info, warn};

use async_trait::async_trait;

use node_data::ledger::{to_str, Block, BlockWithLabel, Label};

use node_data::message::AsyncQueue;
use node_data::message::{Payload, Topics};
use tokio::sync::RwLock;
use tokio::time::{sleep_until, Instant};

use std::time::Duration;

use self::acceptor::Acceptor;
use self::fsm::SimpleFSM;

pub use acceptor::verify_block_cert;

const TOPICS: &[u8] = &[
    Topics::Block as u8,
    Topics::Candidate as u8,
    Topics::Validation as u8,
    Topics::Ratification as u8,
    Topics::Quorum as u8,
];

const ACCEPT_BLOCK_TIMEOUT_SEC: Duration = Duration::from_secs(20);

pub struct ChainSrv {
    /// Inbound wire messages queue
    inbound: AsyncQueue<Message>,
    keys_path: String,
}

#[async_trait]
impl<N: Network, DB: database::DB, VM: vm::VMExecution>
    LongLivedService<N, DB, VM> for ChainSrv
{
    async fn execute(
        &mut self,
        network: Arc<RwLock<N>>,
        db: Arc<RwLock<DB>>,
        vm: Arc<RwLock<VM>>,
    ) -> anyhow::Result<usize> {
        // Register routes
        LongLivedService::<N, DB, VM>::add_routes(
            self,
            TOPICS,
            self.inbound.clone(),
            &network,
        )
        .await?;

        // Restore/Load most recent block
        let mrb = Self::load_most_recent_block(db.clone()).await?;

        let provisioners_list = vm.read().await.get_provisioners()?;

        // Initialize Acceptor and trigger consensus task
        let acc = Acceptor::init_consensus(
            &self.keys_path,
            mrb,
            provisioners_list,
            db,
            network.clone(),
            vm.clone(),
        )
        .await?;
        let acc = Arc::new(RwLock::new(acc));

        // Start-up FSM instance
        let mut fsm = SimpleFSM::new(acc.clone(), network.clone());

        let outbound_chan = acc.read().await.get_outbound_chan().await;
        let result_chan = acc.read().await.get_result_chan().await;

        // Accept_Block timeout is activated when a node is unable to accept a
        // valid block within a specified time frame.
        let mut timeout = Self::next_timeout();

        // Message loop for Chain context
        loop {
            tokio::select! {
                biased;
                // Receives results from the upper layer
                recv = &mut result_chan.recv() => {
                    match recv? {
                        Ok(blk) => {
                            info!(
                                event = "block received",
                                src = "consensus",
                                blk_height = blk.header().height,
                                blk_hash = to_str(&blk.header().hash),
                            );

                            // Handles a block that originates from local consensus
                            // TODO: Remove the redundant blk.clone()
                            if let Err(e) = fsm.on_event(&blk, &Message::new_block(Box::new(blk.clone()))).await  {
                                 error!(event = "fsm::on_event failed", src = "consensus",  err = format!("{}",e));
                            } else {
                                timeout = Self::next_timeout();
                            }
                        }
                        Err(e) => {
                             warn!("consensus err: {:?}", e);
                        }
                    }
                },
                // Handles any inbound wire.
                // Component should either process it or re-route it to the next upper layer
                recv =  self.inbound.recv() => {
                    let msg = recv?;
                    match &msg.payload {
                        Payload::Block(blk) => {
                           info!(
                                event = "block received",
                                src = "wire",
                                blk_height = blk.header().height,
                                blk_hash = to_str(&blk.header().hash),
                            );

                            if let Err(e) = fsm.on_event(blk, &msg).await  {
                                 error!(event = "fsm::on_event failed", src = "wire", err = format!("{}",e));
                            } else {
                                timeout = Self::next_timeout();
                            }
                        }

                        // Re-route message to the acceptor
                        Payload::Candidate(_)
                        | Payload::Validation(_)
                        | Payload::Ratification(_)
                        | Payload::Quorum(_) => {
                            if let Err(e) = acc.read().await.reroute_msg(msg).await {
                                warn!("Unable to reroute_msg to the acceptor: {e}");
                            }
                        }
                        _ => warn!("invalid inbound message"),
                    }
                },
                // Re-routes messages originated from Consensus (upper) layer to the network layer.
                recv = &mut outbound_chan.recv() => {
                    let msg = recv?;
                    if let Err(e) = network.read().await.broadcast(&msg).await {
                        warn!("Unable to re-route message {e}");
                    }
                },
                // Handles accept_block_timeout event
                _ = sleep_until(timeout) => {
                    fsm.on_idle(ACCEPT_BLOCK_TIMEOUT_SEC).await;
                    timeout = Self::next_timeout();
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
            keys_path,
        }
    }

    /// Load both most recent and last_finalized blocks from persisted ledger.
    ///
    /// Panics
    ///
    /// If register entry is read but block is not found.
    async fn load_most_recent_block<DB: database::DB>(
        db: Arc<RwLock<DB>>,
    ) -> Result<BlockWithLabel> {
        let mut blk = Block::default();

        db.read().await.update(|t| {
            blk = match t.get_register()? {
                Some(r) => t.fetch_block(&r.mrb_hash)?.unwrap(),
                None => {
                    // Lack of register record means the loaded database is
                    // either malformed or empty.
                    let genesis_blk = genesis::generate_state();

                    // Persist genesis block
                    t.store_block(genesis_blk.header(), &[], Label::Final)?;

                    genesis_blk
                }
            };
            Ok(())
        })?;

        let label = db
            .read()
            .await
            .view(|t| t.fetch_block_label_by_height(blk.header().height))?
            .unwrap();

        tracing::info!(
            event = "Ledger block loaded",
            height = blk.header().height,
            hash = hex::encode(blk.header().hash),
            state_root = hex::encode(blk.header().state_hash),
            label = format!("{:?}", label),
        );

        Ok(BlockWithLabel::new_with_label(blk, label))
    }

    fn next_timeout() -> Instant {
        Instant::now()
            .checked_add(ACCEPT_BLOCK_TIMEOUT_SEC)
            .unwrap()
    }
}
