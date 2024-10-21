// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod outofsync;

use outofsync::OutOfSyncImpl;

use super::acceptor::{Acceptor, RevertTarget};
use super::stall_chain_fsm::{self, StalledChainFSM};
use crate::chain::fallback;
use crate::database;
use crate::{vm, Network};

use crate::database::{ConsensusStorage, Ledger};
use metrics::counter;
use node_data::ledger::{to_str, Attestation, Block};
use node_data::message::payload::{
    GetResource, Inv, Quorum, RatificationResult, Vote,
};

// use node_data::message::{payload, Message, Metadata, WireMessage};
use node_data::message::{Message, Metadata};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::Instant;
use tracing::{debug, error, info, warn};

const DEFAULT_ATT_CACHE_EXPIRY: Duration = Duration::from_secs(60);

/// Maximum number of hops between the requester and the node that contains the
/// requested resource
const DEFAULT_HOPS_LIMIT: u16 = 16;

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

    /// Attestations cached from received Quorum messages
    attestations_cache: HashMap<[u8; 32], (Attestation, Instant)>,

    /// State machine to detect a stalled state of the chain
    stalled_sm: StalledChainFSM<DB, N, VM>,
}

impl<N: Network, DB: database::DB, VM: vm::VMExecution> SimpleFSM<N, DB, VM> {
    pub async fn new(
        acc: Arc<RwLock<Acceptor<N, DB, VM>>>,
        network: Arc<RwLock<N>>,
    ) -> Self {
        let blacklisted_blocks = Arc::new(RwLock::new(HashSet::new()));
        let stalled_sm = StalledChainFSM::new_with_acc(acc.clone()).await;
        let curr = State::InSync(InSyncImpl::<DB, VM, N>::new(
            acc.clone(),
            network.clone(),
            blacklisted_blocks.clone(),
        ));

        Self {
            curr,
            acc,
            network: network.clone(),
            blacklisted_blocks,
            attestations_cache: Default::default(),
            stalled_sm,
        }
    }

    pub async fn on_failed_consensus(&mut self) {
        self.acc.write().await.restart_consensus().await;
    }

