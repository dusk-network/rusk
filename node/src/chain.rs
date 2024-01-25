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

mod header_validation;

use self::acceptor::Acceptor;
use self::fsm::SimpleFSM;
use crate::database::rocksdb::MD_HASH_KEY;
use crate::database::{Ledger, Metadata};
use crate::{database, vm, Network};
use crate::{LongLivedService, Message};
use anyhow::Result;
use async_trait::async_trait;
use dusk_consensus::commons::ConsensusError;
use node_data::ledger::{to_str, BlockWithLabel, Label};
use node_data::message::AsyncQueue;
use node_data::message::{Payload, Topics};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use tokio::time::{sleep_until, Instant};
use tracing::{error, info, warn};

const TOPICS: &[u8] = &[
    Topics::Block as u8,
    Topics::Candidate as u8,
    Topics::Validation as u8,
    Topics::Ratification as u8,
    Topics::Quorum as u8,
];

const ACCEPT_BLOCK_TIMEOUT_SEC: Duration = Duration::from_secs(20);
const HEARTBEAT_SEC: Duration = Duration::from_secs(1);

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
        let mrb = Self::load_most_recent_block(db.clone(), vm.clone()).await?;

        let state_hash = mrb.inner().header().state_hash;
        let provisioners_list = vm.read().await.get_provisioners(state_hash)?;

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
        let mut heartbeat = Instant::now().checked_add(HEARTBEAT_SEC).unwrap();

        // Message loop for Chain context
        loop {
            tokio::select! {
                biased;
                // Receives results from the upper layer
                recv = &mut result_chan.recv() => {
                    let mut failed_consensus = false;
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
                            if let Err(err) = fsm.on_event(&blk, &Message::new_block(Box::new(blk.clone()))).await  {
                                // Internal consensus execution has produced an invalid block
                                error!(event = "failed_consensus",  ?err);
                                failed_consensus = true;
                            } else {
                                timeout = Self::next_timeout();
                            }
                        }
                        Err(ConsensusError::Canceled) => {
                            info!("consensus canceled");
                        }
                        Err(err) => {
                            // Internal consensus execution has terminated with an error instead of a valid block
                            failed_consensus = true;
                            error!(event = "failed_consensus", ?err);
                        }
                    }

                    if failed_consensus {
                        fsm.on_failed_consensus().await;
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
                 // Handles heartbeat event
                _ = sleep_until(heartbeat) => {
                    if let Err(err) = fsm.on_heartbeat_event().await {
                        error!(event = "heartbeat_failed", ?err);
                    }

                    heartbeat = Instant::now().checked_add(HEARTBEAT_SEC).unwrap();
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
    async fn load_most_recent_block<DB: database::DB, VM: vm::VMExecution>(
        db: Arc<RwLock<DB>>,
        vm: Arc<RwLock<VM>>,
    ) -> Result<BlockWithLabel> {
        let stored_block = db.read().await.update(|t| {
            Ok(t.op_read(MD_HASH_KEY)?.and_then(|mrb_hash| {
                t.fetch_block(&mrb_hash[..])
                    .expect("block to be found if metadata is set")
            }))
        })?;

        let block = match stored_block {
            Some(blk) => {
                let label = db
                    .read()
                    .await
                    .view(|t| {
                        t.fetch_block_label_by_height(blk.header().height)
                    })?
                    .unwrap();

                BlockWithLabel::new_with_label(blk, label)
            }
            None => {
                // Lack of register record means the loaded database is
                // either malformed or empty.
                let state = vm.read().await.get_state_root()?;
                let genesis_blk = genesis::generate_state(state);
                db.write().await.update(|t| {
                    // Persist genesis block
                    t.store_block(genesis_blk.header(), &[], Label::Final)
                })?;

                BlockWithLabel::new_with_label(genesis_blk, Label::Final)
            }
        };

        let block_header = block.inner().header();

        tracing::info!(
            event = "Ledger block loaded",
            height = block_header.height,
            hash = hex::encode(block_header.hash),
            state_root = hex::encode(block_header.state_hash),
            label = ?block.label()
        );

        Ok(block)
    }

    fn next_timeout() -> Instant {
        Instant::now()
            .checked_add(ACCEPT_BLOCK_TIMEOUT_SEC)
            .unwrap()
    }
}
