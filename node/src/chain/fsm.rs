// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::{
    acceptor::BlockAcceptor, consensus, genesis, sequencer::Sequencer,
};
use crate::database::{self, Ledger};
use crate::{vm, Network};
use dusk_consensus::user::provisioners::Provisioners;
use node_data::ledger::{self, Block, Hash, Transaction};
use node_data::message::Message;
use std::time::Duration;
use std::{
    cell::RefCell, marker::PhantomData, rc::Rc, sync::Arc, time::SystemTime,
};
use tokio::sync::RwLock;

type SharedBlock = Rc<RefCell<Block>>;

enum State<N: Network, DB: database::DB, VM: vm::VMExecution> {
    InSync(InSyncImpl<DB, VM, N>),
    OutOfSync(OutOfSyncImpl),
    InFallback,
}

/// Implements a finite-state-machine to manage InSync, OutOfSync and
/// InFallback states.
pub(crate) struct SimpleFSM<N: Network, DB: database::DB, VM: vm::VMExecution> {
    curr: State<N, DB, VM>,

    /// Most recently accepted block a.k.a blockchain tip
    mrb: SharedBlock,

    /// List of eligible provisioners of actual round
    eligible_provisioners: Provisioners,

    /// Upper layer consensus task
    upper: super::consensus::Task,

    network: Arc<RwLock<N>>,
    db: Arc<RwLock<DB>>,
    vm: Arc<RwLock<VM>>,
}

impl<N: Network, DB: database::DB, VM: vm::VMExecution> SimpleFSM<N, DB, VM> {
    fn new(
        keys_path: String,
        network: Arc<RwLock<N>>,
        db: Arc<RwLock<DB>>,
        vm: Arc<RwLock<VM>>,
    ) -> Self {
        Self {
            curr: State::InSync(InSyncImpl::<DB, VM, N>::default()),
            mrb: Rc::new(RefCell::new(Block::default())),
            eligible_provisioners: Provisioners::default(),
            upper: consensus::Task::new_with_keys(keys_path),
            network,
            db,
            vm,
        }
    }

    async fn init(&mut self, vm: &Arc<RwLock<VM>>) -> anyhow::Result<usize> {
        let (genesis_block, eligible_provisioners) = genesis::generate_state();

        *self.mrb.borrow_mut() = genesis_block;
        self.eligible_provisioners = eligible_provisioners;

        self.upper.spawn(
            &(*self.mrb.borrow()).header,
            &self.eligible_provisioners,
            &self.db,
            &self.vm,
            &self.network,
        );

        // Store genesis block
        self.db
            .read()
            .await
            .update(|t| t.store_block(&self.mrb.borrow(), true));

        // Always request missing blocks on startup.
        //
        // Instead of waiting for next block to be produced and delivered, we
        // request missing blocks from randomly selected the network nodes.
        self.request_missing_blocks(3).await?;

        anyhow::Ok(0)
    }

    pub async fn on_event(&mut self, blk: Block) -> anyhow::Result<()> {
        match &mut self.curr {
            State::InSync(ref mut curr) => {
                if curr.on_event(&blk).await? {
                    /// Transition from InSync to OutOfSync state
                    curr.on_exiting();

                    // Enter new state
                    let mut next = OutOfSyncImpl::new(self.mrb.clone());
                    next.on_entering(&blk).await;
                    self.curr = State::OutOfSync(next);
                }
            }
            State::OutOfSync(ref mut curr) => {
                if curr.on_event(&blk).await? {
                    /// Transition from OutOfSync to InSync  state
                    curr.on_exiting();

                    // Enter new state
                    let mut next = InSyncImpl::new(self.mrb.clone());
                    next.on_entering(&blk).await;
                    self.curr = State::InSync(next);
                }
            }
            State::InFallback => {
                // TODO: This will be handled with another issue
            }
        }

        Ok(())
    }

