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

use dusk_consensus::commons::{ConsensusError, Database, RoundUpdate};
use dusk_consensus::consensus::Consensus;
use dusk_consensus::contract_state::{
    CallParams, Error, Operations, Output, StateRoot,
};
use dusk_consensus::user::provisioners::Provisioners;
use node_data::ledger::{self, to_str, Block, Hash, Header, Label};

use node_data::message::AsyncQueue;
use node_data::message::{Payload, Topics};
use tokio::sync::RwLock;
use tokio::time::{sleep_until, Instant};

use std::time::Duration;

use self::acceptor::{Acceptor, RevertTarget};
use self::fsm::SimpleFSM;

pub use acceptor::verify_block_cert;

const TOPICS: &[u8] = &[
    Topics::Block as u8,
    Topics::NewBlock as u8,
    Topics::FirstReduction as u8,
    Topics::SecondReduction as u8,
    Topics::Agreement as u8,
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
        let (mrb, last_finalized) =
            Self::load_most_recent_block(db.clone()).await?;

        let provisioners_list = vm.read().await.get_provisioners()?;

        // Initialize Acceptor and trigger consensus task
        let acc = Arc::new(RwLock::new(
            Acceptor::new_with_run(
                &self.keys_path,
                &mrb,
                last_finalized,
                &provisioners_list,
                db.clone(),
                network.clone(),
                vm.clone(),
            )
            .await,
        ));

        // NB. After restart, state_root returned by VM is always the last
        // finalized one.
        let state_root = vm.read().await.get_state_root()?;

        info!(
            event = "VM state loaded",
            state_root = hex::encode(state_root),
        );

        // Detect a consistency issue between VM and Ledger states.
        if mrb.header().height > 0 && mrb.header().state_hash != state_root {
            info!("revert to last finalized state");
            // Revert to last known finalized state.
            acc.read()
                .await
                .try_revert(RevertTarget::LastFinalizedState)
                .await?;
        }

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
                        Payload::NewBlock(_)
                        | Payload::Reduction(_)
                        | Payload::Agreement(_) => {
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
    ) -> Result<(Block, Block)> {
        let mut mrb = Block::default();
        let mut last_finalized = Block::default();

        db.read().await.update(|t| {
            mrb = match t.get_register()? {
                Some(r) => t.fetch_block(&r.mrb_hash)?.unwrap(),
                None => {
                    // Lack of register record means the loaded database is
                    // either malformed or empty.
                    let genesis_blk = genesis::generate_state();

                    /// Persist genesis block
                    t.store_block(genesis_blk.header(), &[], Label::Final)?;
                  
                    genesis_blk
                }
            };

            // Initialize last_finalized block
            if mrb.has_instant_finality() {
                last_finalized = mrb.clone();
            } else {
                // scan
                let mut h = mrb.header().height;
                loop {
                    h -= 1;
                    if let Ok(Some(blk)) = t.fetch_block_by_height(h) {
                        if blk.has_instant_finality() {
                            last_finalized = blk;
                            break;
                        };
                    } else {
                        break;
                    }
                }
            }

            Ok(())
        })?;

        tracing::info!(
            event = "Ledger block loaded",
            height = mrb.header().height,
            finalized_height = last_finalized.header().height,
            hash = hex::encode(mrb.header().hash),
            state_root = hex::encode(mrb.header().state_hash)
        );

        Ok((mrb, last_finalized))
    }

    fn next_timeout() -> Instant {
        Instant::now()
            .checked_add(ACCEPT_BLOCK_TIMEOUT_SEC)
            .unwrap()
    }
}
