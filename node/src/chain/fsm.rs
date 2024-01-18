// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::acceptor::{Acceptor, RevertTarget};
use crate::chain::fallback;
use crate::database;
use crate::{vm, Network};

use crate::database::Ledger;
use node_data::ledger::{to_str, Block, Label};
use node_data::message::payload::{GetBlocks, Inv};
use node_data::message::Message;
use std::collections::{HashMap, HashSet};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::ops::Deref;
use std::time::Duration;
use std::{sync::Arc, time::SystemTime};
use tokio::sync::RwLock;
use tokio::time::Instant;
use tracing::{error, info, warn};

const MAX_BLOCKS_TO_REQUEST: i16 = 50;
const EXPIRY_TIMEOUT_MILLIS: i16 = 5000;

pub(crate) const REDUNDANCY_PEER_FACTOR: usize = 5;

type SharedHashSet = Arc<RwLock<HashSet<[u8; 32]>>>;

#[derive(Clone)]
struct PresyncInfo {
    peer_addr: SocketAddr,
    start_height: u64,
    target_blk: Block,
    expiry: Instant,
}

impl PresyncInfo {
    const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
    fn new(
        peer_addr: SocketAddr,
        target_blk: Block,
        start_height: u64,
    ) -> Self {
        Self {
            peer_addr,
            target_blk,
            expiry: Instant::now().checked_add(Self::DEFAULT_TIMEOUT).unwrap(),
            start_height,
        }
    }