    /// Handles an event of a block occurrence.
    ///
    /// A block event could originate from either local consensus execution, a
    /// wire Block message (topics::Block), or a wire Quorum message
    /// (topics::Quorum).
    ///
    /// If the block is accepted, it returns the block itself
    pub async fn on_block_event(
        &mut self,
        blk: Block,
        metadata: Option<Metadata>,
    ) -> anyhow::Result<Option<Block>> {
        let block_hash = &blk.header().hash;

        // Filter out blocks that have already been marked as
        // blacklisted upon successful fallback execution.
        if self.blacklisted_blocks.read().await.contains(block_hash) {
            info!(
                event = "block discarded",
                reason = "blacklisted",
                hash = to_str(&blk.header().hash),
                height = blk.header().height,
                iter = blk.header().iteration,
            );
            // block discarded, should we clean up attestation cache (if any)?
            return Ok(None);
        }

        let blk = self.attach_att_if_needed(blk);
        if let Some(blk) = blk.as_ref() {
            let fsm_res = match &mut self.curr {
                State::InSync(ref mut curr) => {
                    if let Some((target_block, peer_addr)) =
                        curr.on_block_event(blk, metadata).await?
                    {
                        // Transition from InSync to OutOfSync state
                        curr.on_exiting().await;

                        // Enter new state
                        let mut next = OutOfSyncImpl::new(
                            self.acc.clone(),
                            self.network.clone(),
                        )
                        .await;
                        next.on_entering(target_block, peer_addr).await;
                        self.curr = State::OutOfSync(next);
                    }
                    anyhow::Ok(())
                }
                State::OutOfSync(ref mut curr) => {
                    if curr.on_block_event(blk, metadata).await? {
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
                    anyhow::Ok(())
                }
            };

            // Try to detect a stalled chain
            // Generally speaking, if a node is receiving future blocks from the
            // network but it cannot accept a new block for long time, then
            // it might be a sign of a getting stalled on non-main branch.

            let res = self.stalled_sm.on_block_received(blk).await.clone();
            match res {
                stall_chain_fsm::State::StalledOnFork(
                    local_hash_at_fork,
                    remote_blk,
                ) => {
                    info!(
                        event = "stalled on fork",
                        local_hash = to_str(&local_hash_at_fork),
                        remote_hash = to_str(&remote_blk.header().hash),
                        remote_height = remote_blk.header().height,
                    );
                    let mut acc = self.acc.write().await;

                    let prev_local_state_root =
                        acc.db.read().await.view(|t| {
                            let local_blk = t
                                .fetch_block_header(&local_hash_at_fork)?
                                .expect("local hash should exist");

                            let prev_blk = t
                                .fetch_block_header(&local_blk.prev_block_hash)?
                                .expect("prev block hash should exist");

                            anyhow::Ok(prev_blk.state_hash)
                        })?;

                    match acc
                        .try_revert(RevertTarget::Commit(prev_local_state_root))
                        .await
                    {
                        Ok(_) => {
                            counter!("dusk_revert_count").increment(1);
                            info!(event = "reverted to last finalized");

                            info!(
                                event = "recovery block",
                                height = remote_blk.header().height,
                                hash = to_str(&remote_blk.header().hash),
                            );

                            acc.try_accept_block(&remote_blk, true).await?;

                            // Black list the block hash to avoid accepting it
                            // again due to fallback execution
                            self.blacklisted_blocks
                                .write()
                                .await
                                .insert(local_hash_at_fork);

                            // Try to reset the stalled chain FSM to `running`
                            // state
                            if let Err(err) =
                                self.stalled_sm.reset(remote_blk.header())
                            {
                                info!(
                                    event = "revert failed",
                                    err = format!("{:?}", err)
                                );
                            }
                        }
                        Err(e) => {
                            error!(
                                event = "revert failed",
                                err = format!("{:?}", e)
                            );
                            return Ok(None);
                        }
                    }
                }
                stall_chain_fsm::State::Stalled(_) => {
                    self.blacklisted_blocks.write().await.clear();
                }
                _ => {}
            }

            // Ensure that an error in FSM does not affect the stalled_sm
            fsm_res?;
        }

        Ok(blk)
    }

    async fn flood_request_block(&mut self, hash: [u8; 32], att: Attestation) {
        if self.attestations_cache.contains_key(&hash) {
            return;
        }

        // Save attestation in case only candidate block is received
        let expiry = Instant::now()
            .checked_add(DEFAULT_ATT_CACHE_EXPIRY)
            .unwrap();
        self.attestations_cache.insert(hash, (att, expiry));

        let mut inv = Inv::new(1);
        inv.add_candidate_from_hash(hash);

        flood_request(&self.network, &inv).await;
    }

    /// Handles a Success Quorum message that is received from either the
    /// network or internal consensus execution.
    ///
    /// If the corresponding Candidate is in the DB, we attach the Attestation
    /// and handle the new block; otherwise, we request the Candidate from the
    /// network
    pub(crate) async fn on_success_quorum(
        &mut self,
        qmsg: &Quorum,
        metadata: Option<Metadata>,
    ) {
        // Clean up attestation cache
        self.clean_att_cache();

        if let RatificationResult::Success(Vote::Valid(candidate)) =
            qmsg.att.result
        {
            let db = self.acc.read().await.db.clone();
            let tip_header = self.acc.read().await.tip_header().await;
            let tip_height = tip_header.height;
            let quorum_height = qmsg.header.round;

            // Check if we already accepted this block
            if let Ok(blk_exists) =
                db.read().await.view(|t| t.get_block_exists(&candidate))
            {
                if blk_exists {
                    warn!("skipping Quorum for known block");
                    return;
                }
            };

            let quorum_blk = if quorum_height > tip_height + 1 {
                // Quorum from future

                // We do not check the db because we currently do not store
                // candidates from the future
                None
            } else if (quorum_height == tip_height + 1)
                || (quorum_height == tip_height && tip_header.hash != candidate)
            {
                // If Quorum is for at height tip+1 or tip (but for a different
                // candidate) we try to fetch the candidate from the DB
                let res = db
                    .read()
                    .await
                    .view(|t| t.fetch_candidate_block(&candidate));

                match res {
                    Ok(b) => b,
                    Err(_) => None,
                }
            } else {
                // INFO: we currently ignore Quorum messages from the past
                None
            };

            let attestation = qmsg.att;

            if let Some(mut blk) = quorum_blk {
                // Candidate found. We can build the "full" block
                info!(
                    event = "New block from Quorum",
                    blk_height = blk.header().height,
                    blk_hash = to_str(&blk.header().hash),
                    is_local = metadata.is_none(),
                );

                // Attach the Attestation to the block
                blk.set_attestation(attestation);

                // Handle the new block
                let res = self.on_block_event(blk, metadata).await;
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error on block handling: {e}");
                    }
                }
            } else {
                // Candidate block not found
                debug!(
                    event = "Candidate not found. Requesting it to the network",
                    hash = to_str(&candidate),
                    height = quorum_height,
                );

                // Cache the attestation and request the candidate from the
                // network.
                self.flood_request_block(candidate, attestation).await;
            }
        } else {
            error!("Invalid Quorum message");
        }
    }

    pub(crate) async fn on_heartbeat_event(&mut self) -> anyhow::Result<()> {
        self.stalled_sm.on_heartbeat_event().await;

        match &mut self.curr {
            State::InSync(ref mut curr) => {
                if curr.on_heartbeat().await? {
                    // Transition from InSync to OutOfSync state
                    curr.on_exiting().await;

                    // Enter new state
                    let next = OutOfSyncImpl::new(
                        self.acc.clone(),
                        self.network.clone(),
                    )
                    .await;
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

    /// Try to attach the attestation to a block that misses it
    ///
    /// Return None if it's not able to attach the attestation
    fn attach_att_if_needed(&mut self, mut blk: Block) -> Option<Block> {
        let block_hash = blk.header().hash;

        let block_with_att = if blk.header().att == Attestation::default() {
            // The default att means the block was retrieved from Candidate
            // CF thus missing the attestation. If so, we try to set the valid
            // attestation from the cache attestations.
            if let Some((att, _)) =
                self.attestations_cache.get(&blk.header().hash)
            {
                blk.set_attestation(*att);
                Some(blk)
            } else {
                error!("att not found for {}", hex::encode(blk.header().hash));
                None
            }
        } else {
            Some(blk)
        };

        // Clean up attestation cache
        self.clean_att_cache();
        self.attestations_cache.remove(&block_hash);

        block_with_att
    }

    fn clean_att_cache(&mut self) {
        let now = Instant::now();
        self.attestations_cache
            .retain(|_, (_, expiry)| *expiry > now);
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

    /// Return Some if there is the need to switch to OutOfSync mode.
    /// This way the sync-up procedure to download all missing blocks from the
    /// main chain will be triggered
    async fn on_block_event(
        &mut self,
        remote_blk: &Block,
        metadata: Option<Metadata>,
    ) -> anyhow::Result<Option<(Block, SocketAddr)>> {
        let mut acc = self.acc.write().await;
        let tip_header = acc.tip_header().await;
        let remote_header = remote_blk.header();
        let remote_height = remote_header.height;

        // If we already accepted a block with the same height as remote_blk,
        // check if remote_blk has higher priority. If so, we revert to its
        // prev_block, and accept it as the new tip
        if remote_height <= tip_header.height {
            // Ensure the block is different from what we have in our chain
            if remote_height == tip_header.height {
                if remote_header.hash == tip_header.hash {
                    return Ok(None);
                }
            } else {
                let blk_exists = acc
                    .db
                    .read()
                    .await
                    .view(|t| t.get_block_exists(&remote_header.hash))?;

                if blk_exists {
                    return Ok(None);
                }
            }

            // Ensure remote_blk is higher than the last finalized
            // We do this check after the previous one because
            // get_last_final_block if heavy
            if remote_height
                <= acc.get_last_final_block().await?.header().height
            {
                return Ok(None);
            }

            // Check if prev_blk is in our chain
            // If not, remote_blk is on a fork
            let prev_blk_exists =
                acc.db.read().await.view(|t| {
                    t.get_block_exists(&remote_header.prev_block_hash)
                })?;

            if !prev_blk_exists {
                warn!(
                    "received block from fork at height {remote_height}: {}",
                    to_str(&remote_header.hash)
                );
                return Ok(None);
            }

            // Fetch the chain block at the same height as remote_blk
            let local_blk = if remote_height == tip_header.height {
                acc.tip.read().await.inner().clone()
            } else {
                acc.db
                    .read()
                    .await
                    .view(|t| t.fetch_block_by_height(remote_height))?
                    .expect("local block should exist")
            };
            let local_header = local_blk.header();
            let local_height = local_header.height;

            match remote_header.iteration.cmp(&local_header.iteration) {
                Ordering::Less => {
                    // If remote_blk.iteration < local_blk.iteration, then we
                    // fallback to prev_blk and accept remote_blk
                    info!(
                        event = "entering fallback",
                        height = local_height,
                        iter = local_header.iteration,
                        new_iter = remote_header.iteration,
                    );

                    // Retrieve prev_block state
                    let prev_state = acc
                        .db
                        .read()
                        .await
                        .view(|t| {
                            let res = t
                                .fetch_block_header(
                                    &remote_header.prev_block_hash,
                                )?
                                .map(|prev| prev.state_hash);

                            anyhow::Ok(res)
                        })?
                        .ok_or_else(|| {
                            anyhow::anyhow!("could not retrieve state_hash")
                        })?;

                    match fallback::WithContext::new(acc.deref())
                        .try_revert(
                            local_header,
                            remote_header,
                            RevertTarget::Commit(prev_state),
                        )
                        .await
                    {
                        Ok(_) => {
                            // Successfully fallbacked to prev_blk
                            counter!("dusk_fallback_count").increment(1);

                            // Blacklist the local_blk so we discard it if
                            // we receive it again
                            self.blacklisted_blocks
                                .write()
                                .await
                                .insert(local_header.hash);

                            // After reverting we can accept `remote_blk` as the
                            // new tip
                            acc.try_accept_block(remote_blk, true).await?;
                            return Ok(None);
                        }
                        Err(e) => {
                            error!(
                                event = "fallback failed",
                                height = local_height,
                                remote_height,
                                err = format!("{:?}", e)
                            );
                            return Ok(None);
                        }
                    }
                }

                Ordering::Greater => {
                    // If remote_blk.iteration > local_blk.iteration, we send
                    // the sender our local block. This
                    // behavior is intended to make the peer
                    // switch to our higher-priority block.
                    if let Some(meta) = metadata {
                        let remote_source = meta.src_addr;

                        debug!("sending our lower-iteration block at height {local_height} to {remote_source}");

                        let msg = Message::from(local_blk);
                        let net = self.network.read().await;
                        let send = net.send_to_peer(msg, remote_source);
                        if let Err(e) = send.await {
                            warn!("Unable to send_to_peer {e}")
                        };
                    }
                }
                Ordering::Equal => {
                    // If remote_blk and local_blk have the same iteration, it
                    // means two conflicting candidates have been generated
                    let local_hash = to_str(&local_header.hash);
                    let remote_hash = to_str(&remote_header.hash);
                    warn!("Double candidate detected. Local block: {local_hash}, remote block {remote_hash}");
                }
            }

            return Ok(None);
        }

        // If remote_blk is a successor of our tip, we try to accept it
        if remote_height == tip_header.height + 1 {
            let finalized = acc.try_accept_block(remote_blk, true).await?;

            // On first final block accepted while we're inSync, clear
            // blacklisted blocks
            if finalized {
                self.blacklisted_blocks.write().await.clear();
            }

            // If the accepted block is the one requested to presync peer,
            // switch to OutOfSync/Syncing mode
            if let Some(metadata) = &metadata {
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

            return Ok(None);
        }

        // If remote_blk.height > tip.height+1, we might be out of sync.
        // Before switching to outOfSync mode and download missing blocks,
        // we ensure that the peer has a valid successor of tip
        if let Some(metadata) = &metadata {
            if self.presync.is_none() {
                self.presync = Some(PresyncInfo::new(
                    metadata.src_addr,
                    remote_blk.clone(),
                    tip_header.height,
                ));
            }

            Self::request_block_by_height(
                &self.network,
                tip_header.height + 1,
                metadata.src_addr,
            )
            .await;
        }

        Ok(None)
    }

    /// Requests a block by height from a `peer_addr`
    async fn request_block_by_height(
        network: &Arc<RwLock<N>>,
        height: u64,
        peer_addr: SocketAddr,
    ) {
        let mut inv = Inv::new(1);
        inv.add_block_from_height(height);
        let this_peer = *network.read().await.public_addr();
        let req = GetResource::new(inv, Some(this_peer), u64::MAX, 1);
        debug!(event = "request block by height", ?req, ?peer_addr);

        if let Err(err) = network
            .read()
            .await
            .send_to_peer(req.into(), peer_addr)
            .await
        {
            warn!("could not request block {err}")
        }
    }

    async fn on_heartbeat(&mut self) -> anyhow::Result<bool> {
        if let Some(pre_sync) = &mut self.presync {
            if pre_sync.expiry <= Instant::now() {
                // Reset presync if it timed out
                self.presync = None;
            }
        }

        Ok(false)
    }
}

/// Requests a block by height/hash from the network with so-called
/// Flood-request approach.
async fn flood_request<N: Network>(network: &Arc<RwLock<N>>, inv: &Inv) {
    debug!(event = "flood_request", ?inv);

    if let Err(err) = network
        .read()
        .await
        .flood_request(inv, None, DEFAULT_HOPS_LIMIT)
        .await
    {
        warn!("could not request block {err}")
    };
}
