// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::acceptor::Acceptor;
use crate::chain::fallback;
use crate::database;
use crate::{vm, Network};

use node_data::ledger::{to_str, Block, Label};
use node_data::message::payload::GetBlocks;
use node_data::message::Message;
use std::collections::{HashMap, HashSet};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::ops::Deref;
use std::time::Duration;
use std::{sync::Arc, time::SystemTime};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

const MAX_BLOCKS_TO_REQUEST: i16 = 50;
const EXPIRY_TIMEOUT_MILLIS: i16 = 5000;

pub(crate) const REDUNDANCY_PEER_FACTOR: usize = 5;

type SharedHashSet = Arc<RwLock<HashSet<[u8; 32]>>>;

enum State<N: Network, DB: database::DB, VM: vm::VMExecution> {
    InSync(InSyncImpl<DB, VM, N>),
    OutOfSync(OutOfSyncImpl<DB, VM, N>),
}

/// Implements a finite-state-machine to manage InSync and OutOfSync
pub(crate) struct SimpleFSM<N: Network, DB: database::DB, VM: vm::VMExecution> {
    curr: State<N, DB, VM>,
    acc: Arc<RwLock<Acceptor<N, DB, VM>>>,
    network: Arc<RwLock<N>>,

    blacklisted_blocks: SharedHashSet,
}

impl<N: Network, DB: database::DB, VM: vm::VMExecution> SimpleFSM<N, DB, VM> {
    pub fn new(
        acc: Arc<RwLock<Acceptor<N, DB, VM>>>,
        network: Arc<RwLock<N>>,
    ) -> Self {
        let blacklisted_blocks = Arc::new(RwLock::new(HashSet::new()));

        Self {
            curr: State::InSync(InSyncImpl::<DB, VM, N>::new(
                acc.clone(),
                network.clone(),
                blacklisted_blocks.clone(),
            )),
            acc,
            network,
            blacklisted_blocks,
        }
    }

    pub async fn on_idle(&mut self, timeout: Duration) {
        let acc = self.acc.read().await;
        let height = acc.get_curr_height().await;
        let iter = acc.get_curr_iteration().await;
        if let Ok(last_finalized) = acc.get_latest_final_block().await {
            info!(
                event = "fsm::idle",
                height,
                iter,
                timeout_sec = timeout.as_secs(),
                "finalized_height" = last_finalized.header().height,
            );

            // Clear up all blacklisted blocks
            self.blacklisted_blocks.write().await.clear();

            // Request missing blocks since my last finalized block
            let get_blocks = Message::new_get_blocks(GetBlocks {
                locator: last_finalized.header().hash,
            });
            if let Err(e) = self
                .network
                .read()
                .await
                .send_to_alive_peers(&get_blocks, REDUNDANCY_PEER_FACTOR)
                .await
            {
                warn!("Unable to request GetBlocks {e}");
            }
        } else {
            error!("could not request blocks");
        }
    }

    pub async fn on_failed_consensus(&mut self) {
        self.acc.write().await.restart_consensus().await;
    }

    pub async fn on_event(
        &mut self,
        blk: &Block,
        msg: &Message,
    ) -> anyhow::Result<()> {
        // Filter out blocks that have already been marked as
        // blacklisted upon successful fallback execution.
        if self
            .blacklisted_blocks
            .read()
            .await
            .contains(&blk.header().hash)
        {
            info!(
                event = "block discarded",
                reason = "blacklisted",
                hash = to_str(&blk.header().hash),
                height = blk.header().height,
                iter = blk.header().iteration,
            );
            return Ok(());
        }

        match &mut self.curr {
            State::InSync(ref mut curr) => {
                if curr.on_event(blk, msg).await? {
                    // Transition from InSync to OutOfSync state
                    curr.on_exiting().await;

                    // Enter new state
                    let mut next = OutOfSyncImpl::new(
                        self.acc.clone(),
                        self.network.clone(),
                    );
                    next.on_entering(blk, msg).await;
                    self.curr = State::OutOfSync(next);
                }
            }
            State::OutOfSync(ref mut curr) => {
                if curr.on_event(blk, msg).await? {
                    // Transition from OutOfSync to InSync state
                    curr.on_exiting().await;

                    // Enter new state
                    let mut next = InSyncImpl::new(
                        self.acc.clone(),
                        self.network.clone(),
                        self.blacklisted_blocks.clone(),
                    );
                    next.on_entering(blk).await.map_err(|e| {
                        error!("Unable to enter in_sync state: {e}");
                        e
                    })?;
                    self.curr = State::InSync(next);
                }
            }
        }
        Ok(())
    }

