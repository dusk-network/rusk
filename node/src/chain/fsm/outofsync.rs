// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::BTreeMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::ops::Bound::{Excluded, Unbounded};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use dusk_consensus::config::{
    MAX_BLOCK_SIZE, MAX_NUMBER_OF_FAULTS, MAX_NUMBER_OF_TRANSACTIONS,
};
use dusk_consensus::errors::HeaderError;
use dusk_consensus::merkle::merkle_root;
use node_data::get_current_timestamp;
use node_data::ledger::Block;
use node_data::message::Metadata;
use node_data::message::payload::{Candidate, GetResource, Inv, Quorum};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::PresyncInfo;
use crate::chain::acceptor::Acceptor;
use crate::chain::header_validation::Validator;
use crate::{Network, database, vm};

const MAX_POOL_BLOCKS_SIZE: usize = 1000;
const MAX_BLOCKS_TO_REQUEST: u64 = 100;
const MAX_SYNC_PEERS: usize = 32;
const SYNC_TIMEOUT: Duration = Duration::from_secs(5);
const SYNC_ATTEMPTS: u8 = 3;
const LIVE_CANDIDATE_MAX_AGE: u64 = 30;

// When we retry, rotate deterministically among peers that claimed they can
// serve the next missing height. We do not rank them by the furthest height
// they advertised, because those Quorum heights are unverified and easy to
// exaggerate.
fn next_sync_peer(
    peers: &BTreeMap<SocketAddr, u64>,
    current_peer: SocketAddr,
    next_height: u64,
) -> Option<SocketAddr> {
    peers
        .range((Excluded(current_peer), Unbounded))
        .find(|(_, height)| **height >= next_height)
        .or_else(|| {
            peers
                .range(..current_peer)
                .find(|(_, height)| **height >= next_height)
        })
        .map(|(peer, _)| *peer)
}

// Keep only peers that still claim they can serve the next missing block, and
// cap the retry set so a long or adversarial sync session does not accumulate
// an ever-growing list of peer candidates.
fn prune_sync_peers(
    peers: &mut BTreeMap<SocketAddr, u64>,
    next_height: u64,
    keep_peer: SocketAddr,
) {
    peers.retain(|_, height| *height >= next_height);

    while peers.len() > MAX_SYNC_PEERS {
        let Some(peer_addr) =
            peers.keys().copied().find(|peer| *peer != keep_peer)
        else {
            break;
        };
        peers.remove(&peer_addr);
    }
}

