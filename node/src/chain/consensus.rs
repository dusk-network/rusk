// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::database::{Candidate, Ledger, Mempool};
use crate::{database, vm, Network};
use crate::{LongLivedService, Message};
use anyhow::bail;
use async_trait::async_trait;
use dusk_consensus::commons::{ConsensusError, Database, RoundUpdate};
use dusk_consensus::consensus::Consensus;
use dusk_consensus::contract_state::{
    CallParams, Error, Operations, Output, StateRoot, VerificationOutput,
};
use dusk_consensus::user::provisioners::Provisioners;
use node_data::ledger::{Block, Hash, Transaction};
use node_data::message::payload::{self, GetCandidate};
use node_data::message::AsyncQueue;
use node_data::message::{Payload, Topics};
use node_data::Serializable;
use tokio::sync::{oneshot, Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{error, info, trace};

use std::sync::Arc;
use std::{any, vec};

/// Consensus Service Task is responsible for running the consensus layer.
///
/// It manages consensus lifecycle and provides a way to interact with it.
pub(crate) struct Task {
    pub(crate) agreement_inbound: AsyncQueue<Message>,
    pub(crate) main_inbound: AsyncQueue<Message>,
    pub(crate) outbound: AsyncQueue<Message>,
    pub(crate) result: AsyncQueue<Result<Block, ConsensusError>>,

    /// a pair of join_handle and cancel_chan of the running consensus task.
    ///
    /// None means no consensus is running,
    running_task: Option<(JoinHandle<u64>, oneshot::Sender<i32>)>,

    /// task id a counter to track consensus tasks
    task_id: u64,

    /// Loaded Consensus keys
    pub keys: (dusk_bls12_381_sign::SecretKey, node_data::bls::PublicKey),
}

impl Task {
    /// Creates a new consensus task with the given keys encrypted with password
    /// from env var DUSK_CONSENSUS_KEYS_PASS.
    pub(crate) fn new_with_keys(path: String) -> Self {
        let pwd = std::env::var("DUSK_CONSENSUS_KEYS_PASS")
            .expect("DUSK_CONSENSUS_KEYS_PASS not set");
        info!(event = "loading consensus keys", path = path);
        let keys = node_data::bls::load_keys(path, pwd);

        info!(
            event = "loaded consensus keys",
            pubkey = format!("{:?}", keys.1)
        );

        Self {
            agreement_inbound: AsyncQueue::default(),
            main_inbound: AsyncQueue::default(),
            outbound: AsyncQueue::default(),
            result: AsyncQueue::default(),
            running_task: None,
            task_id: 0,
            keys,
        }
    }

    pub(crate) fn spawn<D: database::DB, VM: vm::VMExecution, N: Network>(
        &mut self,
        most_recent_block: &node_data::ledger::Header,
        provisioners: &Provisioners,
        db: &Arc<RwLock<D>>,
        vm: &Arc<RwLock<VM>>,
        network: &Arc<RwLock<N>>,
    ) {
        let mut c = Consensus::new(
            self.main_inbound.clone(),
            self.outbound.clone(),
            self.agreement_inbound.clone(),
            self.outbound.clone(),
            Arc::new(Mutex::new(Executor::new(db, vm))),
            Arc::new(Mutex::new(CandidateDB::new(db.clone(), network.clone()))),
        );

        let ru = RoundUpdate {
            round: most_recent_block.height + 1,
            seed: most_recent_block.seed,
            hash: most_recent_block.hash,
            timestamp: most_recent_block.timestamp,
            secret_key: self.keys.0,
            pubkey_bls: self.keys.1.clone(),
        };

        self.task_id += 1;

        let (all_num, eligible_num) =
            provisioners.get_provisioners_info(ru.round);

        info!(
            event = "spawn consensus",
            id = self.task_id,
            round = ru.round,
            all = all_num,           // all provisioners count
            eligible = eligible_num  // eligible provisioners count
        );

        let id = self.task_id;
        let mut result_queue = self.result.clone();
        let provisioners = provisioners.clone();
        let (cancel_tx, cancel_rx) = oneshot::channel::<i32>();

        self.running_task = Some((
            tokio::spawn(async move {
                result_queue
                    .send(c.spin(ru, provisioners, cancel_rx).await)
                    .await;

                trace!("terminate consensus task: {}", id);
                id
            }),
            cancel_tx,
        ));
    }

    /// Aborts the running consensus task and waits for its termination.
    pub(crate) async fn abort_with_wait(&mut self) {
        if let Some((handle, cancel_chan)) = self.running_task.take() {
            cancel_chan.send(0);
            handle.await;
        }
    }

    pub(crate) fn abort(&mut self) {
        if let Some((handle, cancel_chan)) = self.running_task.take() {
            cancel_chan.send(0);
        }
    }
}

#[derive(Debug, Default)]
/// Implements dusk_consensus Database trait to store candidate blocks in the
/// RocksDB storage.
pub struct CandidateDB<DB: database::DB, N: Network> {
    db: Arc<RwLock<DB>>,
    network: Arc<RwLock<N>>,
}

impl<DB: database::DB, N: Network> CandidateDB<DB, N> {
    pub fn new(db: Arc<RwLock<DB>>, network: Arc<RwLock<N>>) -> Self {
        Self { db, network }
    }
}

#[async_trait]
impl<DB: database::DB, N: Network> dusk_consensus::commons::Database
    for CandidateDB<DB, N>
{
    fn store_candidate_block(&mut self, b: Block) {
        tracing::trace!("store candidate block: {:?}", b);

        if let Ok(db) = self.db.try_read() {
            db.update(|t| t.store_candidate_block(b));
        }
    }

    /// Makes attempts to fetch a candidate block from either the local storage
    /// or the network.
    async fn get_candidate_block_by_hash(
        &self,
        h: &Hash,
    ) -> anyhow::Result<Block> {
        // Make an attempt to fetch the candidate block from local storage
        let res = self.db.read().await.view(|t| t.fetch_candidate_block(h))?;

        if let Some(b) = res {
            return Ok(b);
        }

        const RECV_PEERS_COUNT: usize = 5;
        const TIMEOUT_MILLIS: u64 = 1000;

        // For redundancy reasons, we send the GetCandidate request to multiple
        // network peers
        let request = Message::new_get_candidate(GetCandidate { hash: *h });
        let res = self
            .network
            .write()
            .await
            .send_and_wait(
                &request,
                Topics::Candidate,
                TIMEOUT_MILLIS,
                RECV_PEERS_COUNT,
            )
            .await?;

        match res.payload {
            Payload::CandidateResp(cr) => {
                let b = cr.candidate;

                // Ensure that the received candidate block is the one we
                // requested
                if b.header.hash != *h {
                    return Err(anyhow::anyhow!(
                        "incorrect candidate block hash"
                    ));
                }

                tracing::info!(
                    "received candidate height: {:?}  hash: {:?}",
                    b.header.height,
                    hex::ToHex::encode_hex::<String>(&b.header.hash)
                );

                Ok(b)
            }
            _ => Err(anyhow::anyhow!("couldn't get candidate block")),
        }
    }

    fn delete_candidate_blocks(&mut self) {
        if let Ok(db) = self.db.try_read() {
            db.update(|t| t.clear_candidates());
        }
    }
}

/// Implements Executor trait to mock Contract Storage calls.
pub struct Executor<DB: database::DB, VM: vm::VMExecution> {
    db: Arc<RwLock<DB>>,
    vm: Arc<RwLock<VM>>,
}

impl<DB: database::DB, VM: vm::VMExecution> Executor<DB, VM> {
    fn new(db: &Arc<RwLock<DB>>, vm: &Arc<RwLock<VM>>) -> Self {
        Executor {
            db: db.clone(),
            vm: vm.clone(),
        }
    }
}

#[async_trait::async_trait]
impl<DB: database::DB, VM: vm::VMExecution> Operations for Executor<DB, VM> {
    async fn verify_state_transition(
        &self,
        params: CallParams,
        txs: Vec<Transaction>,
    ) -> Result<VerificationOutput, dusk_consensus::contract_state::Error> {
        info!("verifying state");

        let vm = self.vm.read().await;

        Ok(vm.verify_state_transition(&params, txs).map_err(|err| {
            error!("failed to call VST {}", err);
            Error::Failed
        })?)
    }

    async fn execute_state_transition(
        &self,
        params: CallParams,
    ) -> Result<Output, Error> {
        info!("executing state transition");
        let vm = self.vm.read().await;

        let db = self.db.read().await;
        let (executed_txs, discarded_txs, verification_output) = db
            .view(|view| {
                let txs = view.get_txs_sorted_by_fee().map_err(|err| {
                    anyhow::anyhow!("failed to get mempool txs: {}", err)
                })?;
                let ret = vm.execute_state_transition(params, txs).map_err(
                    |err| anyhow::anyhow!("failed to call EST {}", err),
                )?;
                Ok(ret)
            })
            .map_err(|err: anyhow::Error| {
                error!("{err}");
                Error::Failed
            })?;

        Ok(Output {
            txs: executed_txs,
            verification_output,
            discarded_txs,
            provisioners: Provisioners::default(),
        })
    }
}