    fn start_height(&self) -> u64 {
        self.start_height
    }
}

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
                if let Some((b, peer_addr)) = curr.on_event(blk, msg).await? {
                    // Transition from InSync to OutOfSync state
                    curr.on_exiting().await;

                    // Enter new state
                    let mut next = OutOfSyncImpl::new(
                        self.acc.clone(),
                        self.network.clone(),
                    );
                    next.on_entering(&b, peer_addr).await;
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
    presync: Option<PresyncInfo>,
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
            presync: None,
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
        remote_blk: &Block,
        msg: &Message,
    ) -> anyhow::Result<Option<(Block, SocketAddr)>> {
        let mut acc = self.acc.write().await;
        let local_header = acc.tip_header().await;
        let remote_height = remote_blk.header().height;

        if remote_height < local_header.height {
            // Ensure that the block does not exist in the local state
            let exists = acc
                .db
                .read()
                .await
                .view(|t| t.get_block_exists(&remote_blk.header().hash))?;

            if exists {
                // Already exists in local state
                return Ok(None);
            }

            // Ensure that the block height is higher than the last finalized
            // TODO: Retrieve the block from memory
            if remote_height
                <= acc.get_latest_final_block().await?.header().height
            {
                return Ok(None);
            }

            // If our local chain has a block L_B with ConsensusState not Final,
            // and we receive a block R_B such that:
            //
            // R_B.PrevBlock == L_B.PrevBlock
            // R_B.Iteration < L_B.Iteration
            //
            // Then we fallback to N_B.PrevBlock and accept N_B
            let local_header = acc.db.read().await.view(|t| {
                if let Some((prev_header, _)) =
                    t.fetch_block_header(&remote_blk.header().prev_block_hash)?
                {
                    let local_height = prev_header.height + 1;
                    if let Some(l_b) = t.fetch_block_by_height(local_height)? {
                        if remote_blk.header().iteration
                            < l_b.header().iteration
                        {
                            return Ok(Some(l_b.header().clone()));
                        }
                    }
                }

                anyhow::Ok(None)
            })?;

            if let Some(local_header) = local_header {
                match fallback::WithContext::new(acc.deref())
                    .try_revert(
                        &local_header,
                        remote_blk.header(),
                        RevertTarget::LastFinalizedState,
                    )
                    .await
                {
                    Ok(_) => {
                        if remote_height == acc.get_curr_height().await + 1 {
                            acc.try_accept_block(remote_blk, true).await?;
                            return Ok(None);
                        }
                    }
                    Err(e) => {
                        error!(
                            event = "fallback failed",
                            height = local_header.height,
                            remote_height,
                            err = format!("{:?}", e)
                        );
                        return Ok(None);
                    }
                }
            }

            return Ok(None);
        }

        if remote_height == local_header.height {
            if remote_blk.header().hash == local_header.hash {
                // Duplicated block.
                // Node has already accepted it.
                return Ok(None);
            }

            info!(
                event = "entering fallback",
                height = local_header.height,
                iter = local_header.iteration,
                new_iter = remote_blk.header().iteration,
            );

            match fallback::WithContext::new(acc.deref())
                .try_revert(
                    &local_header,
                    remote_blk.header(),
                    RevertTarget::LastFinalizedState,
                )
                .await
            {
                Err(e) => {
                    // Fallback execution has failed. The block is ignored and
                    // Node remains in InSync state.
                    error!(event = "fallback failed", err = format!("{:?}", e));
                    return Ok(None);
                }
                Ok(_) => {
                    // Fallback has completed successfully. Node has managed to
                    // fallback to the most recent finalized block and state.

                    // Blacklist the old-block hash so that if it's again
                    // sent then this node does not try to accept it.
                    self.blacklisted_blocks
                        .write()
                        .await
                        .insert(local_header.hash);

                    if remote_height == acc.get_curr_height().await + 1 {
                        // If we have fallback-ed to previous block only, then
                        // accepting the new block would be enough to continue
                        // in in_Sync mode instead of switching to Out-Of-Sync
                        // mode.

                        acc.try_accept_block(remote_blk, true).await?;
                        return Ok(None);
                    }

                    // By switching to OutOfSync mode, we trigger the
                    // sync-up procedure to download all missing blocks from the
                    // main chain.
                    if let Some(metadata) = &msg.metadata {
                        let res = (remote_blk.clone(), metadata.src_addr);
                        return Ok(Some(res));
                    } else {
                        return Ok(None);
                    };
                }
            }
        }

        // Try accepting consecutive block
        if remote_height == local_header.height + 1 {
            let label = acc.try_accept_block(remote_blk, true).await?;

            // On first final block accepted while we're inSync, clear
            // blacklisted blocks
            if let Label::Final = label {
                self.blacklisted_blocks.write().await.clear();
            }

            // If the accepted block is the one requested to presync peer,
            // switch to OutOfSync/Syncing mode
            if let Some(metadata) = &msg.metadata {
                if let Some(presync) = &mut self.presync {
                    if metadata.src_addr == presync.peer_addr
                        && remote_height == presync.start_height() + 1
                    {
                        let res =
                            (presync.target_blk.clone(), presync.peer_addr);
                        self.presync = None;
                        return Ok(Some(res));
                    }
                }
            }

            // When accepting block from the wire in inSync state, we
            // rebroadcast it
            if let Err(e) = self.network.write().await.broadcast(msg).await {
                warn!("Unable to broadcast accepted block: {e}");
            }

            return Ok(None);
        }

        // Block with height higher than (tip + 1) is received
        // Before switching to outOfSync mode and download missing blocks,
        // ensure that the Peer does know next valid block
        if let Some(metadata) = &msg.metadata {
            if self.presync.is_none() {
                self.presync = Some(PresyncInfo::new(
                    metadata.src_addr,
                    remote_blk.clone(),
                    local_header.height,
                ));
            }

            self.request_block(local_header.height + 1, metadata.src_addr)
                .await;
        }

        Ok(None)
    }

    /// Requests a block by height from a specified peer
    async fn request_block(&self, height: u64, peer_addr: SocketAddr) {
        let mut inv = Inv::default();
        inv.add_block_from_height(height);

        if let Err(err) = self
            .network
            .read()
            .await
            .send_to_peer(&Message::new_get_data(inv), peer_addr)
            .await
        {
            warn!("could not request block {err}")
        };
    }

    async fn on_heartbeat(&mut self) -> anyhow::Result<bool> {
        // TODO: Consider reporting metrics here
        // TODO: Consider handling ACCEPT_BLOCK_TIMEOUT event here

        if let Some(pre_sync) = &mut self.presync {
            if pre_sync.expiry <= Instant::now() {
                // Reset presync if it timed out
                self.presync = None;
            }
        }

        Ok(false)
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
    async fn on_entering(&mut self, blk: &Block, dest_addr: SocketAddr) {
        let (curr_height, locator) = {
            let acc = self.acc.read().await;
            (acc.get_curr_height().await, acc.get_curr_hash().await)
        };

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