// Update the retry candidate set with a peer that claimed it can serve this
// sync session. We keep the highest target that peer advertised, then prune
// peers that are no longer useful or exceed the retry-set cap.
fn remember_sync_peer(
    peers: &mut BTreeMap<SocketAddr, u64>,
    peer_addr: SocketAddr,
    target_height: u64,
    next_height: u64,
    keep_peer: SocketAddr,
) {
    peers
        .entry(peer_addr)
        .and_modify(|height| *height = (*height).max(target_height))
        .or_insert(target_height);
    prune_sync_peers(peers, next_height, keep_peer);
}
/// The `OutOfSyncImpl` struct manages the synchronization state of a node
/// that is out of sync with the network. It handles the detection of missing
/// blocks, requests for block data from peers, and transitions between sync
/// states. The struct uses a rolling pool to efficiently manage the receipt
/// and processing of blocks, ensuring that blocks are stored and processed in
/// sequential order. This allows the node to catch up to the target height
/// effectively while managing network requests and block data efficiently.
///
/// # Fields
///
/// * `range: (u64, u64)` - A tuple representing the range of missing blocks
///   that need to be synchronized. The range is defined as `(tip + 1, target)`,
///   where `tip` is the current local block height and `target` is the height
///   of the block that should be synchronized to.
///
/// * `last_request: u64` - Tracks the height of the last block that was
///   requested during the synchronization process. This helps in managing which
///   blocks are still missing and need to be fetched. If the height of the last
///   request is close to the current local height (within one-third of the
///   maximum request range), more missing blocks will be requested to maintain
///   efficient synchronization.
///
/// * `start_time: SystemTime` - The timestamp marking the start of the
///   out-of-sync state. This is used to calculate timeouts and manage retry
///   attempts for synchronization. If the timeout expires without receiving
///   sufficient blocks, the node may retry synchronization or restart its
///   consensus process.
///
/// * `pool: BTreeMap<u64, Block>` - A rolling pool of blocks received from the
///   network but not yet processed or accepted. The key is the block height,
///   and the value is the `Block` itself. This pool is used to temporarily hold
///   blocks until they can be processed sequentially. When a block is accepted,
///   the pool is drained of consecutive blocks in order, helping to maintain
///   efficient synchronization. The pool has a maximum size to prevent memory
///   overflow, and blocks are prioritized based on their proximity to the
///   current height.
///
/// * `remote_peer: SocketAddr` - The address of the peer from which blocks are
///   being requested. This peer is responsible for helping the node synchronize
///   with the rest of the network. If the node receives valid blocks from this
///   peer, it may reset its timeout to allow more time for synchronization.
///
/// * `retries: u8` - The number of timeout-driven retries already performed for
///   the current sync cycle. It starts at `0` when entering `OutOfSync`, is
///   reset to `0` whenever valid block progress is made, and is incremented
///   each time the timeout expires without progress. When the timeout expires
///   while `retries == SYNC_ATTEMPTS - 1`, the node stops retrying and may
///   transition back to an in-sync state as a fallback.
///
/// * `acc: Arc<RwLock<Acceptor<N, DB, VM>>>` - A thread-safe reference to the
///   `Acceptor`, which is responsible for handling incoming blocks and managing
///   the consensus process during synchronization. The `Acceptor` is also used
///   to determine the current local block height and to accept blocks once they
///   are received and validated.
///
/// * `network: Arc<RwLock<N>>` - A thread-safe reference to the network trait,
///   which provides functionality for sending and receiving messages between
///   peers during the sync process. The network is used to send block requests
///   and receive block data from peers, enabling the node to synchronize its
///   state with the rest of the network.
///
/// * `local_peer: SocketAddr` - The address of the local peer (the current
///   node) that is performing the synchronization process. This address is
///   included in block requests so that peers know where to send the requested
///   block data.
///
/// # Rolling Pool Mechanism
///
/// The rolling pool is designed to efficiently handle block receipt and
/// processing while the node is out of sync. It stores blocks in a
/// `BTreeMap`, allowing the node to process blocks in order based on their
/// height. The pool has several key behaviors:
///
/// - **Sequential Acceptance**: Blocks must be processed in sequential order.
///   If a block is received out of order (i.e., not the next expected block),
///   it is added to the pool. Once the next expected block is processed, the
///   pool is drained of any remaining consecutive blocks that can be processed
///   in order.
///
/// - **Limited Size and Prioritization**: The pool has a maximum size defined
///   by `MAX_POOL_BLOCKS_SIZE`. If the pool is full, lower-priority blocks
///   (those with greater heights) may be removed to make space for more
///   relevant blocks. This ensures that the pool remains efficient and only
///   stores blocks that are close to the current height and are likely to be
///   processed soon.
///
/// - **Triggering Requests for Missing Blocks**: The node periodically checks
///   the pool to identify any missing blocks that have not yet been received.
///   If a significant number of blocks have already been requested but are
///   still missing, the node sends additional requests to peers to fetch these
///   blocks. This helps maintain a steady flow of block data and ensures the
///   node can catch up to the target height efficiently.
///
/// - **Rolling Window for Block Requests**: Block requests are made in chunks,
///   with the maximum number of blocks requested defined by
///   `MAX_BLOCKS_TO_REQUEST`. As the node accepts blocks and its local height
///   advances, it dynamically triggers new requests for any remaining missing
///   blocks within the sync range, creating a "rolling window" of requested
///   blocks. When the number of blocks requested drops below one-third of
///   `MAX_BLOCKS_TO_REQUEST`, the node triggers new requests to maintain
///   consistent synchronization progress.
///
/// - **Timeout and Retry Logic**: The sync process uses a timeout mechanism
///   (`SYNC_TIMEOUT`) to ensure that the node does not wait indefinitely for
///   blocks. If the timeout expires and progress is insufficient, the node
///   retries the block requests or transitions back to the consensus process as
///   a fallback.
///
/// This rolling pool mechanism allows the node to synchronize with the
/// network efficiently, ensuring that blocks are processed in the correct
/// order and that network requests are managed effectively to minimize
/// redundant data and processing.
pub(super) struct OutOfSyncImpl<
    DB: database::DB,
    VM: vm::VMExecution,
    N: Network,
> {
    range: (u64, u64),
    last_request: u64,
    start_time: SystemTime,
    pool: BTreeMap<u64, Block>,
    // Peers that have recently advertised a sync target we can use for this
    // session.
    peers: BTreeMap<SocketAddr, u64>,
    remote_peer: SocketAddr,
    retries: u8,

    acc: Arc<RwLock<Acceptor<N, DB, VM>>>,
    network: Arc<RwLock<N>>,

    local_peer: SocketAddr,
}

