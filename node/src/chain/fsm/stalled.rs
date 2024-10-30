// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node_data::{ledger::Header, message::payload};

use anyhow::{anyhow, Result};

use super::*;
use crate::vm::VMExecution;

/// Timeout for accepting a block.
/// If we have not accepted a block for more than this time, we switch to
/// stalled state
const ACCEPT_TIMEOUT: u64 = 60; // seconds

/// If we are in stalled state for more than this time, we need to re-request
/// missing blocks
const STALLED_TIMEOUT: u64 = 30; // seconds

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum State {
    /// Blocks are being accepted
    Running,
    /// No block has been accepted recently
    ///
    /// Node might be stuck on non-main branch and might need to recover
    /// It could be also stalled due to temporary network issues or main branch
    /// not producing blocks
    Stalled(u64),
    /// Node is disconnected from the main branch
    StalledOnFork([u8; 32], Box<Block>),
}

/// Implements a simple FSM to detect a stalled state of the chain
///
/// Supported state transitions:
///
/// Normal transitions:
/// Running -> Running ... (no state change)
///
/// Emergency transitions:
///
/// Running -> Stalled -> Running
///
/// Running -> Stalled -> StalledOnFork -> Running
pub(crate) struct StalledChainFSM<DB: database::DB, N: Network, VM: VMExecution>
{
    acc: Arc<RwLock<Acceptor<N, DB, VM>>>,

    state: State,

    /// Tip of the chain with timestamp
    tip: (Header, u64),
}

impl<DB: database::DB, N: Network, VM: VMExecution> StalledChainFSM<DB, N, VM> {
    pub(crate) async fn new_with_acc(
        acc: Arc<RwLock<Acceptor<N, DB, VM>>>,
    ) -> Self {
        let tip = acc.read().await.get_curr_tip().await;

        let mut sm = Self {
            state: State::Running,
            tip: Default::default(),
            acc,
        };

        sm.update_tip(tip.inner().header());
        sm
    }

    /// Attempts to reset the FSM state, if tip has changed
    pub(crate) fn reset(&mut self, tip: &Header) -> Result<()> {
        if self.tip.0.hash != tip.hash {
            // Tip has changed, which means a new block is accepted either due
            // to normal block acceptance or fallback execution
            self.update_tip(tip);
            self.state_transition(State::Running);

            return Ok(());
        }

        Err(anyhow!("Tip has not changed"))
    }

    /// Handles heartbeat event
    pub(crate) async fn on_heartbeat_event(&mut self) {
        trace!(event = "chain.heartbeat",);

        match &self.state {
            State::Running => self.on_running().await,
            State::Stalled(_) => self.on_heartbeat_in_stalled().await,
            State::StalledOnFork(_, _) => warn!("Stalled on fork"),
        }
    }

    /// Handles block received event
    ///
    /// Returns the new state of the FSM after processing the block
    pub(crate) async fn on_block_received(&mut self, blk: &Block) -> &State {
        trace!(
            event = "chain.block_received",
            hash = to_str(&blk.header().hash),
            height = blk.header().height,
            iter = blk.header().iteration,
        );

        let tip = self
            .acc
            .read()
            .await
            .get_curr_tip()
            .await
            .inner()
            .header()
            .clone();

        let _ = self.reset(&tip);

        let curr = &self.state;
        match curr {
            State::Running => self.on_running().await,
            State::Stalled(_) => {
                if let Err(err) = self.on_stalled(blk).await {
                    error!("Error while processing block: {:?}", err);
                }
            }
            State::StalledOnFork(_, _) => warn!("Stalled on fork"),
        };

        &self.state
    }

    /// Handles a running state
    async fn on_running(&mut self) {
        if self.tip.1 + ACCEPT_TIMEOUT < node_data::get_current_timestamp() {
            // While we are still receiving blocks, no block
            // has been accepted for a long time (tip has not changed
            // recently)
            let _ = self.request_missing_blocks().await.map_err(|e| {
                error!("Error in request_missing_blocks: {:?}", e);
            });

            self.state_transition(State::Stalled(
                node_data::get_current_timestamp(),
            ));
        }
    }

    /// Handles block from wire in the `Stalled` state
    async fn on_stalled(&mut self, new_blk: &Block) -> Result<()> {
        if new_blk.header().height > self.tip.0.height {
            // Block is newer than the local tip block
            return Ok(());
        }

        let db: Arc<RwLock<DB>> = self.acc.read().await.db.clone();
        let exists = db
            .read()
            .await
            .view(|t| t.get_block_exists(&new_blk.header().hash))?;

        if exists {
            // Block already exists in ledger
            return Ok(());
        }

        let local_blk = db
            .read()
            .await
            .view(|t| t.fetch_block_by_height(new_blk.header().height))?
            .expect("local block should exist");

        let remote_blk = new_blk;

        // If we are here, this might be a block from the main
        // branch
        let res = self
            .acc
            .read()
            .await
            .verify_header_against_local(
                local_blk.header(),
                remote_blk.header(),
            )
            .await;

        if let Err(err) = res {
            error!(
                event = "recovery failed",
                local_hash = to_str(&local_blk.header().hash),
                local_height = local_blk.header().height,
                remote_hash = to_str(&remote_blk.header().hash),
                remote_height = remote_blk.header().height,
                err = format!("verification err: {:?}", err)
            );
        } else {
            self.state_transition(State::StalledOnFork(
                local_blk.header().hash,
                Box::new(remote_blk.clone()),
            ));
            return Ok(());
        }

        Ok(())
    }

    async fn on_heartbeat_in_stalled(&mut self) {
        if let State::Stalled(timestamp) = self.state {
            if timestamp + STALLED_TIMEOUT < node_data::get_current_timestamp()
            {
                let _ = self.request_missing_blocks().await.map_err(|e| {
                    error!("Error in request_missing_blocks: {:?}", e);
                });

                self.state_transition(State::Stalled(
                    node_data::get_current_timestamp(),
                ));
            }
        }
    }

    /// Requests missing blocks since last finalized block
    async fn request_missing_blocks(&self) -> Result<()> {
        let (last_final, _) = self.last_final_block().await?;
        let locator = last_final;

        let network = self.acc.read().await.network.clone();
        network
            .read()
            .await
            .send_to_alive_peers(payload::GetBlocks::new(locator).into(), 8)
            .await?;

        Ok(())
    }

    fn update_tip(&mut self, tip: &Header) {
        self.tip.0 = tip.clone();
        self.tip.1 = node_data::get_current_timestamp();
    }

    /// Changes curr state and logs the transition event
    fn state_transition(&mut self, state: State) -> &State {
        if state == self.state {
            return &self.state;
        }

        self.state = state;

        let state_str: String = match &self.state {
            State::Running => "running".to_string(),
            State::Stalled(timestamp) => {
                format!("stalled at {}", timestamp)
            }
            State::StalledOnFork(hash, _) => {
                format!("stalled_on_fork at {}", to_str(hash))
            }
        };

        let hdr = &self.tip.0;
        info!(
            event = format!("chain.{}", state_str),
            tip_hash = to_str(&hdr.hash),
            tip_height = hdr.height,
            tip_iter = hdr.iteration,
            tip_updated_at = self.tip.1,
        );

        &self.state
    }

    async fn last_final_block(&self) -> Result<([u8; 32], u64)> {
        let hdr = self
            .acc
            .read()
            .await
            .get_last_final_block()
            .await?
            .header()
            .clone();

        Ok((hdr.hash, hdr.height))
    }
}
