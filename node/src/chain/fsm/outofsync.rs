// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::BTreeMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;
use std::{sync::Arc, time::SystemTime};

use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use node_data::ledger::Block;
use node_data::message::payload::{GetResource, Inv};

use crate::chain::acceptor::Acceptor;
use crate::{database, vm, Network};

use super::PresyncInfo;

const MAX_POOL_BLOCKS_SIZE: usize = 1000;
const MAX_BLOCKS_TO_REQUEST: u64 = 100;
const SYNC_TIMEOUT: Duration = Duration::from_secs(5);

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
/// * `attempts: u8` - The number of attempts remaining for requesting missing
///   blocks before giving up and restarting the consensus process. Each time
///   the timeout expires without progress, this counter is decremented. When it
///   reaches zero, the node will stop retrying and may transition back to an
///   in-sync state as a fallback.
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
    remote_peer: SocketAddr,
    attempts: u8,

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
            acc,
            local_peer: this_peer,
            network,
            remote_peer: SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(127, 0, 0, 1),
                8000,
            )),
            attempts: 3,
        }
    }
    /// Performed when entering the OutOfSync state
    ///
    /// Handles the logic for entering the out-of-sync state. Sets the target
    /// block range, adds the `target_block` to the pool, updates the
    /// `remote_peer` address, and starts to request missing blocks
    pub async fn on_entering(&mut self, presync: PresyncInfo) {
        let target_block = presync.target_blk;
        let peer_addr = presync.peer_addr;
        let pool = presync.pool;
        let curr_height = self.acc.read().await.get_curr_height().await;

        self.range = (curr_height + 1, target_block.header().height);

        // add target_block to the pool
        let key = target_block.header().height;
        self.drain_pool().await;
        for b in &pool {
            self.pool.insert(b.header().height, b.clone());
        }
        self.pool.insert(key, target_block);
        self.remote_peer = peer_addr;

        if let Some(last_request) = self.request_pool_missing_blocks().await {
            self.last_request = last_request
        }

        let (from, to) = &self.range;
        info!(event = "entering out-of-sync", from, to, ?peer_addr);
        for (_, b) in self.pool.clone() {
            let _ = self.on_block_event(&b).await;
        }
    }

    /// performed when exiting the state
    pub async fn on_exiting(&mut self) {
        self.drain_pool().await;
    }

    /// Removes blocks from the pool that are below the current local height,
    /// as they are already processed and do not need further consideration.
    pub async fn drain_pool(&mut self) {
        let curr_height = self.acc.read().await.get_curr_height().await;
        self.pool.retain(|h, _| h >= &curr_height);
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

        if self.attempts == 0 && self.is_timeout_expired() {
            acc.restart_consensus().await;
            // Timeout-ed sync-up
            // Transit back to InSync mode
            return Ok(true);
        }

        let current_height = acc.get_curr_height().await;
        if block_height <= current_height {
            return Ok(false);
        }

        if block_height > self.range.1 {
            debug!(
                event = "update sync target",
                prev = self.range.1,
                new = block_height,
                mode = "out_of_sync"
            );
            self.range.1 = block_height
        }

        // Try accepting consecutive block
        if block_height == current_height + 1 {
            acc.try_accept_block(blk, false).await?;
            // reset expiry_time only if we receive a valid block
            self.start_time = SystemTime::now();
            debug!(
                event = "accepted block",
                block_height = block_height,
                last_request = self.last_request,
                mode = "out_of_sync"
            );
            self.range.0 = block_height + 1;

            // Try to accept other consecutive blocks from the pool, if
            // available
            for height in self.range.0..=self.range.1 {
                if let Some(blk) = self.pool.get(&height) {
                    acc.try_accept_block(blk, false).await?;
                    // reset expiry_time only if we receive a valid block
                    self.start_time = SystemTime::now();
                    self.range.0 += 1;
                    debug!(
                        event = "accepting next block",
                        block_height = height,
                        last_request = self.last_request,
                        mode = "out_of_sync"
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
            debug!(
                event = "pool drain",
                pool_len = self.pool.len(),
                last_request = self.last_request,
                mode = "out_of_sync"
            );

            let tip = acc.get_curr_height().await;
            // Check target height is reached
            if tip >= self.range.1 {
                debug!(
                    event = "sync target reached",
                    height = tip,
                    mode = "out_of_sync"
                );
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
                block_height,
                pool_len,
                mode = "out_of_sync"
            );
            return Ok(false);
        }

        // If we almost dequeued all requested blocks (2/3)
        if self.last_request < current_height + (MAX_BLOCKS_TO_REQUEST / 3) {
            if let Some(last_request) = self.request_pool_missing_blocks().await
            {
                self.last_request = last_request
            }
        }

        // if the pool is full, check if the new block has higher priority
        if pool_len >= MAX_POOL_BLOCKS_SIZE {
            if let Some(entry) = self.pool.last_entry() {
                let stored_height = *entry.key();
                if stored_height > block_height {
                    debug!(
                        event = "block removed",
                        block_height,
                        stored_height,
                        pool_len,
                        mode = "out_of_sync"
                    );
                    entry.remove();
                } else {
                    debug!(
                        event = "block skipped",
                        block_height,
                        pool_len,
                        mode = "out_of_sync"
                    );
                    return Ok(false);
                }
            }
        }

        // add block to the pool
        self.pool.insert(block_height, blk.clone());

        debug!(
            event = "block saved",
            block_height,
            pool_len = self.pool.len(),
            mode = "out_of_sync"
        );

        Ok(false)
    }

    fn is_timeout_expired(&self) -> bool {
        self.start_time.checked_add(SYNC_TIMEOUT).unwrap() <= SystemTime::now()
    }

    pub async fn on_heartbeat(&mut self) -> anyhow::Result<bool> {
        if self.is_timeout_expired() {
            if self.attempts == 0 {
                debug!(
                    event = "out_of_sync timer expired",
                    attempts = self.attempts,
                    mode = "out_of_sync"
                );
                // sync-up has timed out, recover consensus task
                self.acc.write().await.restart_consensus().await;

                // sync-up timed out for N attempts
                // Transit back to InSync mode as a fail-over
                return Ok(true);
            }

            // Request missing from local_pool blocks
            if let Some(last_request) = self.request_pool_missing_blocks().await
            {
                self.last_request = last_request
            }

            self.start_time = SystemTime::now();
            self.attempts -= 1;
        }

        Ok(false)
    }

    async fn request_missing_block(&self, height: u64) {
        let mut inv = Inv::new(0);
        inv.add_block_from_height(height);
        let get_resource =
            GetResource::new(inv, Some(self.local_peer), u64::MAX, 1);

        debug!(event = "request block", height, mode = "out_of_sync",);
        if let Err(e) = self
            .network
            .read()
            .await
            .send_to_alive_peers(get_resource.into(), 2)
            .await
        {
            warn!(
                event = "Unable to request missing block",
                ?e,
                mode = "out_of_sync",
            );
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
                mode = "out_of_sync",
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
                debug!(
                    event = "Unable to request missing blocks",
                    ?e,
                    mode = "out_of_sync",
                );
                warn!("Unable to request missing blocks {e}");
                return None;
            }
        }
        last_request
    }
}
