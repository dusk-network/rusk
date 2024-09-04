// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node_data::{
    ledger::{to_str, Block},
    message::{payload::GetBlocks, Message},
};
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;
use tracing::{error, warn};

use crate::{
    database::{self, Ledger},
    vm::VMExecution,
    Network,
};

use super::acceptor::Acceptor;

use super::fsm::REDUNDANCY_PEER_FACTOR;

const CONSECUTIVE_BLOCKS_THRESHOLD: usize = 5;
const STALLED_TIMEOUT: u64 = 30; // seconds

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum State {
    /// Blocks are being accepted
    Running,
    /// No block has been accepted recently
    ///
    /// Node might be stuck on non-main branch and might need to recover
    Stalled,
    /// Node is disconnected from the main branch
    StalledOnFork,
}

/// Implements a simple FSM to detect a stalled state of the chain
pub(crate) struct StalledChainFSM<DB: database::DB, N: Network, VM: VMExecution>
{
    acc: Arc<RwLock<Acceptor<N, DB, VM>>>,

    state: State,
    recovery_blocks: BTreeMap<u64, Block>,

    /// Latest finalized block
    latest_finalized: Block,

    /// Tip of the chain with timestamp
    tip: (Block, u64),
}

impl<DB: database::DB, N: Network, VM: VMExecution> StalledChainFSM<DB, N, VM> {
    pub(crate) fn new(
        acc: Arc<RwLock<Acceptor<N, DB, VM>>>,
        latest_finalized: Block,
        tip: Block,
    ) -> Self {
        let mut sm = Self {
            state: State::Running,
            recovery_blocks: BTreeMap::new(),
            tip: Default::default(),
            latest_finalized,
            acc,
        };

        sm.update_tip(tip);
        sm
    }

    /// Handles block received event
    ///
    /// Returns the new state of the FSM after processing the block
    pub(crate) async fn on_block_received(&mut self, blk: &Block) -> State {
        let tip = self.acc.read().await.get_curr_tip().await;

        if self.tip.0.header().hash != tip.inner().header().hash {
            // Tip has changed, which means a new block is accepted either due
            // to normal block acceptance or fallback execution
            self.recovery_blocks.clear();

            if tip.is_final() {
                self.latest_finalized = tip.inner().clone();
            }

            self.update_tip(tip.inner().clone());
            self.state = State::Running;
        }

        let curr = self.state;
        self.state = match curr {
            State::Running => self.on_running().await,
            State::Stalled => self.on_stalled(blk).await,
            State::StalledOnFork => curr,
        };

        self.state
    }

    /// Returns recovery blocks as a vector
    pub(crate) fn recovery_blocks(&self) -> Vec<Block> {
        self.recovery_blocks.values().cloned().collect()
    }

    /// Handles a running state
    async fn on_running(&self) -> State {
        if self.tip.1 + STALLED_TIMEOUT
            < SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
        {
            // While we are still receiving blocks, no block
            // has been accepted for a long time (tip has not changed
            // recently)
            return self.on_accept_block_timeout().await;
        }

        State::Running
    }

    /// Handles block from wire in the `Stalled` state
    async fn on_stalled(&mut self, new_blk: &Block) -> State {
        let key = new_blk.header().height;
        self.recovery_blocks
            .entry(key)
            .or_insert_with(|| new_blk.clone());

        if self.recovery_blocks.len() < CONSECUTIVE_BLOCKS_THRESHOLD {
            // Not enough consecutive blocks collected yet
            return State::Stalled;
        }

        // Check recovery blocks contains at most N consecutive blocks
        let mut prev = self.latest_finalized.header().height;

        let consecutive = self.recovery_blocks.keys().all(|&key| {
            let is_consecutive: bool = key == prev + 1;
            prev = key;
            is_consecutive
        });

        if !consecutive {
            // recovery blocks are missing
            return State::Stalled;
        }

        let db = &self.acc.read().await.db;

        // Detect if collected blocks are valid
        for (_, blk) in self.recovery_blocks.iter() {
            let exists = db
                .read()
                .await
                .view(|t| t.get_block_exists(&blk.header().hash))
                .unwrap(); // TODO:

            if !exists {
                // Block already exists in ledger
                continue;
            }

            let local_blk = db
                .read()
                .await
                .view(|t| t.fetch_block_header(&blk.header().prev_block_hash))
                .unwrap();

            if local_blk.is_none() {
                // Block is invalid
                error!(
                    event = "revert failed",
                    hash = to_str(&blk.header().hash),
                    err = format!("could not find prev block")
                );

                return State::Stalled;
            }

            // If we are here, most probably this is a block from the main
            // branch
            match self
                .acc
                .read()
                .await
                .verify_header_against_local(
                    local_blk.as_ref().unwrap(),
                    blk.header(),
                )
                .await
            {
                Ok(_) => return State::StalledOnFork,
                Err(err) => {
                    // Block is invalid
                    error!(
                        event = "revert failed", // TODO:
                        hash = to_str(&blk.header().hash),
                        err = format!("{:?}", err)
                    );
                }
            }
        }

        State::Stalled
    }

    /// Handles block acceptance timeout
    ///
    /// Request missing blocks since last finalized block
    async fn on_accept_block_timeout(&self) -> State {
        // Request missing blocks since my last finalized block
        let get_blocks = Message::new_get_blocks(GetBlocks {
            locator: self.latest_finalized.header().hash,
        });

        let network = &self.acc.read().await.network;
        if let Err(e) = network
            .read()
            .await
            .send_to_alive_peers(&get_blocks, REDUNDANCY_PEER_FACTOR)
            .await
        {
            warn!("Unable to request GetBlocks {e}");
            return State::Running;
        }

        State::Stalled
    }

    fn update_tip(&mut self, tip: Block) {
        self.tip.0 = tip;
        self.tip.1 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}
