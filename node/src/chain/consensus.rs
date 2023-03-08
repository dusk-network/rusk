// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::database::{Candidate, Ledger};
use crate::{database, Network};
use crate::{LongLivedService, Message};
use async_trait::async_trait;
use dusk_consensus::commons::{ConsensusError, Database, RoundUpdate};
use dusk_consensus::consensus::Consensus;
use dusk_consensus::contract_state::{
    CallParams, Error, Operations, Output, StateRoot,
};
use dusk_consensus::user::provisioners::Provisioners;
use node_data::ledger::{Block, Hash};
use node_data::message::AsyncQueue;
use node_data::message::{Payload, Topics};
use node_data::Serializable;
use tokio::sync::{oneshot, Mutex, RwLock};
use tokio::task::JoinHandle;

use std::any;
use std::sync::Arc;

/// Consensus Service Task is responsible for running the consensus layer.
///
/// It manages consensus lifecycle and provides a way to interact with it.
pub(crate) struct Task {
    pub(crate) agreement_inbound: AsyncQueue<Message>,
    pub(crate) main_inbound: AsyncQueue<Message>,
    pub(crate) outbound: AsyncQueue<Message>,
    pub(crate) result: AsyncQueue<Result<Block, ConsensusError>>,

    /// join_handle of the running consensus tokio task.
    ///
    /// None If no consensus is running,
    task_handle: Option<JoinHandle<u64>>,
    task_id: u64,

    /// Loaded Consensus keys
    keys: (dusk_bls12_381_sign::SecretKey, node_data::bls::PublicKey),
}

impl Task {
    /// Creates a new consensus task with the given keys encrypted with password
    /// from env var DUSK_CONSENSUS_KEYS_PASS.
    pub(crate) fn new_with_keys(path: String) -> Self {
        let pwd = std::env::var("DUSK_CONSENSUS_KEYS_PASS").unwrap();
        let keys = node_data::bls::load_keys(path, pwd);

        tracing::info!("Loaded consensus keys: {:?}", keys.1);

        Self {
            agreement_inbound: AsyncQueue::default(),
            main_inbound: AsyncQueue::default(),
            outbound: AsyncQueue::default(),
            result: AsyncQueue::default(),
            task_handle: None,
            task_id: 0,
            keys,
        }
    }

    /// Aborts the running consensus task
    pub(crate) fn abort(&mut self) {
        if let Some(h) = self.task_handle.take() {
            h.abort()
        }
    }

    pub(crate) fn spawn<D: database::DB>(
        &mut self,
        most_recent_block: &node_data::ledger::Header,
        provisioners: &Provisioners,
        db: &Arc<RwLock<D>>,
    ) {
        let mut c = Consensus::new(
            self.main_inbound.clone(),
            self.outbound.clone(),
            self.agreement_inbound.clone(),
            self.outbound.clone(),
            Arc::new(Mutex::new(Executor {})),
            Arc::new(Mutex::new(CandidateDB::new(db.clone()))),
        );

        let round_update = RoundUpdate {
            round: most_recent_block.height + 1,
            seed: most_recent_block.seed,
            hash: most_recent_block.hash,
            timestamp: most_recent_block.timestamp,
            secret_key: self.keys.0,
            pubkey_bls: self.keys.1.clone(),
        };

        self.task_id += 1;
        tracing::trace!("spawn consensus task: {}", self.task_id);

        let id = self.task_id;
        let result_queue = self.result.clone();
        let provisioners = provisioners.clone();

        let layer_handle = tokio::spawn(async move {
            let (_cancel_tx, cancel_rx) = oneshot::channel::<i32>();

            result_queue
                .try_send(c.spin(round_update, provisioners, cancel_rx).await);

            id
        });

        self.task_handle = Some(layer_handle);
    }
}

#[derive(Debug, Default)]
/// Implements dusk_consensus Database trait to store candidate blocks in the
/// RocksDB storage.
pub struct CandidateDB<DB: database::DB> {
    db: Arc<RwLock<DB>>,
}

impl<DB: database::DB> CandidateDB<DB> {
    pub fn new(db: Arc<RwLock<DB>>) -> Self {
        Self { db }
    }
}

impl<DB: database::DB> dusk_consensus::commons::Database for CandidateDB<DB> {
    fn store_candidate_block(&mut self, b: Block) {
        tracing::trace!("store candidate block: {:?}", b);

        if let Ok(db) = self.db.try_read() {
            db.update(|t| t.store_candidate_block(b));
        }
    }

    fn get_candidate_block_by_hash(&self, h: &Hash) -> Option<Block> {
        let mut res = Option::None;
        if let Ok(db) = self.db.try_read() {
            db.view(|t| {
                res = t.fetch_candidate_block(h)?;
                Ok(())
            });
        }

        res
    }

    fn delete_candidate_blocks(&mut self) {
        if let Ok(db) = self.db.try_read() {
            db.update(|t| t.clear_candidates());
        }
    }
}

/// Implements Executor trait to mock Contract Storage calls.
pub struct Executor {}
impl Operations for Executor {
    fn verify_state_transition(
        &self,
        _params: CallParams,
    ) -> Result<StateRoot, dusk_consensus::contract_state::Error> {
        tracing::info!("verifying state");
        Ok([0; 32])
    }

    fn execute_state_transition(
        &self,
        _params: CallParams,
    ) -> Result<Output, Error> {
        tracing::info!("executing state transition");
        Ok(Output::default())
    }

    fn accept(&self, _params: CallParams) -> Result<Output, Error> {
        tracing::info!("accepting new state");
        Ok(Output::default())
    }

    fn finalize(&self, _params: CallParams) -> Result<Output, Error> {
        tracing::info!("finalizing new state");
        Ok(Output::default())
    }

    fn get_state_root(&self) -> Result<StateRoot, Error> {
        Ok([0; 32])
    }
}