impl<DB: database::DB, VM: vm::VMExecution, N: Network>
    OutOfSyncImpl<DB, VM, N>
{
    pub async fn new(
        acc: Arc<RwLock<Acceptor<N, DB, VM>>>,
        network: Arc<RwLock<N>>,
    ) -> Self {
        let this_peer = *network.read().await.public_addr();
        Self {
            start_time: SystemTime::now(),
            range: (0, 0),
            last_request: 0,
            pool: BTreeMap::new(),
            peers: BTreeMap::new(),
            acc,
            local_peer: this_peer,
            network,
            remote_peer: SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(127, 0, 0, 1),
                8000,
            )),
            retries: 0,
        }
    }

    /// Performed when entering the OutOfSync state
    ///
    /// Handles the logic for entering the out-of-sync state. Sets the target
    /// block range, adds the `target_block` to the pool, updates the
    /// `remote_peer` address, and starts to request missing blocks
    pub async fn on_entering(&mut self, presync: PresyncInfo) {
        let peer_addr = presync.peer_addr;
        let pool = presync.pool;
        let curr_height = self.acc.read().await.get_curr_height().await;

        self.retries = 0;
        self.range = (curr_height + 1, presync.remote_height);

        // Stop consensus.
        self.acc.write().await.abort_consensus().await;

        // add target_block to the pool
        self.drain_pool().await;
        for b in &pool {
            self.pool.insert(b.header().height, b.clone());
        }
        self.peers.clear();
        // Start the retry candidate set with the peer that passed presync.
        self.peers.insert(peer_addr, self.range.1);
        self.remote_peer = peer_addr;

        if let Some(last_request) = self.request_pool_missing_blocks().await {
            self.last_request = last_request
        }

        let (from, to) = &self.range;
        info!(event = "entering", from, to, ?peer_addr);
        for (_, b) in self.pool.clone() {
            let _ = self.on_block_event(&b).await;
        }
    }

    /// performed when exiting the state
    pub async fn on_exiting(&mut self) {
        self.drain_pool().await;
        self.peers.clear();
    }

    /// Removes blocks from the pool that are below the current local height,
    /// as they are already processed and do not need further consideration.
    pub async fn drain_pool(&mut self) {
        let curr_height = self.acc.read().await.get_curr_height().await;
        self.pool.retain(|h, _| h >= &curr_height);
    }

    pub async fn on_quorum(
        &mut self,
        quorum: &Quorum,
        metadata: Option<&Metadata>,
    ) {
        let target_height = quorum.header.round.saturating_sub(1);

        if let Some(peer_addr) = metadata.map(|m| m.src_addr) {
            // Keep track of which peers claimed they could serve this range so
            // a timeout can switch to another source instead of asking the
            // same peer again.
            remember_sync_peer(
                &mut self.peers,
                peer_addr,
                target_height,
                self.range.0,
                self.remote_peer,
            );
        }

        if self.range.1 < target_height {
            debug!(
                event = "update sync target due to quorum",
                prev = self.range.1,
                new = target_height,
            );
            self.range.1 = target_height;
            if let Some(last_request) = self.request_pool_missing_blocks().await
            {
                self.last_request = last_request;
            }
        }
    }

    pub async fn on_candidate(
        &mut self,
        candidate: &Candidate,
    ) -> anyhow::Result<bool> {
        let (tip_header, db, context_provisioners) = {
            let acc = self.acc.read().await;
            let tip_header = acc.tip_header().await;
            let db = acc.db.clone();
            let context_provisioners = {
                let provisioners = acc.provisioners_list.read().await;
                provisioners.clone()
            };

            (tip_header, db, context_provisioners)
        };
        let candidate_header = candidate.candidate.header();

        // Only a Candidate that extends our current tip can prove we do
        // not need this sync session anymore.
        if candidate_header.height != tip_header.height + 1
            || candidate_header.prev_block_hash != tip_header.hash
        {
            return Ok(false);
        }

        // Only a recent Candidate can prove the live chain is already back at
        // our tip. Old but valid Candidates are replayable and should not be
        // enough to abort a legitimate sync session.
        let now = get_current_timestamp();
        if candidate_header
            .timestamp
            .saturating_add(LIVE_CANDIDATE_MAX_AGE)
            < now
        {
            debug!(
                event = "ignore stale sync candidate",
                candidate_timestamp = candidate_header.timestamp,
                now,
                height = candidate_header.height,
                iter = candidate_header.iteration,
            );
            return Ok(false);
        }

        let expected_generator = context_provisioners.current().get_generator(
            candidate_header.iteration,
            tip_header.seed,
            candidate_header.height,
        );
        let validator = Validator::new(db, &tip_header, &context_provisioners);

        match validator
            .verify_block_header_fields(
                candidate_header,
                &expected_generator,
                false,
            )
            .await
        {
            Ok(_) => {}
            Err(
                err @ (HeaderError::Storage(_, _) | HeaderError::Generic(_)),
            ) => {
                return Err(err.into());
            }
            Err(err) => {
                debug!(
                    event = "ignore sync candidate with invalid header",
                    ?err,
                    height = candidate_header.height,
                    iter = candidate_header.iteration,
                );
                return Ok(false);
            }
        }

        // Authenticate the header before we spend CPU on payload-root checks.
        // If the tip already changed, skip the expensive payload validation
        // because this sync candidate can no longer end the current session.
        let live_tip = self.acc.read().await.tip_header().await;
        if live_tip.height != tip_header.height
            || live_tip.hash != tip_header.hash
        {
            debug!(
                event = "skip sync candidate after tip change",
                snapshot_height = tip_header.height,
                live_height = live_tip.height,
            );
            return Ok(false);
        }

        if let Err(err) = validate_candidate_payload(candidate) {
            debug!(
                event = "ignore malformed sync candidate",
                ?err,
                height = candidate_header.height,
                iter = candidate_header.iteration,
            );
            return Ok(false);
        }

        // Recheck once more before leaving OutOfSync so payload validation does
        // not race with a live tip change.
        let live_tip = self.acc.read().await.tip_header().await;
        if live_tip.height != tip_header.height
            || live_tip.hash != tip_header.hash
        {
            debug!(
                event = "skip sync candidate after tip change",
                snapshot_height = tip_header.height,
                live_height = live_tip.height,
            );
            return Ok(false);
        }

        // Restart consensus before leaving OutOfSync so the Candidate already
        // in flight can continue through the normal acceptor path.
        self.acc.write().await.restart_consensus().await;
        Ok(true)
    }

    /// Processes incoming blocks during the out-of-sync state. Determines
    /// whether a block should be accepted, added to the pool, or skipped.
    /// Handles consecutive block acceptance, pool draining, and state
    /// transition checks.
    ///
    /// Returns `true` if the node should transition back to the in-sync state.
    pub async fn on_block_event(
        &mut self,
        blk: &Block,
    ) -> anyhow::Result<bool> {
        let mut acc = self.acc.write().await;
        let block_height = blk.header().height;

        let current_height = acc.get_curr_height().await;
        if block_height <= current_height {
            return Ok(false);
        }

        if block_height != current_height + 1
            && self.retries == SYNC_ATTEMPTS - 1
            && self.is_timeout_expired()
        {
            // Keep the per-block timeout fallback for non-progressing traffic
            // so a block flood cannot starve the heartbeat path. The next
            // expected block is the one exception because it can immediately
            // prove the sync session is still making progress.
            acc.restart_consensus().await;
            // Timeout-ed sync-up
            // Transit back to InSync mode
            return Ok(true);
        }

        if block_height > self.range.1 {
            debug!(
                event = "update sync target",
                prev = self.range.1,
                new = block_height,
            );
            self.range.1 = block_height
        }

        // Try accepting consecutive block
        if block_height == current_height + 1 {
            acc.accept_block(blk, false).await?;
            // reset expiry_time only if we receive a valid block
            self.start_time = SystemTime::now();
            // Reset the retry counter when a block is accepted: receiving a
            // valid block means the peer is alive and delivering.
            self.retries = 0;
            debug!(
                event = "accepted block",
                block_height,
                last_request = self.last_request,
            );
            self.range.0 = block_height + 1;

            // Try to accept other consecutive blocks from the pool, if
            // available
            for height in self.range.0..=self.range.1 {
                if let Some(blk) = self.pool.get(&height) {
                    acc.accept_block(blk, false).await?;
                    // reset expiry_time only if we receive a valid block
                    self.start_time = SystemTime::now();
                    self.range.0 += 1;
                    debug!(
                        event = "accepted next block",
                        block_height = height,
                        last_request = self.last_request,
                    );
                } else {
                    // This means we accepted a block and the next block
                    // available in the pool is not the next one
                    if let Some((&h, _)) = self.pool.first_key_value() {
                        // We then check if the first block in the pool is
                        // related to something we requested, or is just the
                        // current cluster tip.
                        // If it's something we requested, highly probably it
                        // means that we missed a block we requested (assuming
                        // that we receive block sequentially)
                        // If so, we just request the missing block using a
                        // GetResource to alive peers
                        if h < self.last_request {
                            self.request_missing_block(height).await;
                        }
                    }

                    break;
                }
            }
            self.pool.retain(|k, _| k >= &self.range.0);
            prune_sync_peers(&mut self.peers, self.range.0, self.remote_peer);
            debug!(
                event = "pool drain",
                pool_len = self.pool.len(),
                last_request = self.last_request,
            );

            let tip = acc.get_curr_height().await;
            // Check target height is reached
            if tip >= self.range.1 {
                debug!(event = "sync target reached", height = tip);
                self.pool.clear();

                // Block sync-up procedure manages to download all requested
                acc.restart_consensus().await;

                // Transit to InSync mode
                return Ok(true);
            }

            return Ok(false);
        }

        let block_height = blk.header().height;
        let pool_len = self.pool.len();

        if self.pool.contains_key(&block_height) {
            debug!(
                event = "block skipped (already present)",
                block_height, pool_len,
            );
            return Ok(false);
        }

        // If we almost dequeued all requested blocks (2/3)
        if self.last_request < current_height + (MAX_BLOCKS_TO_REQUEST / 3)
            && let Some(last_request) = self.request_pool_missing_blocks().await
        {
            self.last_request = last_request
        }

        // if the pool is full, check if the new block has higher priority
        if pool_len >= MAX_POOL_BLOCKS_SIZE
            && let Some(entry) = self.pool.last_entry()
        {
            let stored_height = *entry.key();
            if stored_height > block_height {
                debug!(
                    event = "block removed",
                    block_height, stored_height, pool_len,
                );
                entry.remove();
            } else {
                debug!(event = "block skipped", block_height, pool_len);
                return Ok(false);
            }
        }

        // add block to the pool
        self.pool.insert(block_height, blk.clone());

        debug!(
            event = "block saved",
            block_height,
            pool_len = self.pool.len(),
        );

        Ok(false)
    }

    fn is_timeout_expired(&self) -> bool {
        self.start_time.checked_add(SYNC_TIMEOUT).unwrap() <= SystemTime::now()
    }

    pub async fn on_heartbeat(&mut self) -> anyhow::Result<bool> {
        if self.is_timeout_expired() {
            if self.retries == SYNC_ATTEMPTS - 1 {
                debug!(event = "timer expired", retries = self.retries);
                // sync-up has timed out, recover consensus task
                self.acc.write().await.restart_consensus().await;

                // sync-up timed out after exhausting all retries
                // Transit back to InSync mode as a fail-over
                return Ok(true);
            }

            self.retries += 1;
            prune_sync_peers(&mut self.peers, self.range.0, self.remote_peer);

            if let Some(peer_addr) =
                next_sync_peer(&self.peers, self.remote_peer, self.range.0)
            {
                // Retry against another plausible peer before reissuing the
                // same missing-block request window.
                debug!(
                    event = "rotate sync peer",
                    prev = ?self.remote_peer,
                    new = ?peer_addr,
                    next_height = self.range.0,
                );
                self.remote_peer = peer_addr;
            }

            // Request missing from local_pool blocks
            if let Some(last_request) = self.request_pool_missing_blocks().await
            {
                self.last_request = last_request
            }

            self.start_time = SystemTime::now();
        }

        Ok(false)
    }

    async fn request_missing_block(&self, height: u64) {
        let mut inv = Inv::new(0);
        inv.add_block_from_height(height);
        let get_resource =
            GetResource::new(inv, Some(self.local_peer), u64::MAX, 1);

        debug!(event = "request block", height);
        if let Err(e) = self
            .network
            .read()
            .await
            .send_to_alive_peers(get_resource.into(), 2)
            .await
        {
            warn!(event = "Unable to request missing block", ?e);
        }
    }

    /// Scans the current block range for any missing blocks that are not
    /// present in the pool and requests them from the `remote_peer`.
    ///
    /// Returns the height of the last block requested, if any.
    async fn request_pool_missing_blocks(&self) -> Option<u64> {
        let mut last_request = None;
        let mut inv = Inv::new(0);

        let mut inv_count = 0;
        for height in self.range.0..=self.range.1 {
            if self.pool.contains_key(&height) {
                // already received
                continue;
            }
            inv.add_block_from_height(height);
            inv_count += 1;
            last_request = Some(height);
            if inv_count >= MAX_BLOCKS_TO_REQUEST {
                break;
            }
        }

        if !inv.inv_list.is_empty() {
            debug!(
                event = "request blocks",
                target = last_request.unwrap_or_default(),
            );

            let get_resource =
                GetResource::new(inv, Some(self.local_peer), u64::MAX, 1);

            if let Err(e) = self
                .network
                .read()
                .await
                .send_to_peer(get_resource.into(), self.remote_peer)
                .await
            {
                debug!(event = "Unable to request missing blocks", ?e);
                warn!("Unable to request missing blocks {e}");
                return None;
            }
        }
        last_request
    }
}

