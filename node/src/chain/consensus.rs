// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::database::{self, Candidate, Mempool, Metadata};
use crate::{vm, Message};
use async_trait::async_trait;
use dusk_consensus::commons::{ConsensusError, RoundUpdate, TimeoutSet};
use dusk_consensus::consensus::Consensus;
use dusk_consensus::operations::{
    self, CallParams, Error, Operations, Output, VerificationOutput, Voter,
};
use dusk_consensus::queue::MsgRegistry;
use dusk_consensus::user::provisioners::ContextProvisioners;
use node_data::ledger::{Block, Fault, Header};
use node_data::message::AsyncQueue;

use tokio::sync::{oneshot, Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, info, trace, warn};

use crate::chain::header_validation::Validator;
use crate::chain::metrics::AverageElapsedTime;
use crate::database::rocksdb::{
    MD_AVG_PROPOSAL, MD_AVG_RATIFICATION, MD_AVG_VALIDATION,
};
use metrics::gauge;
use node_data::{ledger, Serializable, StepName};
use std::sync::Arc;
use std::time::Duration;

/// Consensus Service Task is responsible for running the consensus layer.
///
/// It manages consensus lifecycle and provides a way to interact with it.
pub(crate) struct Task {
    pub(crate) main_inbound: AsyncQueue<Message>,
    pub(crate) outbound: AsyncQueue<Message>,

    pub(crate) future_msg: Arc<Mutex<MsgRegistry<Message>>>,

    pub(crate) result: AsyncQueue<Result<(), ConsensusError>>,

    /// a pair of join_handle and cancel_chan of the running consensus task.
    ///
    /// None means no consensus is running,
    running_task: Option<(JoinHandle<u64>, oneshot::Sender<i32>)>,

    /// task id a counter to track consensus tasks
    task_id: u64,

    /// Loaded Consensus keys
    pub keys: (execution_core::BlsSecretKey, node_data::bls::PublicKey),
}

impl Task {
    /// Creates a new consensus task with the given keys encrypted with password
    /// from env var DUSK_CONSENSUS_KEYS_PASS.
    pub(crate) fn new_with_keys(
        path: String,
        max_inbound_size: usize,
    ) -> anyhow::Result<Self> {
        let pwd = std::env::var("DUSK_CONSENSUS_KEYS_PASS")
            .map_err(|_| anyhow::anyhow!("DUSK_CONSENSUS_KEYS_PASS not set"))?;
        info!(event = "loading consensus keys", path = path);
        let keys = node_data::bls::load_keys(path, pwd)?;

        info!(
            event = "loaded consensus keys",
            pubkey = format!("{:?}", keys.1)
        );

        Ok(Self {
            main_inbound: AsyncQueue::bounded(
                max_inbound_size,
                "consensus_inbound",
            ),
            outbound: AsyncQueue::bounded(
                max_inbound_size,
                "consensus_outbound",
            ),
            future_msg: Arc::new(Mutex::new(MsgRegistry::default())),
            result: AsyncQueue::bounded(1, "consensus_result"),
            running_task: None,
            task_id: 0,
            keys,
        })
    }

    pub(crate) fn spawn<D: database::DB, VM: vm::VMExecution>(
        &mut self,
        tip: &node_data::ledger::Block,
        provisioners_list: ContextProvisioners,
        db: &Arc<RwLock<D>>,
        vm: &Arc<RwLock<VM>>,
        base_timeout: TimeoutSet,
        voters: Vec<Voter>,
    ) {
        let current = provisioners_list.to_current();
        let consensus_task = Consensus::new(
            self.main_inbound.clone(),
            self.outbound.clone(),
            self.future_msg.clone(),
            Arc::new(Executor::new(
                db,
                vm,
                tip.header().clone(),
                provisioners_list, // TODO: Avoid cloning
            )),
            Arc::new(Mutex::new(CandidateDB::new(db.clone()))),
        );

        let ru = RoundUpdate::new(
            self.keys.1.clone(),
            self.keys.0.clone(),
            tip.header(),
            base_timeout.clone(),
            voters,
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

        gauge!("dusk_provisioners_eligible").set(eligible_num as f64);
        gauge!("dusk_provisioners_all").set(all_num as f64);

        let id = self.task_id;
        let resp = self.result.clone();
        let (cancel_tx, cancel_rx) = oneshot::channel::<i32>();

        self.running_task = Some((
            tokio::spawn(async move {
                // Run the consensus task
                let res =
                    consensus_task.spin(ru, current.into(), cancel_rx).await;

                // Notify chain component about the consensus result
                resp.try_send(res);

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
                trace!("Unable to send cancel for abort_with_wait")
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

    pub(crate) fn is_running(&self) -> bool {
        self.running_task.is_some()
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

#[async_trait]
impl<DB: database::DB> dusk_consensus::commons::Database for CandidateDB<DB> {
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
    tip_header: ledger::Header,
    provisioners: ContextProvisioners,
}

impl<DB: database::DB, VM: vm::VMExecution> Executor<DB, VM> {
    fn new(
        db: &Arc<RwLock<DB>>,
        vm: &Arc<RwLock<VM>>,
        tip_header: ledger::Header,
        provisioners: ContextProvisioners,
    ) -> Self {
        Executor {
            db: db.clone(),
            vm: vm.clone(),
            tip_header,
            provisioners,
        }
    }
}

#[async_trait::async_trait]
impl<DB: database::DB, VM: vm::VMExecution> Operations for Executor<DB, VM> {
    async fn verify_block_header(
        &self,
        candidate_header: &Header,
        disable_winning_att_check: bool,
    ) -> Result<(u8, Vec<Voter>, Vec<Voter>), Error> {
        let validator = Validator::new(
            self.db.clone(),
            &self.tip_header,
            &self.provisioners,
        );

        validator
            .execute_checks(candidate_header, disable_winning_att_check)
            .await
            .map_err(operations::Error::InvalidHeader)
    }

    async fn verify_faults(
        &self,
        block_height: u64,
        faults: &[Fault],
    ) -> Result<(), Error> {
        let validator = Validator::new(
            self.db.clone(),
            &self.tip_header,
            &self.provisioners,
        );
        Ok(validator.verify_faults(block_height, faults).await?)
    }

    async fn verify_state_transition(
        &self,
        blk: &Block,
        voters: &[Voter],
    ) -> Result<VerificationOutput, dusk_consensus::operations::Error> {
        info!("verifying state");

        let vm = self.vm.read().await;

        vm.verify_state_transition(blk, Some(voters))
            .map_err(operations::Error::InvalidVST)
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
            .map_err(Error::InvalidEST)?;
        let _ = db.update(|m| {
            for t in &discarded_txs {
                let _ = m.delete_tx(t.id());
            }
            Ok(())
        });

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
            StepName::Proposal => MD_AVG_PROPOSAL,
            StepName::Validation => MD_AVG_VALIDATION,
            StepName::Ratification => MD_AVG_RATIFICATION,
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
            .map_err(Error::MetricsUpdate)?;

        Ok(())
    }
}
