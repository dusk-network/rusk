// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node_data::ledger::Block;
use std::collections::BTreeMap;

const CONSECUTIVE_BLOCKS_THRESHOLD: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum State {
    /// Blocks are being accepted
    Running,
    /// No block has been accepted recently
    ///
    /// A chain progress may be stalled for any reasons listed below:
    /// - `disconnected`  Node is not receiving any messages from the network
    /// - `open_consensus` Network is struggling to produce a block for many
    ///   iterations
    /// - `higher_iteration_branch` Network has moved forward with block of a
    ///   higher iteration
    Stalled,
    /// Node is disconnected from the main branch
    StalledOnFork,
}

/// Implements a simple FSM to detect a stalled state of the chain
pub(crate) struct StalledChainFSM {
    state: State,
    recovery_blocks: BTreeMap<u64, Block>,

    latest_finalized: Option<Block>,
    tip: Option<Block>,
}

impl StalledChainFSM {
    pub(crate) fn new() -> Self {
        Self {
            state: State::Running,
            recovery_blocks: BTreeMap::new(),
            latest_finalized: None,
            tip: None,
        }
    }

    pub(crate) fn on_block_received(&mut self, blk: &Block) -> State {
        let curr = self.state;
        match curr {
            State::Stalled => self.on_stalled(blk),
            State::StalledOnFork | State::Running => curr,
        }
    }

    /// Returns recovery blocks as a vector
    pub(crate) fn recovery_blocks(&self) -> Vec<Block> {
        self.recovery_blocks.values().cloned().collect()
    }

    /// Handles block from wire in the `Stalled` state
    pub(crate) fn on_stalled(&mut self, blk: &Block) -> State {
        let key = blk.header().height;
        self.recovery_blocks
            .entry(key)
            .or_insert_with(|| blk.clone());

        if self.recovery_blocks.len() < CONSECUTIVE_BLOCKS_THRESHOLD {
            return State::Stalled;
        }

        // Check recovery blocks contains at most N consecutive blocks
        let mut prev = self
            .latest_finalized
            .as_ref()
            .map(|b| b.header().height)
            .unwrap_or(0); // TODO:

        let consecutive = self.recovery_blocks.keys().all(|&key| {
            let is_consecutive = key == prev + 1;
            prev = key;
            is_consecutive
        });

        if !consecutive {
            // Not enough consecutive blocks collected yet
            return State::Stalled;
        }

        // Detect if collected blocks are valid
        if self
            .recovery_blocks
            .iter()
            .all(|(_, blk)| self.dry_run_accept_block(blk))
        {
            State::StalledOnFork
        } else {
            State::Stalled
        }
    }

    pub(crate) fn on_block_accepted(&mut self, blk: &Block, is_final: bool) {
        self.state = State::Running;
        self.recovery_blocks.clear();

        if is_final {
            self.latest_finalized = Some(blk.clone());
        }

        self.tip = Some(blk.clone());
    }

    pub(crate) fn on_accept_block_timeout(&mut self) {
        self.state = State::Stalled;
    }

    pub(crate) fn dry_run_accept_block(&self, _blk: &Block) -> bool {
        // TODO: Implement dry-run accept block
        false
    }
}
