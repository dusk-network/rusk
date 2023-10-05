// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::{acceptor::Acceptor, consensus, genesis};
use crate::chain::fallback;
use crate::database::{self, Ledger};
use crate::{vm, Network};
use dusk_consensus::user::provisioners::{self, Provisioners};
use node_data::ledger::{self, to_str, Block, Hash, Transaction};
use node_data::message::payload::GetBlocks;
use node_data::message::Message;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::time::{Duration, UNIX_EPOCH};
use std::{sync::Arc, time::SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, error, info};

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

    pub async fn on_idle(&mut self, timeout_sec: u8) -> anyhow::Result<()> {
        let acc = self.acc.read().await;
        let mrb_height = acc.get_curr_height().await;
        let iter = acc.get_curr_iteration().await;
        let last_finalized = acc.get_finalized().await;

        info!(
            event = "fsm::idle",
            mrb_height,
            iter,
            timeout_sec,
            "finalized_height" = last_finalized.header().height,
        );

        // Request updates for any block after my last finalized state
        self.network
            .write()
            .await
            .send_to_alive_peers(
                &Message::new_get_blocks(GetBlocks {
                    locator: last_finalized.header().hash,
                }),
                REDUNDANCY_PEER_FACTOR,
            )
            .await;

        Ok(())
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
            return Ok(());
        }

        match &mut self.curr {
            State::InSync(ref mut curr) => {
                if curr.on_event(blk, msg).await? {
                    /// Transition from InSync to OutOfSync state
                    curr.on_exiting();

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
                    /// Transition from OutOfSync to InSync state
                    curr.on_exiting();

                    // Enter new state
                    let mut next = InSyncImpl::new(
                        self.acc.clone(),
                        self.network.clone(),
                        self.blacklisted_blocks.clone(),
                    );
                    next.on_entering(blk).await;
                    self.curr = State::InSync(next);
                }
            }
        }
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
        let acc = self.acc.write().await;
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
        let acc = self.acc.write().await;
        let h = blk.header().height;
        let curr_h = acc.get_curr_height().await;
        let iter = acc.get_curr_iteration().await;
        let curr_hash = acc.get_curr_hash().await;

        info!(
            event = "fsm::on_event",
            height = curr_h,
            iter,
            blk_height = h,
            blk_iter = blk.header().iteration,
            blk_hash = to_str(&blk.header().hash),
        );

        if h < curr_h {
            return Ok(false);
        }

        if h == curr_h {
            if blk.header().hash == curr_hash {
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

                    self.network.write().await.broadcast(msg).await;

                    // By switching to OutOfSync mode, we trigger the
                    // sync-up procedure to download all missing blocks from the
                    // correct chain.
                    return Ok(msg.metadata.is_some());
                }
            }
        }

        // Try accepting consecutive block
        if h == curr_h + 1 {
            acc.try_accept_block(blk, true).await?;

            // On first finalized block accepted while we're inSync, clear
            // blacklisted blocks
            if blk.header().iteration == 1 {
                self.blacklisted_blocks.write().await.clear();
            }

            // When accepting block from the wire in inSync state, we
            // rebroadcast it
            self.network.write().await.broadcast(msg).await;

            return Ok(false);
        }

        // Transition to OutOfSync
        Ok(self.allow_transition(msg).is_ok())
    }

    fn allow_transition(&self, msg: &Message) -> anyhow::Result<()> {
        // TODO: Consider verifying certificate here
        Ok(())
    }
}

struct OutOfSyncImpl<DB: database::DB, VM: vm::VMExecution, N: Network> {
    range: (u64, u64),
    start_time: SystemTime,
    pool: HashMap<u64, Block>,

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
        }
    }

    /// performed when entering the OutOfSync state
    async fn on_entering(&mut self, blk: &Block, msg: &Message) {
        let (curr_height, locator) = {
            let acc = self.acc.read().await;
            (acc.get_curr_height().await, acc.get_curr_hash().await)
        };

        let dest_addr = msg.metadata.as_ref();

        self.range = (
            curr_height,
            std::cmp::min(
                curr_height + MAX_BLOCKS_TO_REQUEST as u64,
                blk.header().height,
            ),
        );

        // Request missing blocks from source peer
        let gb_msg = Message::new_get_blocks(GetBlocks { locator });

        if let Some(peer) = dest_addr {
            self.network
                .write()
                .await
                .send_to_peer(&gb_msg, peer.src_addr)
                .await;
        } else {
            // In case we reverted multiple blocks after fallback execution
            // triggered by a block from local consensus, there is no fixed peer
            // to sync with.
            self.network
                .write()
                .await
                .send_to_alive_peers(&gb_msg, REDUNDANCY_PEER_FACTOR)
                .await;
        }

        // add to the pool
        let h = blk.header().height;
        self.pool.insert(h, blk.clone());

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
        let acc = self.acc.write().await;
        let h = blk.header().height;

        if self
            .start_time
            .checked_add(Duration::from_millis(EXPIRY_TIMEOUT_MILLIS as u64))
            .unwrap()
            <= SystemTime::now()
        {
            // Timeout-ed sync-up
            // Transit back to InSync mode
            return Ok(true);
        }

        if h <= acc.get_curr_height().await {
            return Ok(false);
        }

        // Try accepting consecutive block
        if h == acc.get_curr_height().await + 1 {
            acc.try_accept_block(blk, (h == self.range.1)).await?;

            self.start_time = SystemTime::now();

            // Try to accept other consecutive blocks from the pool, if
            // available
            for height in (h + 1)..(self.range.1 + 1) {
                let enable_consensus = (height == self.range.1);

                if let Some(blk) = self.pool.get(&height) {
                    acc.try_accept_block(blk, enable_consensus).await?;
                } else {
                    break;
                }
            }

            // Check target height is reached
            if acc.get_curr_height().await == self.range.1 {
                // Block sync-up procedure manages to download all requested
                // blocks

                // Transit to InSync mode
                return Ok(true);
            }

            return Ok(false);
        }

        // add block to the pool
        let key = blk.header().height;
        self.pool.insert(key, blk.clone());

        Ok(false)
    }
}