    /// Requests missing blocks by sending GetBlocks wire message to N alive
    /// peers.
    async fn request_missing_blocks(
        &self,
        peers_count: usize,
    ) -> anyhow::Result<()> {
        let locator = self.mrb.borrow().header.hash;
        // GetBlocks
        let msg =
            Message::new_get_blocks(node_data::message::payload::GetBlocks {
                locator,
            });

        self.network
            .read()
            .await
            .send_to_alive_peers(&msg, peers_count)
            .await;

        Ok(())
    }
}

struct InSyncImpl<DB: database::DB, VM: vm::VMExecution, N: Network> {
    acceptor: BlockAcceptor<DB, VM, N>,
    mrb: SharedBlock,
}

impl<DB: database::DB, VM: vm::VMExecution, N: Network> InSyncImpl<DB, VM, N> {
    fn new(mrb: SharedBlock, acc: BlockAcceptor<DB, VM, N>) -> Self {
        Self { mrb }
    }

    /// performed when entering the state
    async fn on_entering(&mut self, blk: &Block) {
        let h = blk.header.height;

        // Try accepting consecutive block
        if h == self.mrb.borrow().header.height + 1 {
            /* TODO:
            self.accept_block::<DB, VM, N>(&db, &vm, &network, blk)
                .await?;

            network.read().await.broadcast(msg).await;
            */
        }
    }

    ///  performed when exiting the state
    async fn on_exiting(&mut self) {}

    async fn on_event(&mut self, blk: &Block) -> anyhow::Result<bool> {
        let h = blk.header.height;

        if h <= self.mrb.borrow().header.height {
            // Surpress errros for now.
            return Ok(false);
        }

        // Try accepting consecutive block
        if h == self.mrb.borrow().header.height + 1 {
            /* TODO:
            self.accept_block::<DB, VM, N>(&db, &vm, &network, blk)
                .await?;

            network.read().await.broadcast(msg).await;
            */

            *self.mrb.borrow_mut() = Block::default(); // TODO:
            return Ok(false);
        }

        /// TODO: If the block certficate is verifiable, verify it
        Ok(true)
    }
}

struct OutOfSyncImpl {
    mrb: SharedBlock,
    range: (u64, u64),
    start_time: SystemTime,

    pool: Sequencer,
}

impl<'a> OutOfSyncImpl {
    fn new(mrb: SharedBlock) -> Self {
        let curr_height = mrb.borrow().header.height;
        Self {
            start_time: SystemTime::now(),
            mrb,
            range: (curr_height, curr_height + 500), // TODO: 500
            pool: Sequencer::default(),
        }
    }
    ///  performed when entering the state
    async fn on_entering(&mut self, blk: &Block) {
        // request missing blocks
        // GetBlocks message

        // TODO:  use source address here
        // Self::request_missing_blocks(self.most_recent_block.header.hash, 5);

        // add to sequencer
        self.pool.add(blk.clone());
    }

    ///  performed when exiting the state
    async fn on_exiting(&mut self) {}

    async fn on_event(&mut self, blk: &Block) -> anyhow::Result<bool> {
        let h = blk.header.height;

        if h <= self.mrb.borrow().header.height {
            // TODO: warning
            return Ok(false);
        }

        // Try accepting consecutive block
        if h == self.mrb.borrow().header.height + 1 {
            //self.accept_block::<DB, VM, N>(&db, &vm, &network, blk)
            //    .await?;

            // TODO: Try to accept other consecutive blocks, if available
            for b in &self.pool.iter() {

                //self.accept_block::<DB, VM, N>(&db, &vm, &network, blk)
                //    .await?;
            }

            // If target height is reached the switch to InSync mode
            if h == self.range.1 {
                // Transit to InSync mode
                return Ok(true);
            }

            return Ok(false);
        }

        if self
            .start_time
            .checked_add(Duration::from_millis(1500))
            .unwrap()
            > SystemTime::now()
        {
            // Timeout-ed sync-up
            // Transit back to InSync mode
            return Ok(true);
        }

        /// add block to sequencer
        self.pool.add(blk.clone());

        Ok(false)
    }
}
