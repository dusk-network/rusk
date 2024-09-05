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
use tracing::{error, info, warn};

use crate::{
    database::{self, Ledger},
    vm::VMExecution,
    Network,
};

use super::acceptor::Acceptor;

use super::fsm::REDUNDANCY_PEER_FACTOR;

const STALLED_TIMEOUT: u64 = 60; // seconds

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
    pub(crate) async fn new_with_acc(
        acc: Arc<RwLock<Acceptor<N, DB, VM>>>,
    ) -> Self {
        let tip = acc.read().await.get_curr_tip().await;
        let latest_finalized = acc
            .read()
            .await
            .get_latest_final_block()
            .await
            .expect("latest final block should exist");

        let mut sm = Self {
            state: State::Running,
            recovery_blocks: BTreeMap::new(),
            tip: Default::default(),
            latest_finalized,
            acc,
        };

        sm.update_tip(tip.inner().clone());
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
            self.state_transition(State::Running);
        }

        let curr = self.state;
        match curr {
            State::Running => self.on_running().await,
            State::Stalled => {
                if let Err(err) = self.on_stalled(blk).await {
                    error!("Error while processing block: {:?}", err);
                }
            }
            State::StalledOnFork => warn!("Stalled on fork"),
        };

        self.state
    }

    /// Returns recovery blocks as a vector
    pub(crate) fn recovery_blocks(&self) -> Vec<Block> {
        self.recovery_blocks.values().cloned().collect()
    }

    /// Handles a running state
    async fn on_running(&mut self) {
        if self.tip.1 + STALLED_TIMEOUT
            < SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
        {
            // While we are still receiving blocks, no block
            // has been accepted for a long time (tip has not changed
            // recently)
            self.on_accept_block_timeout().await
        }
    }

    /// Handles block from wire in the `Stalled` state
    async fn on_stalled(&mut self, new_blk: &Block) -> anyhow::Result<()> {
        let key = new_blk.header().height;
        self.recovery_blocks
            .entry(key)
            .or_insert_with(|| new_blk.clone());

        // Ensure all blocks from local_final until current_tip are
        // collected
        let from = self.latest_finalized.header().height;
        let to = self.tip.0.header().height + 1;
        for height in from..to {
            if !self.recovery_blocks.contains_key(&height) {
                return Ok(()); // wait for more blocks
            }
        }

        // Detect if collected blocks are valid
        for (_, blk) in self.recovery_blocks.iter() {
            let db: Arc<RwLock<DB>> = self.acc.read().await.db.clone();

            let exists = db
                .read()
                .await
                .view(|t| t.get_block_exists(&blk.header().hash))?;

            if exists {
                // Block already exists in ledger
                continue;
            }

            let local_blk = db
                .read()
                .await
                .view(|t| t.fetch_block_by_height(blk.header().height))?;

            let local_blk = match local_blk {
                Some(blk) => blk,
                None => {
                    error!(
                        event = "recovery failed",
                        hash = to_str(&blk.header().hash),
                        err = format!(
                            "could not find local block at height {}",
                            blk.header().height
                        )
                    );
                    return Ok(());
                }
            };

            let main_branch_blk = blk;

            // If we are here, most probably this is a block from the main
            // branch
            let res = self
                .acc
                .read()
                .await
                .verify_header_against_local(
                    local_blk.header(),
                    main_branch_blk.header(),
                )
                .await;

            if let Err(err) = res {
                error!(
                    event = "recovery failed",
                    local_hash = to_str(&local_blk.header().hash),
                    remote_hash = to_str(&blk.header().hash),
                    err = format!("verification err: {:?}", err)
                );
            } else {
                self.state_transition(State::StalledOnFork);
                return Ok(());
            }
        }

        Ok(())
    }

    /// Handles block acceptance timeout
    ///
    /// Request missing blocks since last finalized block
    async fn on_accept_block_timeout(&mut self) {
        // Request missing blocks since my last finalized block
        let get_blocks = Message::new_get_blocks(GetBlocks {
            locator: self.latest_finalized.header().hash,
        });

        let network = self.acc.read().await.network.clone();
        if let Err(e) = network
            .read()
            .await
            .send_to_alive_peers(&get_blocks, REDUNDANCY_PEER_FACTOR)
            .await
        {
            warn!("Unable to request GetBlocks {e}");
            return;
        }

        self.state_transition(State::Stalled);
    }

    fn update_tip(&mut self, tip: Block) {
        self.tip.0 = tip;
        self.tip.1 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Changes curr state and logs the transition event
    fn state_transition(&mut self, state: State) -> State {
        if state == self.state {
            return state;
        }

        self.state = state;

        let state_str: &str = match state {
            State::Running => "running",
            State::Stalled => "stalled",
            State::StalledOnFork => "stalled_on_fork",
        };

        let hdr = self.tip.0.header();
        info!(
            event = format!("chain.{}", state_str),
            tip_hash = to_str(&hdr.hash),
            tip_height = hdr.height,
            tip_iter = hdr.iteration,
            tip_updated_at = self.tip.1,
            recovery_blocks = self.recovery_blocks.len(),
            final_block = to_str(&self.latest_finalized.header().hash),
            final_block_height = self.latest_finalized.header().height,
        );

        state
    }
}