    pub(crate) async fn on_heartbeat_event(&mut self) -> anyhow::Result<()> {
        match &mut self.curr {
            State::InSync(ref mut curr) => {
                if curr.on_heartbeat().await? {
                    // Transition from InSync to OutOfSync state
                    curr.on_exiting().await;

                    // Enter new state
                    let next = OutOfSyncImpl::new(
                        self.acc.clone(),
                        self.network.clone(),
                    );
                    self.curr = State::OutOfSync(next);
                }
            }
            State::OutOfSync(ref mut curr) => {
                if curr.on_heartbeat().await? {
                    // Transition from OutOfSync to InSync state
                    curr.on_exiting().await;

                    // Enter new state
                    let next = InSyncImpl::new(
                        self.acc.clone(),
                        self.network.clone(),
                        self.blacklisted_blocks.clone(),
                    );
                    self.curr = State::InSync(next);
                }
            }
        };

        Ok(())
    }
}

struct InSyncImpl<DB: database::DB, VM: vm::VMExecution, N: Network> {
    acc: Arc<RwLock<Acceptor<N, DB, VM>>>,
    network: Arc<RwLock<N>>,

    blacklisted_blocks: SharedHashSet,
}

impl<DB: database::DB, VM: vm::VMExecution, N: Network> InSyncImpl<DB, VM, N> {
    fn new(
        acc: Arc<RwLock<Acceptor<N, DB, VM>>>,
        network: Arc<RwLock<N>>,
        blacklisted_blocks: SharedHashSet,
    ) -> Self {
        Self {
            acc,
            network,
            blacklisted_blocks,
        }
    }

    /// performed when entering the state
    async fn on_entering(&mut self, blk: &Block) -> anyhow::Result<()> {
        let mut acc = self.acc.write().await;
        let curr_h = acc.get_curr_height().await;

        if blk.header().height == curr_h + 1 {
            acc.try_accept_block(blk, true).await?;
        }

        info!(event = "entering in-sync", height = curr_h);

        Ok(())
    }

    /// performed when exiting the state
    async fn on_exiting(&mut self) {}

    async fn on_event(
        &mut self,
        blk: &Block,
        msg: &Message,
    ) -> anyhow::Result<bool> {
        let mut acc = self.acc.write().await;
        let h = blk.header().height;
        let curr_h = acc.get_curr_height().await;
        let iter = acc.get_curr_iteration().await;
        let curr_hash = acc.get_curr_hash().await;

        if h < curr_h {
            return Ok(false);
        }

        if h == curr_h {
            if blk.header().hash == curr_hash {
                // Duplicated block.
                // Node has already accepted it.
                return Ok(false);
            }

            info!(
                event = "entering fallback",
                height = curr_h,
                iter = iter,
                new_iter = blk.header().iteration,
            );

            match fallback::WithContext::new(acc.deref())
                .try_execute_fallback(blk)
                .await
            {
                Err(e) => {
                    // Fallback execution has failed. The block is ignored and
                    // Node remains in InSync state.
                    error!(event = "fallback failed", err = format!("{:?}", e));
                    return Ok(false);
                }
                Ok(_) => {
                    // Fallback has completed successfully. Node has managed to
                    // fallback to the most recent finalized block and state.

                    // Blacklist the old-block hash so that if it's again
                    // sent then this node does not try to accept it.
                    self.blacklisted_blocks.write().await.insert(curr_hash);

                    if h == acc.get_curr_height().await + 1 {
                        // If we have fallback-ed to previous block only, then
                        // accepting the new block would be enough to continue
                        // in in_Sync mode instead of switching to Out-Of-Sync
                        // mode.

                        acc.try_accept_block(blk, true).await?;
                        return Ok(false);
                    }

                    // By switching to OutOfSync mode, we trigger the
                    // sync-up procedure to download all missing blocks from the
                    // main chain.
                    return Ok(msg.metadata.is_some());
                }
            }
        }

        // Try accepting consecutive block
        if h == curr_h + 1 {
            let label = acc.try_accept_block(blk, true).await?;

            // On first final block accepted while we're inSync, clear
            // blacklisted blocks
            if let Label::Final = label {
                self.blacklisted_blocks.write().await.clear();
            }

            // When accepting block from the wire in inSync state, we
            // rebroadcast it
            if let Err(e) = self.network.write().await.broadcast(msg).await {
                warn!("Unable to broadcast accepted block: {e}");
            }

            return Ok(false);
        }

        // Transition to OutOfSync
        Ok(self.allow_transition(msg).is_ok())
    }

    async fn on_heartbeat(&mut self) -> anyhow::Result<bool> {
        // TODO: Consider reporting metrics here
        // TODO: Consider handling ACCEPT_BLOCK_TIMEOUT event here
        Ok(false)
    }