// Reject obviously malformed Candidates before doing the more expensive
// consensus-specific header checks.
fn validate_candidate_payload(candidate: &Candidate) -> anyhow::Result<()> {
    let candidate_block = &candidate.candidate;
    let candidate_header = candidate_block.header();

    if candidate_block.txs().len() > MAX_NUMBER_OF_TRANSACTIONS {
        anyhow::bail!("candidate contains too many transactions");
    }

    if candidate_block.faults().len() > MAX_NUMBER_OF_FAULTS {
        anyhow::bail!("candidate contains too many faults");
    }

    let candidate_size = candidate_block
        .size()
        .map_err(|_| anyhow::anyhow!("unknown block size"))?;

    if candidate_size > MAX_BLOCK_SIZE {
        anyhow::bail!("candidate exceeds max block size");
    }

    let tx_digests: Vec<_> =
        candidate_block.txs().iter().map(|tx| tx.digest()).collect();
    if merkle_root(&tx_digests[..]) != candidate_header.txroot {
        anyhow::bail!("candidate has invalid tx merkle root");
    }

    let fault_digests: Vec<_> = candidate_block
        .faults()
        .iter()
        .map(|fault| fault.digest())
        .collect();
    if merkle_root(&fault_digests[..]) != candidate_header.faultroot {
        anyhow::bail!("candidate has invalid fault merkle root");
    }

    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotates_to_next_eligible_sync_peer() {
        let current_peer: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let alt_one: SocketAddr = "127.0.0.1:9001".parse().unwrap();
        let alt_two: SocketAddr = "127.0.0.1:9002".parse().unwrap();

        let peers = BTreeMap::from([
            (current_peer, 100),
            (alt_one, 105),
            (alt_two, 110),
        ]);

        assert_eq!(next_sync_peer(&peers, current_peer, 101), Some(alt_one),);
    }

    #[test]
    fn wraps_to_earliest_eligible_sync_peer() {
        let first_peer: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let second_peer: SocketAddr = "127.0.0.1:9001".parse().unwrap();
        let current_peer: SocketAddr = "127.0.0.1:9002".parse().unwrap();

        let peers = BTreeMap::from([
            (first_peer, 104),
            (second_peer, 105),
            (current_peer, 110),
        ]);

        assert_eq!(next_sync_peer(&peers, current_peer, 101), Some(first_peer),);
    }

    #[test]
    fn prunes_sync_peers_below_next_height() {
        let keep_peer: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let stale_peer: SocketAddr = "127.0.0.1:9001".parse().unwrap();
        let useful_peer: SocketAddr = "127.0.0.1:9002".parse().unwrap();

        let mut peers = BTreeMap::from([
            (keep_peer, 110),
            (stale_peer, 100),
            (useful_peer, 111),
        ]);

        prune_sync_peers(&mut peers, 105, keep_peer);

        assert_eq!(peers.len(), 2);
        assert!(peers.contains_key(&keep_peer));
        assert!(peers.contains_key(&useful_peer));
        assert!(!peers.contains_key(&stale_peer));
    }

    #[test]
    fn caps_sync_peer_set_without_dropping_current_peer() {
        let keep_peer: SocketAddr = "127.0.0.1:9050".parse().unwrap();
        let mut peers = BTreeMap::new();

        for port in 9000..=9033 {
            let peer: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
            peers.insert(peer, 200);
        }
        peers.insert(keep_peer, 200);

        prune_sync_peers(&mut peers, 150, keep_peer);

        assert_eq!(peers.len(), MAX_SYNC_PEERS);
        assert!(peers.contains_key(&keep_peer));
    }
}
