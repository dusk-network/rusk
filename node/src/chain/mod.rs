// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::data::Topics;
use crate::utils::PendingQueue;
use crate::{data, database, Network};
use crate::{LongLivedService, Message};
use async_trait::async_trait;
use dusk_consensus::commons::{ConsensusError, RoundUpdate};
use dusk_consensus::consensus::Consensus;
use dusk_consensus::contract_state::{
    CallParams, Error, Operations, Output, StateRoot,
};
use dusk_consensus::user::provisioners::Provisioners;
use dusk_consensus::util::pending_queue::PendingQueue as ConsensusPendingQueue; /* TODO: rename it or */
use node_data::ledger::{Block, Hash};
use tokio::sync::{oneshot, Mutex, RwLock};
use tokio::task::JoinHandle;

use std::any;
use std::sync::Arc;

const TOPICS: &[u8] = &[
    data::Topics::Block as u8,
    data::Topics::NewBlock as u8,
    data::Topics::Reduction as u8,
    data::Topics::AggrAgreement as u8,
    data::Topics::Agreement as u8,
];

pub struct ChainSrv {
    inbound: PendingQueue<Message>,

    upper: ConsensusLayer,
}

impl Default for ChainSrv {
    fn default() -> Self {
        Self {
            inbound: Default::default(),
            upper: ConsensusLayer {
                agreement_inbound: ConsensusPendingQueue::new("1"),
                main_inbound: ConsensusPendingQueue::new("2"),
                outbound: ConsensusPendingQueue::new("3"),
                task_handle: None,
            },
        }
    }
}

#[async_trait]
impl<N: Network, DB: database::DB> LongLivedService<N, DB> for ChainSrv {
    async fn execute(
        &mut self,
        network: Arc<RwLock<N>>,
        db: Arc<RwLock<DB>>,
    ) -> anyhow::Result<usize> {
        // Register routes
        LongLivedService::<N, DB>::add_routes(
            self,
            TOPICS,
            self.inbound.clone(),
            &network,
        )
        .await?;

        self.init::<N, DB>(&network).await?;

        loop {
            tokio::select! {
                // Receives inbound wire messages.
                // Component should either process it or re-route it to the next upper layer
                recv = &mut self.inbound.recv() => {
                    if let Ok(msg) = recv {
                        match msg.topic {
                            Topics::Block => {
                                // Try to validate message
                                if self.is_valid(&msg).is_ok() {
                                    network.read().await.repropagate(&msg, 0).await;
                                    self.handle_block_msg(&msg);

                                    // Reset Consensus
                                    self.upper.abort();
                                }
                            }

                            // Re-route message to upper layer (in this case it is the Consensus layer)
                            Topics::NewBlock | Topics::Reduction => {
                                self.upper.main_inbound.try_send(msg);
                            }
                            Topics::Agreement | Topics::AggrAgreement => {
                                self.upper.agreement_inbound.try_send(msg);
                            }
                            _ => tracing::warn!("invalid inbound message"),
                        }
                    }
                },

                // Re-routes messages originated from Consensus (upper) layer to the network layer.
                recv = &mut self.upper.outbound.recv() => {
                    if let Ok(msg) = recv {
                        network.read().await.repropagate(&msg, 0).await;
                    }
                },



            }
        }
    }

    /// Returns service name.
    fn name(&self) -> &'static str {
        "chain"
    }
}

impl ChainSrv {
    async fn init<N: Network, DB: database::DB>(
        &mut self,
        network: &Arc<RwLock<N>>,
    ) -> anyhow::Result<usize> {
        self.upper.spawn();

        anyhow::Ok(0)
    }

    fn is_valid(&self, msg: &Message) -> anyhow::Result<()> {
        // TODO:
        Ok(())
    }

    fn handle_block_msg(&mut self, msg: &Message) {
        // TODO:
    }
}

struct ConsensusLayer {
    agreement_inbound: ConsensusPendingQueue,
    main_inbound: ConsensusPendingQueue,

    outbound: ConsensusPendingQueue,

    /// Keeps a  join_handle of a running consensus tokio task.
    ///
    /// None If no consensus is running,
    task_handle: Option<JoinHandle<Result<Block, ConsensusError>>>,
}

impl ConsensusLayer {
    fn abort(&mut self) {
        let task = self.task_handle;
        self.task_handle = None;
        if let Some(v) = task {
            v.abort();
        }
    }

    fn spawn(&mut self) {
        let mut c = Consensus::new(
            self.main_inbound.clone(),
            self.outbound.clone(),
            self.agreement_inbound.clone(),
            self.outbound.clone(),
            Arc::new(Mutex::new(Executor {})),
            Arc::new(Mutex::new(SimpleDB::default())),
        );

        let round_update = RoundUpdate::default(); // TODO:
        let provisioners = Provisioners::default(); // TODO:

        let layer_handle = tokio::spawn(async move {
            let (_cancel_tx, cancel_rx) = oneshot::channel::<i32>();
            c.spin(round_update, provisioners, cancel_rx).await
        });

        self.task_handle = Some(layer_handle);
    }
}

/// Implements Executor trait to mock Contract Storage calls.
pub struct Executor {}
impl Operations for Executor {
    fn verify_state_transition(
        &self,
        _params: CallParams,
    ) -> Result<StateRoot, dusk_consensus::contract_state::Error> {
        Ok([0; 32])
    }

    fn execute_state_transition(
        &self,
        _params: CallParams,
    ) -> Result<Output, Error> {
        Ok(Output::default())
    }

    fn accept(&self, _params: CallParams) -> Result<Output, Error> {
        Ok(Output::default())
    }

    fn finalize(&self, _params: CallParams) -> Result<Output, Error> {
        Ok(Output::default())
    }

    fn get_state_root(&self) -> Result<StateRoot, Error> {
        Ok([0; 32])
    }
}

#[derive(Debug, Default)]
/// Implements Database trait to store candidates blocks in heap memory.
pub struct SimpleDB {
    // TODO: SimpleDB, database
    candidates: std::collections::BTreeMap<Hash, Block>,
}

impl dusk_consensus::commons::Database for SimpleDB {
    fn store_candidate_block(&mut self, b: Block) {
        if b.header.hash == Hash::default() {
            tracing::error!("candidate block with empty hash");
            return;
        }

        self.candidates.entry(b.header.hash).or_insert(b);
    }

    fn get_candidate_block_by_hash(&self, h: &Hash) -> Option<(Hash, Block)> {
        if let Some(v) = self.candidates.get_key_value(h) {
            return Some((*v.0, v.1.clone()));
        }
        None
    }

    fn delete_candidate_blocks(&mut self) {
        self.candidates.clear();
    }
}
