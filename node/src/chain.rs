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
mod metrics;
mod stall_chain_fsm;

use self::acceptor::Acceptor;
use self::fsm::SimpleFSM;
use crate::database::rocksdb::MD_HASH_KEY;
use crate::database::{Ledger, Metadata};
use crate::{database, vm, Network};
use crate::{LongLivedService, Message};
use anyhow::Result;
use async_trait::async_trait;
use dusk_consensus::commons::ConsensusError;
pub use header_validation::verify_att;
use node_data::events::Event;
use node_data::ledger::{to_str, BlockWithLabel, Label};
use node_data::message::AsyncQueue;
use node_data::message::{Payload, Topics};
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
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

pub struct ChainSrv<N: Network, DB: database::DB, VM: vm::VMExecution> {
    /// Inbound wire messages queue
    inbound: AsyncQueue<Message>,
    keys_path: String,
    acceptor: Option<Arc<RwLock<Acceptor<N, DB, VM>>>>,
    max_consensus_queue_size: usize,
    event_sender: Sender<Event>,
}

#[async_trait]
impl<N: Network, DB: database::DB, VM: vm::VMExecution>
    LongLivedService<N, DB, VM> for ChainSrv<N, DB, VM>
{
    async fn initialize(
        &mut self,
        network: Arc<RwLock<N>>,
        db: Arc<RwLock<DB>>,
        vm: Arc<RwLock<VM>>,
    ) -> anyhow::Result<()> {
        let tip =
            Self::load_tip(db.read().await.deref(), vm.read().await.deref())
                .await?;

        let state_hash = tip.inner().header().state_hash;
        let provisioners_list = vm.read().await.get_provisioners(state_hash)?;

        // Initialize Acceptor
        let acc = Acceptor::init_consensus(
            &self.keys_path,
            tip,
            provisioners_list,
            db,
            network,
            vm,
            self.max_consensus_queue_size,
            self.event_sender.clone(),
        )
        .await?;

        self.acceptor = Some(Arc::new(RwLock::new(acc)));

        Ok(())
    }

    async fn execute(
        &mut self,
        network: Arc<RwLock<N>>,
        _db: Arc<RwLock<DB>>,
        _vm: Arc<RwLock<VM>>,
    ) -> anyhow::Result<usize> {
        // Register routes
        LongLivedService::<N, DB, VM>::add_routes(
            self,
            TOPICS,
            self.inbound.clone(),
            &network,
        )
        .await?;

        let acc = self.acceptor.as_mut().expect("initialize is called");
        acc.write().await.spawn_task().await;

        // Start-up FSM instance
        let mut fsm = SimpleFSM::new(acc.clone(), network.clone()).await;

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
                recv = result_chan.recv() => {
                    match recv? {
                        Err(ConsensusError::Canceled(round)) => {
                            info!(event = "consensus canceled", round);
                        }
                        Err(err) => {
                            // Internal consensus execution has terminated with an error
                            error!(event = "failed_consensus", ?err);
                            fsm.on_failed_consensus().await;
                        }
                        _ => {}
                    }
                },
                // Handles any inbound wire.
                // Component should either process it or re-route it to the next upper layer
                recv =  self.inbound.recv() => {
                    let msg = recv?;
                    match msg.payload {
                        Payload::Block(blk) => {
                           info!(
                                event = "block received",
                                src = "wire",
                                blk_height = blk.header().height,
                                blk_hash = to_str(&blk.header().hash),
                                metadata = ?msg.metadata,
                            );

                            // Handle a block that originates from a network peer.
                            // By disabling block broadcast, a block may be received from a peer
                            // only after explicit request (on demand).
                            match fsm.on_block_event(*blk, msg.metadata).await {
                                Ok(None) => {}
                                Ok(Some(_)) => {
                                    timeout = Self::next_timeout();
                                }
                                Err(err) => {
                                    error!(event = "fsm::on_event failed", src = "wire", err = ?err);
                                }
                            }
                        }

                        // Re-route message to the acceptor
                        Payload::Candidate(_)
                        | Payload::Validation(_)
                        | Payload::Ratification(_) => {
                            let acc = self.acceptor.as_ref().expect("initialize is called");
                            if let Err(e) = acc.read().await.reroute_msg(msg).await {
                                warn!("msg discarded: {e}");
                            }
                        },
                        Payload::Quorum(ref payload) => {
                            let acc = self.acceptor.as_ref().expect("initialize is called");
                            if let Err(e) = acc.read().await.reroute_msg(msg.clone()).await {
                                warn!("msg discarded: {e}");
                            }
                            match fsm.on_quorum_msg(payload, &msg).await {
                                Ok(None) => {}
                                Ok(Some(_)) => {
                                    // block accepted, timeout reset
                                    timeout = Self::next_timeout();
                                }
                                Err(err) => {
                                    warn!(event = "quorum msg", ?err);
                                }
                            };
                        }
                        _ => warn!("invalid inbound message"),
                    }
                },
                // Re-routes messages originated from Consensus (upper) layer to the network layer.
                recv = outbound_chan.recv() => {
                    let msg = recv?;

                    // Handle quorum messages from Consensus layer.
                    // If the associated candidate block already exists,
                    // the winner block will be compiled and redirected to the Acceptor.
                    if let Payload::Quorum(quorum) = &msg.payload {
                        match fsm.on_quorum_msg(quorum, &msg).await {
                            Ok(None) => {}
                            Ok(Some(_)) => {
                                // block accepted, timeout reset
                                timeout = Self::next_timeout();
                            }
                            Err(err) => {
                                warn!(event = "handle quorum msg from internal consensus failed", ?err);
                            }
                        };
                    }

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

impl<N: Network, DB: database::DB, VM: vm::VMExecution> ChainSrv<N, DB, VM> {
    pub fn new(
        keys_path: String,
        max_inbound_size: usize,
        event_sender: Sender<Event>,
    ) -> Self {
        info!(
            "ChainSrv::new with keys_path: {}, max_inbound_size: {}",
            keys_path, max_inbound_size
        );

        Self {
            inbound: AsyncQueue::bounded(max_inbound_size, "chain_inbound"),
            keys_path,
            acceptor: None,
            max_consensus_queue_size: max_inbound_size,
            event_sender,
        }
    }

    /// Load both the chain tip and last finalized block from persisted ledger.
    ///
    /// Panics
    ///
    /// If register entry is read but block is not found.
    async fn load_tip(db: &DB, vm: &VM) -> Result<BlockWithLabel> {
        let stored_block = db.view(|t| {
            anyhow::Ok(t.op_read(MD_HASH_KEY)?.and_then(|tip_hash| {
                t.fetch_block(&tip_hash[..])
                    .expect("block to be found if metadata is set")
            }))
        })?;

        let block = match stored_block {
            Some(blk) => {
                let (_, label) = db
                    .view(|t| {
                        t.fetch_block_label_by_height(blk.header().height)
                    })?
                    .unwrap();

                BlockWithLabel::new_with_label(blk, label)
            }
            None => {
                // Lack of register record means the loaded database is
                // either malformed or empty.
                let state = vm.get_state_root()?;
                let genesis_blk = genesis::generate_state(state);
                db.update(|t| {
                    // Persist genesis block
                    t.store_block(
                        genesis_blk.header(),
                        &[],
                        &[],
                        Label::Final(0),
                    )
                })?;

                BlockWithLabel::new_with_label(genesis_blk, Label::Final(0))
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
