// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::database::{self, Candidate, Mempool, Metadata};
use crate::{vm, Message, Network};
use async_trait::async_trait;
use dusk_consensus::commons::{ConsensusError, RoundUpdate, TimeoutSet};
use dusk_consensus::consensus::Consensus;
use dusk_consensus::operations::{
    CallParams, Error, Operations, Output, VerificationOutput,
};
use dusk_consensus::user::provisioners::ContextProvisioners;
use node_data::ledger::{Block, Hash, Header};
use node_data::message::payload::GetCandidate;
use node_data::message::AsyncQueue;
use node_data::message::{Payload, Topics};
use tokio::sync::{oneshot, Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, trace, warn};

use crate::chain::header_validation::Validator;
use crate::chain::metrics::AverageElapsedTime;
use crate::database::rocksdb::{
    MD_AVG_PROPOSAL, MD_AVG_RATIFICATION, MD_AVG_VALIDATION,
};
use node_data::{ledger, Serializable, StepName};
use std::sync::Arc;
use std::time::Duration;

/// Consensus Service Task is responsible for running the consensus layer.
///
/// It manages consensus lifecycle and provides a way to interact with it.
pub(crate) struct Task {
    pub(crate) quorum_inbound: AsyncQueue<Message>,
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
            quorum_inbound: AsyncQueue::default(),
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
        most_recent_block: &node_data::ledger::Block,
        provisioners_list: ContextProvisioners,
        db: &Arc<RwLock<D>>,
        vm: &Arc<RwLock<VM>>,
        network: &Arc<RwLock<N>>,
        base_timeout: TimeoutSet,
    ) {
        let current = provisioners_list.to_current();
        let c = Consensus::new(
            self.main_inbound.clone(),
            self.outbound.clone(),
            self.quorum_inbound.clone(),
            self.outbound.clone(),
            Arc::new(Mutex::new(Executor::new(
                db,
                vm,
                most_recent_block.header().clone(),
                provisioners_list, // TODO: Avoid cloning
            ))),
            Arc::new(Mutex::new(CandidateDB::new(db.clone(), network.clone()))),
        );

        let ru = RoundUpdate::new(
            self.keys.1.clone(),
            self.keys.0,
            most_recent_block.header(),
            base_timeout.clone(),
        );

        self.task_id += 1;

        let (all_num, eligible_num) = current.get_provisioners_info(ru.round);

        info!(
            event = "spawn consensus",
            id = self.task_id,
            round = ru.round,
            timeout = ?base_timeout,
            all = all_num,           // all provisioners count
            eligible = eligible_num  // eligible provisioners count
        );

        let id = self.task_id;
        let result_queue = self.result.clone();
        let (cancel_tx, cancel_rx) = oneshot::channel::<i32>();

        self.running_task = Some((
            tokio::spawn(async move {
                let cons_result = c.spin(ru, current.into(), cancel_rx).await;
                if let Err(e) = result_queue.send(cons_result).await {
                    error!("Unable to send consensus result to queue {e}")
                }

                trace!("terminate consensus task: {}", id);
                id
            }),
            cancel_tx,
        ));
    }

    /// Aborts the running consensus task and waits for its termination.
    pub(crate) async fn abort_with_wait(&mut self) {
        if let Some((handle, cancel_chan)) = self.running_task.take() {
            if cancel_chan.send(0).is_err() {
                warn!("Unable to send cancel for abort_with_wait")
            }
            if let Err(e) = handle.await {
                warn!("Unable to wait for abort {e}")
            }
        }
    }

    pub(crate) fn abort(&mut self) {
        if let Some((_, cancel_chan)) = self.running_task.take() {
            if cancel_chan.send(0).is_err() {
                warn!("Unable to send cancel for abort")
            };
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

        match self.db.try_read() {
            Ok(db) => {
                if let Err(e) = db.update(|t| t.store_candidate_block(b)) {
                    warn!("Unable to store candidate block: {e}");
                };
            }
            Err(e) => {
                warn!("Cannot acquire lock to store candidate block: {e}");
            }
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

        const RECV_PEERS_COUNT: usize = 7;
        const TIMEOUT_MILLIS: u64 = 2000;

        // For redundancy reasons, we send the GetCandidate request to multiple
        // network peers
        let request = Message::new_get_candidate(GetCandidate { hash: *h });
        let res = self
            .network
            .write()
            .await
            .send_and_wait(
                &request,
                Topics::GetCandidateResp,
                TIMEOUT_MILLIS,
                RECV_PEERS_COUNT,
            )
            .await?;

        match res.payload {
            Payload::CandidateResp(cr) => {
                let b = cr.candidate;

                // Ensure that the received candidate block is the one we
                // requested
                if b.header().hash != *h {
                    return Err(anyhow::anyhow!(
                        "incorrect candidate block hash"
                    ));
                }

                tracing::info!(
                    "received candidate height: {:?}  hash: {:?}",
                    b.header().height,
                    hex::ToHex::encode_hex::<String>(&b.header().hash)
                );

                Ok(b)
            }
            _ => Err(anyhow::anyhow!("couldn't get candidate block")),
        }
    }

    fn delete_candidate_blocks(&mut self) {
        match self.db.try_read() {
            Ok(db) => {
                if let Err(e) = db.update(|t| t.clear_candidates()) {
                    warn!("Unable to cleare candidates: {e}");
                };
            }
            Err(e) => {
                warn!("Cannot acquire lock to clear_candidate: {e}");
            }
        }
    }
}

/// Implements Executor trait to mock Contract Storage calls.
pub struct Executor<DB: database::DB, VM: vm::VMExecution> {
    db: Arc<RwLock<DB>>,
    vm: Arc<RwLock<VM>>,
    mrb_header: ledger::Header,
    provisioners: ContextProvisioners,
}

impl<DB: database::DB, VM: vm::VMExecution> Executor<DB, VM> {
    fn new(
        db: &Arc<RwLock<DB>>,
        vm: &Arc<RwLock<VM>>,
        mrb_header: ledger::Header,
        provisioners: ContextProvisioners,
    ) -> Self {
        Executor {
            db: db.clone(),
            vm: vm.clone(),
            mrb_header,
            provisioners,
        }
    }
}

#[async_trait::async_trait]
impl<DB: database::DB, VM: vm::VMExecution> Operations for Executor<DB, VM> {
    async fn verify_block_header(
        &self,
        candidate_header: &Header,
        disable_winning_cert_check: bool,
    ) -> Result<(), Error> {
        let validator = Validator::new(
            self.db.clone(),
            &self.mrb_header,
            &self.provisioners,
        );

        validator
            .execute_checks(candidate_header, disable_winning_cert_check)
            .await
            .map_err(|err| {
                error!("failed to verify header {}", err);
                Error::Failed
            })?;

        Ok(())
    }

    async fn verify_state_transition(
        &self,
        blk: &Block,
    ) -> Result<VerificationOutput, dusk_consensus::operations::Error> {
        info!("verifying state");

        let vm = self.vm.read().await;

        Ok(vm.verify_state_transition(blk).map_err(|err| {
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
                let ret = vm.execute_state_transition(&params, txs).map_err(
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
        })
    }

    async fn add_step_elapsed_time(
        &self,
        _round: u64,
        step_name: StepName,
        elapsed: Duration,
    ) -> Result<(), Error> {
        let db_key = match step_name {
            StepName::Proposal => &MD_AVG_PROPOSAL[..],
            StepName::Validation => &MD_AVG_VALIDATION[..],
            StepName::Ratification => &MD_AVG_RATIFICATION[..],
        };

        let db = self.db.read().await;
        let _ = db
            .update(|t| {
                let mut metric = match &t.op_read(db_key)? {
                    Some(bytes) => AverageElapsedTime::read(&mut &bytes[..])
                        .unwrap_or_default(),
                    None => AverageElapsedTime::default(),
                };

                metric.push_back(elapsed);
                debug!(event = "avg_updated", ?step_name,  metric = ?metric);

                let mut bytes = Vec::new();
                metric.write(&mut bytes)?;

                t.op_write(db_key, bytes)
            })
            .map_err(|err: anyhow::Error| {
                error!("{err}");
                Error::Failed
            })?;

        Ok(())
    }
}
