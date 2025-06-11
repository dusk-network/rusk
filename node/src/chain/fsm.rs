// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod insync;
mod outofsync;
mod stalled;

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use dusk_consensus::config::is_emergency_block;
use metrics::counter;
use node_data::ledger::{to_str, Attestation, Block};
use node_data::message::payload::{Inv, Quorum, RatificationResult, Vote};
use node_data::message::Metadata;
use tokio::sync::RwLock;
use tokio::time::Instant;
use tracing::{debug, error, info, trace, warn};

use self::insync::InSyncImpl;
use self::outofsync::OutOfSyncImpl;
use self::stalled::StalledChainFSM;
use super::acceptor::{Acceptor, RevertTarget};
use crate::database::{ConsensusStorage, Ledger};
use crate::{database, vm, Network};

use anyhow::{anyhow, Result};

const DEFAULT_ATT_CACHE_EXPIRY: Duration = Duration::from_secs(60);

/// Maximum number of hops between the requester and the node that contains the
/// requested resource
const DEFAULT_HOPS_LIMIT: u16 = 16;

type SharedHashSet = Arc<RwLock<HashSet<[u8; 32]>>>;

/// `PresyncInfo` holds information about the presync process, which is used to
/// verify if a peer has valid block successors before switching the system into
/// out-of-sync mode.
/// This struct helps safeguard against syncing with malicious peers by tracking
/// and validating block continuity from a specific peer within a given
/// timeframe.
#[derive(Clone)]
struct PresyncInfo {
    // The address of the peer that we're syncing with. This helps to identify
    // which peer the presync information is associated with.
    peer_addr: SocketAddr,

    // The current tip height of our local blockchain. This is the height
    // of the highest block we know of before starting the presync process.
    tip_height: u64,

    // The remote height provided by the peer, which indicates the height of
    // the last block the peer knows of. This is used to compare and determine
    // whether the peer is ahead of us and if we're out of sync.
    remote_height: u64,

    // A timestamp indicating when the presync process should expire. If the
    // peer doesn't provide valid blocks by this time, the presync is
    // considered failed.
    expiry: Instant,

    // A pool of blocks that are collected from the peer during the presync
    // process. These blocks will be validated to ensure that the peer has
    // valid successors for the current tip.
    pool: Vec<Block>,
}

impl PresyncInfo {
    const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

    fn from_block(
        peer_addr: SocketAddr,
        remote_block: Block,
        tip_height: u64,
    ) -> Self {
        let remote_height = remote_block.header().height;
        let mut info = Self::from_height(peer_addr, remote_height, tip_height);
        info.pool.push(remote_block);
        info
    }

    fn from_height(
        peer_addr: SocketAddr,
        remote_height: u64,
        tip_height: u64,
    ) -> Self {
        Self {
            peer_addr,
            remote_height,
            expiry: Instant::now().checked_add(Self::DEFAULT_TIMEOUT).unwrap(),
            tip_height,
            pool: vec![],
        }
    }

    fn start_height(&self) -> u64 {
        self.tip_height
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

    pub async fn on_quorum(
        &mut self,
        quorum: &Quorum,
        metadata: Option<&Metadata>,
    ) {
        match &mut self.curr {
            State::OutOfSync(oos) => oos.on_quorum(quorum).await,
            State::InSync(is) => is.on_quorum(quorum, metadata).await,
        }
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
        mut blk: Block,
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

        // Try attach the Attestation, if necessary
        // If can't find the Attestation, the block is discarded
        // unless it's an Emergency Blocks, which have no Attestation
        if !Self::is_block_attested(&blk)
            && !is_emergency_block(blk.header().iteration)
        {
            if let Err(err) = self.attach_blk_att(&mut blk) {
                warn!(event = "block discarded", ?err);
                return Ok(None);
            }
        }

        let fsm_res = match &mut self.curr {
            State::InSync(ref mut curr) => {
                if let Some(presync) =
                    curr.on_block_event(&blk, metadata).await?
                {
                    // Transition from InSync to OutOfSync state
                    curr.on_exiting().await;

                    // Enter new state
                    let mut next = OutOfSyncImpl::new(
                        self.acc.clone(),
                        self.network.clone(),
                    )
                    .await;
                    next.on_entering(presync).await;
                    self.curr = State::OutOfSync(next);
                }
                anyhow::Ok(())
            }
            State::OutOfSync(ref mut curr) => {
                if curr.on_block_event(&blk).await? {
                    // Transition from OutOfSync to InSync state
                    curr.on_exiting().await;

                    // Enter new state
                    let mut next = InSyncImpl::new(
                        self.acc.clone(),
                        self.network.clone(),
                        self.blacklisted_blocks.clone(),
                    );
                    next.on_entering(&blk).await.map_err(|e| {
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

        let res = self.stalled_sm.on_block_received(&blk).await.clone();
        match res {
            stalled::State::StalledOnFork(local_hash_at_fork, remote_blk) => {
                info!(
                    event = "stalled on fork",
                    local_hash = to_str(&local_hash_at_fork),
                    remote_hash = to_str(&remote_blk.header().hash),
                    remote_height = remote_blk.header().height,
                );
                let mut acc = self.acc.write().await;

                let prev_local_state_root = acc.db.read().await.view(|t| {
                    let local_blk = t
                        .block_header(&local_hash_at_fork)?
                        .expect("local hash should exist");

                    let prev_blk = t
                        .block_header(&local_blk.prev_block_hash)?
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

                        acc.accept_block(&remote_blk, true).await?;

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
                                err = format!("{err:?}")
                            );
                        }
                    }
                    Err(e) => {
                        error!(event = "revert failed", err = format!("{e:?}"));
                        return Ok(None);
                    }
                }
            }
            stalled::State::Stalled(_) => {
                self.blacklisted_blocks.write().await.clear();
            }
            _ => {}
        }

        // Ensure that an error in FSM does not affect the stalled_sm
        fsm_res?;

        Ok(Some(blk))
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
                db.read().await.view(|t| t.block_exists(&candidate))
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
                let res = db.read().await.view(|t| t.candidate(&candidate));

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
                    event = "New block",
                    src = "Quorum msg",
                    height = blk.header().height,
                    iter = blk.header().iteration,
                    hash = to_str(&blk.header().hash)
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

    // Checks if a block has an Attestation
    fn is_block_attested(blk: &Block) -> bool {
        blk.header().att != Attestation::default()
    }

    /// Try to attach the attestation to a block that misses it
    ///
    /// Return Err if can't find the Attestation in the cache
    fn attach_blk_att(&mut self, blk: &mut Block) -> Result<()> {
        let block_hash = blk.header().hash;

        // Check if we have the block Attestation in our cache
        if let Some((att, _)) = self.attestations_cache.get(&block_hash) {
            blk.set_attestation(*att);
        } else {
            // warn!("Attestation not found for {}", hex::encode(block_hash));
            return Err(anyhow!(
                "Attestation not found for {}",
                hex::encode(block_hash)
            ));
        }

        // Clean up attestation cache
        self.clean_att_cache();
        self.attestations_cache.remove(&block_hash);

        Ok(())
    }

    fn clean_att_cache(&mut self) {
        let now = Instant::now();
        self.attestations_cache
            .retain(|_, (_, expiry)| *expiry > now);
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
