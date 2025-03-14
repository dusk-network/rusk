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

use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use dusk_consensus::config::is_emergency_block;
use dusk_consensus::errors::ConsensusError;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
pub use header_validation::verify_att;
use node_data::events::Event;
use node_data::ledger::{to_str, BlockWithLabel, Label};
use node_data::message::payload::RatificationResult;
use node_data::message::{AsyncQueue, Payload, Topics};
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;
use tokio::time::{sleep_until, Instant};
use tracing::{debug, error, info, warn};

use self::acceptor::Acceptor;
use self::fsm::SimpleFSM;
#[cfg(feature = "archive")]
use crate::archive::Archive;
use crate::database::rocksdb::MD_HASH_KEY;
use crate::database::{Ledger, Metadata};
use crate::{database, vm, LongLivedService, Message, Network};

const TOPICS: &[u8] = &[
    Topics::Block as u8,
    Topics::Candidate as u8,
    Topics::Validation as u8,
    Topics::Ratification as u8,
    Topics::Quorum as u8,
    Topics::ValidationQuorum as u8,
];

const HEARTBEAT_SEC: Duration = Duration::from_secs(3);

pub struct ChainSrv<N: Network, DB: database::DB, VM: vm::VMExecution> {
    /// Inbound wire messages queue
    inbound: AsyncQueue<Message>,
    keys_path: String,
    acceptor: Option<Arc<RwLock<Acceptor<N, DB, VM>>>>,
    max_consensus_queue_size: usize,
    /// Sender channel for sending out RUES events
    event_sender: Sender<Event>,
    genesis_timestamp: u64,
    dusk_key: BlsPublicKey,
    finality_activation: u64,
    #[cfg(feature = "archive")]
    archive: Archive,
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
        let tip = Self::load_tip(
            db.read().await.deref(),
            vm.read().await.deref(),
            self.genesis_timestamp,
        )
        .await?;

        // Initialize Acceptor
        let acc = Acceptor::init_consensus(
            &self.keys_path,
            tip,
            db,
            network,
            vm,
            #[cfg(feature = "archive")]
            self.archive.clone(),
            self.max_consensus_queue_size,
            self.event_sender.clone(),
            self.dusk_key,
            self.finality_activation,
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

        let mut heartbeat = Instant::now().checked_add(HEARTBEAT_SEC).unwrap();

        // Message loop for Chain context
        loop {
            tokio::select! {
                biased;
                // Receives results from the upper layer
                recv = result_chan.recv() => {
                    match recv? {
                        Err(ConsensusError::Canceled(round)) => {
                            debug!(event = "consensus canceled", round);
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
                recv = self.inbound.recv() => {
                    let msg = recv?;

                    match msg.payload {
                        Payload::Candidate(_)
                        | Payload::Validation(_)
                        | Payload::Ratification(_)
                        | Payload::ValidationQuorum(_) => {
                            self.reroute_acceptor(msg).await;
                        }

                        Payload::Quorum(ref q) => {
                            fsm.on_quorum(q, msg.metadata.as_ref()).await;
                            self.reroute_acceptor(msg).await;

                        }

                        Payload::Block(blk) => {
                            info!(
                                event = "New block",
                                src = "Block msg",
                                height = blk.header().height,
                                iter = blk.header().iteration,
                                hash = to_str(&blk.header().hash),
                                metadata = ?msg.metadata,
                            );

                            // Handle a block that originates from a network peer.
                            // By disabling block broadcast, a block may be received
                            // from a peer only after explicit request (on demand).
                            match fsm.on_block_event(*blk, msg.metadata.clone()).await {
                                Ok(res) => {
                                    if let Some(accepted_blk) = res {
                                        // Repropagate Emergency Blocks
                                        // We already know it's valid because we accepted it
                                        if is_emergency_block(accepted_blk.header().iteration){
                                            // We build a new `msg` to avoid cloning `blk` when
                                            // passing it to `on_block_event`.
                                            // We copy the metadata to keep the original ray_id.
                                            let mut eb_msg = Message::from(accepted_blk);
                                            eb_msg.metadata = msg.metadata;
                                            if let Err(e) = network.read().await.broadcast(&eb_msg).await {
                                                warn!("Unable to re-broadcast Emergency Block: {e}");
                                            }
                                        }
                                    }
                                }
                                Err(err) => {
                                    error!(event = "fsm::on_event failed", src = "wire", err = ?err);
                                }
                            }
                        }

                        _ => {
                            warn!("invalid inbound message");
                        },
                    }

                },
                // Re-routes messages originated from Consensus (upper) layer to the network layer.
                recv = outbound_chan.recv() => {
                    let msg = recv?;

                    // Handle quorum messages from Consensus layer.
                    // If the associated candidate block already exists,
                    // the winner block will be compiled and redirected to the Acceptor.
                    if let Payload::Quorum(quorum) = &msg.payload {
                      if let RatificationResult::Success(_) = quorum.att.result {
                          fsm.on_success_quorum(quorum, msg.metadata.clone()).await;
                      }
                    }

                    if let Payload::GetResource(res) = &msg.payload {
                        if let Err(e) = network.read().await.flood_request(res.get_inv(), None, 16).await {
                            warn!("Unable to re-route message {e}");
                        }
                    } else if let Err(e) = network.read().await.broadcast(&msg).await {
                            warn!("Unable to broadcast message {e}");
                    }

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
        genesis_timestamp: u64,
        dusk_key: BlsPublicKey,
        finality_activation: u64,
        #[cfg(feature = "archive")] archive: Archive,
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
            genesis_timestamp,
            dusk_key,
            finality_activation,
            #[cfg(feature = "archive")]
            archive,
        }
    }

    /// Load both the chain tip and last finalized block from persisted ledger.
    ///
    /// Panics
    ///
    /// If register entry is read but block is not found.
    async fn load_tip(
        db: &DB,
        vm: &VM,
        genesis_timestamp: u64,
    ) -> Result<BlockWithLabel> {
        let stored_block = db.view(|t| {
            anyhow::Ok(t.op_read(MD_HASH_KEY)?.and_then(|tip_hash| {
                t.block(&tip_hash[..])
                    .expect("block to be found if metadata is set")
            }))
        })?;

        let block = match stored_block {
            Some(blk) => {
                let (_, label) = db
                    .view(|t| t.block_label_by_height(blk.header().height))?
                    .unwrap();

                BlockWithLabel::new_with_label(blk, label)
            }
            None => {
                // Lack of register record means the loaded database is
                // either malformed or empty.
                let state = vm.get_state_root()?;
                let genesis_blk =
                    genesis::generate_block(state, genesis_timestamp);
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

    pub async fn revert_last_final(&self) -> anyhow::Result<()> {
        self.acceptor
            .as_ref()
            .expect("Chain to be initialized")
            .read()
            .await
            .try_revert(acceptor::RevertTarget::LastFinalizedState)
            .await
    }

    async fn reroute_acceptor(&self, msg: Message) {
        debug!(
            event = "Consensus message received",
            topic = ?msg.topic(),
            info = ?msg.header,
            metadata = ?msg.metadata,
        );

        // Re-route message to the Consensus
        let acc = self.acceptor.as_ref().expect("initialize is called");
        if let Err(e) = acc.read().await.reroute_msg(msg).await {
            warn!("Could not reroute msg to Consensus: {}", e);
        }
    }
}
