// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::cmp::Ordering;
use std::ops::Deref;

use node_data::message::payload::{GetResource, Inv, Quorum};
use node_data::message::Message;

use super::*;
use crate::chain::fallback;

pub(super) struct InSyncImpl<DB: database::DB, VM: vm::VMExecution, N: Network>
{
    acc: Arc<RwLock<Acceptor<N, DB, VM>>>,
    network: Arc<RwLock<N>>,

    blacklisted_blocks: SharedHashSet,
    presync: Option<PresyncInfo>,
}

impl<DB: database::DB, VM: vm::VMExecution, N: Network> InSyncImpl<DB, VM, N> {
    pub fn new(
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
    pub async fn on_entering(&mut self, blk: &Block) -> anyhow::Result<()> {
        let mut acc = self.acc.write().await;
        let curr_h = acc.get_curr_height().await;

        if blk.header().height == curr_h + 1 {
            acc.accept_block(blk, true).await?;
        }

        info!(event = "entering in-sync", height = curr_h);

        Ok(())
    }

    /// performed when exiting the state
    pub async fn on_exiting(&mut self) {
        self.presync = None
    }

    pub async fn on_quorum(
        &mut self,
        remote_quorum: &Quorum,
        metadata: Option<&Metadata>,
    ) {
        // If remote_blk.height > tip.height+1, we might be out of sync.
        // Before switching to outOfSync mode and download missing blocks,
        // we ensure that the peer has a valid successor of tip
        if let Some(peer_addr) = metadata.map(|m| m.src_addr) {
            // If there's no active presync process, we proceed with validation
            if self.presync.is_none() {
                let tip_height = self.acc.read().await.get_curr_height().await;
                // We use the quorum's previous block, to be sure that network
                // already have the full block available
                let remote_height = remote_quorum.header.round - 1;
                // Don't compare with `= tip + 1` because that's supposed to be
                // handled by the InSync
                if remote_height > tip_height + 1 {
                    // Initialize the presync process, storing metadata about
                    // the peer, the remote height, and our current tip height.
                    // This serves as a safeguard to avoid switching into
                    // out-of-sync mode without verifying the peer's
                    // information.
                    self.presync = Some(PresyncInfo::from_height(
                        peer_addr,
                        remote_height,
                        tip_height,
                    ));

                    // Request the block immediately following our tip height
                    // from the peer to verify if the peer has a valid
                    // continuation of our chain.
                    // If the requested block (from the same peer) is accepted
                    // by the on_block_event before the presync timer expires,
                    // we will transition into out_of_sync mode.
                    self.request_block(tip_height + 1, peer_addr).await;
                }
            }
        }
    }

    /// Return Some if there is the need to switch to OutOfSync mode.
    /// This way the sync-up procedure to download all missing blocks from the
    /// main chain will be triggered
    pub async fn on_block_event(
        &mut self,
        remote_blk: &Block,
        metadata: Option<Metadata>,
    ) -> anyhow::Result<Option<PresyncInfo>> {
        let mut acc = self.acc.write().await;
        let tip_header = acc.tip_header().await;
        let tip_height = tip_header.height;
        let remote_header = remote_blk.header();
        let remote_height = remote_header.height;

        // If we already accepted a block with the same height as remote_blk,
        // check if remote_blk has higher priority. If so, we revert to its
        // prev_block, and accept it as the new tip
        if remote_height <= tip_height {
            // Ensure the block is different from what we have in our chain
            if remote_height == tip_height {
                if remote_header.hash == tip_header.hash {
                    return Ok(None);
                }
            } else {
                let blk_exists = acc
                    .db
                    .read()
                    .await
                    .view(|t| t.block_exists(&remote_header.hash))?;

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
            let prev_blk_exists = acc
                .db
                .read()
                .await
                .view(|t| t.block_exists(&remote_header.prev_block_hash))?;

            if !prev_blk_exists {
                warn!(
                    "received block from fork at height {remote_height}: {}",
                    remote_header.hash.hex()
                );
                return Ok(None);
            }

            // Fetch the chain block at the same height as remote_blk
            let local_blk = if remote_height == tip_height {
                acc.tip.read().await.inner().clone()
            } else {
                acc.db
                    .read()
                    .await
                    .view(|t| t.block_by_height(remote_height))?
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
                                .block_header(&remote_header.prev_block_hash)?
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
                            acc.accept_block(remote_blk, true).await?;
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
                    let local_hash = local_header.hash.hex();
                    let remote_hash = remote_header.hash.hex();
                    warn!("Double candidate detected. Local block: {local_hash}, remote block {remote_hash}");
                }
            }

            return Ok(None);
        }

        // If remote_blk is a successor of our tip, we try to accept it
        if remote_height == tip_height + 1 {
            let finalized = acc.accept_block(remote_blk, true).await?;

            // On first final block accepted while we're inSync, clear
            // blacklisted blocks
            if finalized {
                self.blacklisted_blocks.write().await.clear();
            }

            // If the accepted block is the one requested to presync peer,
            // switch to OutOfSync/Syncing mode
            if let Some(metadata) = &metadata {
                let same = self
                    .presync
                    .as_ref()
                    .map(|presync| {
                        metadata.src_addr == presync.peer_addr
                            && remote_height == presync.start_height() + 1
                    })
                    .unwrap_or_default();
                if same {
                    return Ok(self.presync.take());
                }
            }

            return Ok(None);
        }

        // If remote_blk.height > tip.height+1, we might be out of sync.
        // Before switching to outOfSync mode and download missing blocks,
        // we ensure that the peer has a valid successor of tip
        if let Some(peer_addr) = metadata.map(|m| m.src_addr) {
            match self.presync.as_mut() {
                // If there's no active presync process, we proceed with
                // validation
                None => {
                    self.presync = Some(PresyncInfo::from_block(
                        peer_addr,
                        remote_blk.clone(),
                        tip_height,
                    ));

                    self.request_block(tip_height + 1, peer_addr).await;
                }
                // If there's an active presync process, we add the received
                // block to the pool so to process it when the sync procedure
                // will start
                Some(pre) => {
                    if pre.peer_addr == peer_addr {
                        pre.pool.push(remote_blk.clone())
                    }
                }
            }
        }

        Ok(None)
    }

    /// Requests a block by height from a `peer_addr`
    async fn request_block(&self, height: u64, peer_addr: SocketAddr) {
        let network = self.network.read().await;
        let mut inv = Inv::new(1);
        inv.add_block_from_height(height);
        let this_peer = *network.public_addr();
        let req = GetResource::new(inv, Some(this_peer), u64::MAX, 1);
        debug!(event = "request block by height", ?req, ?peer_addr);

        if let Err(err) = network.send_to_peer(req.into(), peer_addr).await {
            warn!("could not request block {err}")
        }
    }

    pub async fn on_heartbeat(&mut self) -> anyhow::Result<bool> {
        if let Some(pre_sync) = &mut self.presync {
            if pre_sync.expiry <= Instant::now() {
                // Reset presync if it timed out
                self.presync = None;
            }
        }

        Ok(false)
    }
}
