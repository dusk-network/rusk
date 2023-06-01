// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::{acceptor::Acceptor, consensus, genesis};
use crate::database::{self, Ledger};
use crate::{vm, Network};
use dusk_consensus::user::provisioners::Provisioners;
use node_data::ledger::{self, Block, Hash, Transaction};
use node_data::message::payload::GetBlocks;
use node_data::message::Message;
use std::collections::HashMap;
use std::time::Duration;
use std::{sync::Arc, time::SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info};

const MAX_BLOCKS_TO_REQUEST: i16 = 50;
const EXPIRY_TIMEOUT_MILLIS: i16 = 5000;

enum State<N: Network, DB: database::DB, VM: vm::VMExecution> {
    InSync(InSyncImpl<DB, VM, N>),
    OutOfSync(OutOfSyncImpl<DB, VM, N>),
    InFallback,
}

/// Implements a finite-state-machine to manage InSync, OutOfSync and
/// InFallback states.
pub(crate) struct SimpleFSM<N: Network, DB: database::DB, VM: vm::VMExecution> {
    curr: State<N, DB, VM>,
    acc: Arc<RwLock<Acceptor<N, DB, VM>>>,
    network: Arc<RwLock<N>>,
}

impl<N: Network, DB: database::DB, VM: vm::VMExecution> SimpleFSM<N, DB, VM> {
    pub fn new(
        acc: Arc<RwLock<Acceptor<N, DB, VM>>>,
        network: Arc<RwLock<N>>,
    ) -> Self {
        Self {
            curr: State::InSync(InSyncImpl::<DB, VM, N>::new(
                acc.clone(),
                network.clone(),
            )),
            acc,
            network,
        }
    }

    pub async fn on_event(
        &mut self,
        blk: &Block,
        msg: &Message,
    ) -> anyhow::Result<()> {
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
                    let mut next =
                        InSyncImpl::new(self.acc.clone(), self.network.clone());
                    next.on_entering(blk).await;
                    self.curr = State::InSync(next);
                }
            }
            State::InFallback => {
                // TODO: This will be handled with another issue
            }
        }
        Ok(())
    }
}

struct InSyncImpl<DB: database::DB, VM: vm::VMExecution, N: Network> {
    acc: Arc<RwLock<Acceptor<N, DB, VM>>>,
    network: Arc<RwLock<N>>,
}

impl<DB: database::DB, VM: vm::VMExecution, N: Network> InSyncImpl<DB, VM, N> {
    fn new(
        acc: Arc<RwLock<Acceptor<N, DB, VM>>>,
        network: Arc<RwLock<N>>,
    ) -> Self {
        Self { acc, network }
    }

    /// performed when entering the state
    async fn on_entering(&mut self, blk: &Block) -> anyhow::Result<()> {
        let acc = self.acc.write().await;
        let curr_h = acc.get_curr_height().await;

        if blk.header.height == curr_h + 1 {
            acc.try_accept_block(blk, true).await?;
        }

        info!("entering in-sync at {}", curr_h,);

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
        let h = blk.header.height;
        let curr_h = acc.get_curr_height().await;

        if h <= curr_h {
            return Ok(false);
        }

        // Try accepting consecutive block
        if h == curr_h + 1 {
            acc.try_accept_block(blk, true).await?;

            // When accepting block from the wire in inSync state, we
            // rebroadcast it
            self.network.write().await.broadcast(msg).await;

            return Ok(false);
        }

        // Transition to OutOfSync
        Ok(self.allow_transition(msg).is_ok())
    }

    fn allow_transition(&self, msg: &Message) -> anyhow::Result<()> {
        let recv_peer = msg
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

        let dest_addr = msg.metadata.as_ref().unwrap().src_addr;

        self.range = (
            curr_height,
            std::cmp::min(
                curr_height + MAX_BLOCKS_TO_REQUEST as u64,
                blk.header.height,
            ),
        );

        // Request missing blocks from source peer
        let gb_msg = Message::new_get_blocks(GetBlocks { locator });

        self.network
            .write()
            .await
            .send_to_peer(&gb_msg, dest_addr)
            .await;

        // add to the pool
        let key = blk.header.height;
        self.pool.insert(key, blk.clone());

        info!(
            "entering out-of-sync with range: {:?} peer: {:?}",
            self.range, dest_addr,
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
        let h = blk.header.height;

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
        let key = blk.header.height;
        self.pool.insert(key, blk.clone());

        Ok(false)
    }
}