    fn allow_transition(&self, msg: &Message) -> anyhow::Result<()> {
        let _recv_peer = msg
            .metadata
            .as_ref()
            .map(|m| m.src_addr)
            .ok_or_else(|| anyhow::anyhow!("invalid metadata src_addr"))?;

        // TODO: Consider verifying certificate here

        Ok(())
    }
}

struct OutOfSyncImpl<DB: database::DB, VM: vm::VMExecution, N: Network> {
    range: (u64, u64),
    start_time: SystemTime,
    pool: HashMap<u64, Block>,
    peer_addr: SocketAddr,

    acc: Arc<RwLock<Acceptor<N, DB, VM>>>,
    network: Arc<RwLock<N>>,
}

impl<DB: database::DB, VM: vm::VMExecution, N: Network>
    OutOfSyncImpl<DB, VM, N>
{
    fn new(
        acc: Arc<RwLock<Acceptor<N, DB, VM>>>,
        network: Arc<RwLock<N>>,
    ) -> Self {
        Self {
            start_time: SystemTime::now(),
            range: (0, 0),
            pool: HashMap::new(),
            acc,
            network,
            peer_addr: SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(127, 0, 0, 1),
                8000,
            )),
        }
    }
    /// performed when entering the OutOfSync state
    async fn on_entering(&mut self, blk: &Block, msg: &Message) {
        let (curr_height, locator) = {
            let acc = self.acc.read().await;
            (acc.get_curr_height().await, acc.get_curr_hash().await)
        };

        let dest_addr = msg.metadata.as_ref().unwrap().src_addr;

        self.range = (
            curr_height,
            std::cmp::min(
                curr_height + MAX_BLOCKS_TO_REQUEST as u64,
                blk.header().height,
            ),
        );

        // Request missing blocks from source peer
        let gb_msg = Message::new_get_blocks(GetBlocks { locator });

        if let Err(e) = self
            .network
            .read()
            .await
            .send_to_peer(&gb_msg, dest_addr)
            .await
        {
            warn!("Unable to send GetBlocks: {e}")
        };

        // add to the pool
        let key = blk.header().height;
        self.pool.clear();
        self.pool.insert(key, blk.clone());
        self.peer_addr = dest_addr;

        info!(
            event = "entering out-of-sync",
            from = self.range.0,
            to = self.range.1,
            peer = format!("{:?}", dest_addr),
        );
    }

    /// performed when exiting the state
    async fn on_exiting(&mut self) {
        self.pool.clear();
    }

    pub async fn on_event(
        &mut self,
        blk: &Block,
        msg: &Message,
    ) -> anyhow::Result<bool> {
        let mut acc = self.acc.write().await;
        let h = blk.header().height;

        if self
            .start_time
            .checked_add(Duration::from_millis(EXPIRY_TIMEOUT_MILLIS as u64))
            .unwrap()
            <= SystemTime::now()
        {
            acc.restart_consensus().await;
            // Timeout-ed sync-up
            // Transit back to InSync mode
            return Ok(true);
        }

        if h <= acc.get_curr_height().await {
            return Ok(false);
        }

        // Try accepting consecutive block
        if h == acc.get_curr_height().await + 1 {
            acc.try_accept_block(blk, false).await?;

            if let Some(metadata) = &msg.metadata {
                if metadata.src_addr == self.peer_addr {
                    // reset expiry_time only if we receive a valid block from
                    // the syncing peer.
                    self.start_time = SystemTime::now();
                }
            }

            // Try to accept other consecutive blocks from the pool, if
            // available
            for height in (h + 1)..(self.range.1 + 1) {
                if let Some(blk) = self.pool.get(&height) {
                    acc.try_accept_block(blk, false).await?;
                } else {
                    break;
                }
            }

            // Check target height is reached
            if acc.get_curr_height().await == self.range.1 {
                // Block sync-up procedure manages to download all requested
                // blocks
                acc.restart_consensus().await;

                // Transit to InSync mode
                return Ok(true);
            }

            return Ok(false);
        }

        // add block to the pool
        if self.pool.len() < MAX_BLOCKS_TO_REQUEST as usize {
            let key = blk.header().height;
            self.pool.insert(key, blk.clone());
        }

        error!(event = "block saved", len = self.pool.len());

        Ok(false)
    }

    async fn on_heartbeat(&mut self) -> anyhow::Result<bool> {
        if self
            .start_time
            .checked_add(Duration::from_millis(EXPIRY_TIMEOUT_MILLIS as u64))
            .unwrap()
            <= SystemTime::now()
        {
            // sync-up has timed out, recover consensus task
            self.acc.write().await.restart_consensus().await;

            // Transit back to InSync mode
            return Ok(true);
        }

        Ok(false)
    }
}
