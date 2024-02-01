// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::database::{self, Ledger, Mempool, Metadata};
use crate::{vm, Message, Network};
use anyhow::{anyhow, Result};
use dusk_consensus::commons::{ConsensusError, TimeoutSet};
use dusk_consensus::config::{
    CONSENSUS_ROLLING_FINALITY_THRESHOLD, MIN_STEP_TIMEOUT,
};
use dusk_consensus::user::provisioners::{ContextProvisioners, Provisioners};
use node_data::ledger::{
    self, to_str, Block, BlockWithLabel, Label, Seed, SpentTransaction,
};
use node_data::message::AsyncQueue;
use node_data::message::Payload;

use node_data::{Serializable, StepName};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn};

use super::consensus::Task;
use crate::chain::header_validation::Validator;
use crate::chain::metrics::AverageElapsedTime;
use crate::database::rocksdb::{
    MD_AVG_PROPOSAL, MD_AVG_RATIFICATION, MD_AVG_VALIDATION, MD_HASH_KEY,
    MD_STATE_ROOT_KEY,
};

#[allow(dead_code)]
pub(crate) enum RevertTarget {
    Commit([u8; 32]),
    LastFinalizedState,
    LastEpoch,
}

/// Implements block acceptance procedure. This includes block header,
/// certificate and transactions full verifications.
/// Acceptor also manages the initialization and lifespan of Consensus task.
pub(crate) struct Acceptor<N: Network, DB: database::DB, VM: vm::VMExecution> {
    /// Most recently accepted block a.k.a blockchain tip
    mrb: RwLock<BlockWithLabel>,

    /// Provisioners needed to verify next block
    pub(crate) provisioners_list: RwLock<ContextProvisioners>,

    /// Upper layer consensus task
    task: RwLock<super::consensus::Task>,

    pub(crate) db: Arc<RwLock<DB>>,
    pub(crate) vm: Arc<RwLock<VM>>,
    network: Arc<RwLock<N>>,
}

impl<DB: database::DB, VM: vm::VMExecution, N: Network> Drop
    for Acceptor<N, DB, VM>
{
    fn drop(&mut self) {
        if let Ok(mut t) = self.task.try_write() {
            t.abort()
        }
    }
}

const EPOCH: u64 = 2160;
const STAKE_CONTRACT: [u8; 32] = stake_contract_id();
const fn stake_contract_id() -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[0] = 2;
    bytes
}

impl<DB: database::DB, VM: vm::VMExecution, N: Network> Acceptor<N, DB, VM> {
    /// Initializes a new `Acceptor` struct,
    ///
    /// The method loads the VM state, detects consistency issues between VM and
    /// Ledger states, and may revert to the last known finalized state in
    /// case of inconsistency.
    /// Finally it spawns a new consensus [`Task`]
    pub async fn init_consensus(
        keys_path: &str,
        mrb: BlockWithLabel,
        provisioners_list: Provisioners,
        db: Arc<RwLock<DB>>,
        network: Arc<RwLock<N>>,
        vm: Arc<RwLock<VM>>,
    ) -> anyhow::Result<Self> {
        let mrb_height = mrb.inner().header().height;
        let mrb_state_hash = mrb.inner().header().state_hash;

        let mut provisioners_list = ContextProvisioners::new(provisioners_list);

        if mrb.inner().header().height > 0 {
            let (prev_header, _) = db
                .read()
                .await
                .view(|t| {
                    t.fetch_block_header(&mrb.inner().header().prev_block_hash)
                })?
                .expect("Previous block to be found");

            let prev_provisioners =
                vm.read().await.get_provisioners(prev_header.state_hash)?;
            provisioners_list.set_previous(prev_provisioners);
        }

        let acc = Self {
            mrb: RwLock::new(mrb),
            provisioners_list: RwLock::new(provisioners_list),
            db: db.clone(),
            vm: vm.clone(),
            network: network.clone(),
            task: RwLock::new(Task::new_with_keys(keys_path.to_string())),
        };

        // NB. After restart, state_root returned by VM is always the last
        // finalized one.
        let state_root = vm.read().await.get_state_root()?;

        info!(
            event = "VM state loaded",
            state_root = hex::encode(state_root),
        );

        // Detect a consistency issue between VM and Ledger states.
        if mrb_height > 0 && mrb_state_hash != state_root {
            info!("revert to last finalized state");
            // Revert to last known finalized state.
            acc.try_revert(RevertTarget::LastFinalizedState).await?;
        }

        acc.spawn_task().await;
        Ok(acc)
    }

    async fn spawn_task(&self) {
        let provisioners_list = self.provisioners_list.read().await.clone();
        let base_timeouts = self.adjust_round_base_timeouts().await;

        self.task.write().await.spawn(
            self.mrb.read().await.inner(),
            provisioners_list,
            &self.db,
            &self.vm,
            &self.network,
            base_timeouts,
        );
    }

    // Re-route message to consensus task
    pub(crate) async fn reroute_msg(
        &self,
        msg: Message,
    ) -> Result<(), async_channel::SendError<Message>> {
        match &msg.payload {
            Payload::Candidate(_)
            | Payload::Validation(_)
            | Payload::Ratification(_) => {
                self.task.read().await.main_inbound.send(msg).await?;
            }
            Payload::Quorum(_) => {
                self.task.read().await.quorum_inbound.send(msg).await?;
            }
            _ => warn!("invalid inbound message"),
        }
        Ok(())
    }

    fn needs_update(blk: &Block, txs: &[SpentTransaction]) -> bool {
        // Update provisioners at every epoch (where new stakes take effect)
        if blk.header().height % EPOCH == 0 {
            return true;
        }
        // Update provisioners if a slash has been applied
        if blk
            .header()
            .failed_iterations
            .cert_list
            .iter()
            .any(|i| i.is_some())
        {
            return true;
        };
        // Update provisioners if there is a processed unstake transaction
        txs.iter().filter(|t| t.err.is_none()).any(|t| {
            matches!(&t.inner.inner.call, Some((STAKE_CONTRACT, f, _)) if f == "unstake")
        })
    }

    /// Updates most_recent_block together with provisioners list.
    ///
    /// # Arguments
    ///
    /// * `blk` - Block that already exists in ledger
    pub(crate) async fn update_most_recent_block(
        &self,
        blk: &Block,
        label: Label,
    ) -> anyhow::Result<()> {
        let mut task = self.task.write().await;

        let mut mrb = self.mrb.write().await;
        let mut provisioners_list = self.provisioners_list.write().await;

        // Ensure block that will be marked as blockchain tip does exist
        let exists = self
            .db
            .read()
            .await
            .update(|t| t.get_block_exists(&blk.header().hash))?;

        if !exists {
            return Err(anyhow::anyhow!("could not find block"));
        }

        // Reset Consensus
        task.abort_with_wait().await;

        //  Update register.
        self.db.read().await.update(|t| {
            t.op_write(MD_HASH_KEY, blk.header().hash)?;
            t.op_write(MD_STATE_ROOT_KEY, blk.header().state_hash)
        })?;

        let (prev_header, _) = self
            .db
            .read()
            .await
            .view(|t| t.fetch_block_header(&blk.header().prev_block_hash))?
            .expect("Reverting to a block without previous");

        let vm = self.vm.read().await;
        let current_prov = vm.get_provisioners(blk.header().state_hash)?;
        provisioners_list.update(current_prov);
        let previous_prov = vm.get_provisioners(prev_header.state_hash)?;
        provisioners_list.set_previous(previous_prov);

        *mrb = BlockWithLabel::new_with_label(blk.clone(), label);

        Ok(())
    }

    fn log_missing_iterations(
        &self,
        provisioners_list: &Provisioners,
        iteration: u8,
        seed: Seed,
        round: u64,
    ) {
        if iteration == 0 {
            return;
        }
        for iter in 0..iteration {
            let generator =
                provisioners_list.get_generator(iter, seed, round).to_bs58();
            warn!(event = "missed iteration", height = round, iter, generator);
        }
    }

    pub(crate) async fn try_accept_block(
        &mut self,
        blk: &Block,
        enable_consensus: bool,
    ) -> anyhow::Result<Label> {
        let mut task = self.task.write().await;

        let mut mrb = self.mrb.write().await;
        let mut provisioners_list = self.provisioners_list.write().await;
        let block_time =
            blk.header().timestamp - mrb.inner().header().timestamp;

        // Verify Block Header
        let attested = verify_block_header(
            self.db.clone(),
            &mrb.inner().header().clone(),
            &provisioners_list,
            blk.header(),
        )
        .await?;

        // TODO: Remove this variable, it's only used for log purpose
        let mut final_from_rolling = false;

        // Define new block label
        let label = match (attested, mrb.is_final()) {
            (true, true) => Label::Final,
            (false, _) => Label::Accepted,
            (true, _) => {
                let current = blk.header().height;
                let target = current
                    .checked_sub(CONSENSUS_ROLLING_FINALITY_THRESHOLD)
                    .unwrap_or_default();
                self.db.read().await.view(|t| {
                    for h in (target..current).rev() {
                        match t.fetch_block_label_by_height(h)? {
                            None => panic!(
                                "Cannot find block label for height: {h}"
                            ),
                            Some(Label::Final) => {
                                warn!("Found Attested block following a Final one");
                                break;
                            }
                            Some(Label::Accepted) => return Ok(Label::Attested),
                            Some(Label::Attested) => {} // just continue scan
                        };
                    }
                    final_from_rolling = true;
                    anyhow::Ok(Label::Final)
                })?
            }
        };

        let blk = BlockWithLabel::new_with_label(blk.clone(), label);
        let header = blk.inner().header();

        // Reset Consensus
        task.abort_with_wait().await;

        let start = std::time::Instant::now();
        // Persist block in consistency with the VM state update
        {
            let vm = self.vm.write().await;
            let txs = self.db.read().await.update(|t| {
                let (txs, verification_output) = if blk.is_final() {
                    vm.finalize(blk.inner())?
                } else {
                    vm.accept(blk.inner())?
                };

                assert_eq!(header.state_hash, verification_output.state_root);
                assert_eq!(header.event_hash, verification_output.event_hash);

                // Store block with updated transactions with Error and GasSpent
                t.store_block(header, &txs, blk.label())?;

                Ok(txs)
            })?;

            self.log_missing_iterations(
                provisioners_list.current(),
                header.iteration,
                mrb.inner().header().seed,
                header.height,
            );

            for (_, slashed) in
                header.failed_iterations.cert_list.iter().flatten()
            {
                info!("Slashed {}", slashed.to_base58())
            }

            match Self::needs_update(blk.inner(), &txs) {
                true => {
                    let state_hash = blk.inner().header().state_hash;
                    let new_prov = vm.get_provisioners(state_hash)?;
                    provisioners_list.update_and_swap(new_prov)
                }
                false => provisioners_list.remove_previous(),
            }

            // Update most_recent_block
            *mrb = blk;

            anyhow::Ok(())
        }?;

        // Delete from mempool any transaction already included in the block
        self.db.read().await.update(|update| {
            for tx in mrb.inner().txs().iter() {
                database::Mempool::delete_tx(update, tx.hash())?;
                let nullifiers = tx.to_nullifiers();
                for orphan_tx in update.get_txs_by_nullifiers(&nullifiers) {
                    database::Mempool::delete_tx(update, orphan_tx)?;
                }
            }
            Ok(())
        })?;

        let fsv_bitset = mrb.inner().header().cert.validation.bitset;
        let ssv_bitset = mrb.inner().header().cert.ratification.bitset;

        let duration = start.elapsed();
        info!(
            event = "block accepted",
            height = mrb.inner().header().height,
            iter = mrb.inner().header().iteration,
            hash = to_str(&mrb.inner().header().hash),
            txs = mrb.inner().txs().len(),
            state_hash = to_str(&mrb.inner().header().state_hash),
            fsv_bitset,
            ssv_bitset,
            block_time,
            generator = mrb.inner().header().generator_bls_pubkey.to_bs58(),
            dur_ms = duration.as_millis(),
            label = format!("{:?}", label),
            final_from_rolling
        );

        // Restart Consensus.
        if enable_consensus {
            let base_timeouts = self.adjust_round_base_timeouts().await;
            task.spawn(
                mrb.inner(),
                provisioners_list.clone(),
                &self.db,
                &self.vm,
                &self.network,
                base_timeouts,
            );
        }

        Ok(label)
    }

    /// Implements the algorithm of full revert to any of supported targets.
    ///
    /// This incorporates both VM state revert and Ledger state revert.
    pub async fn try_revert(&self, target: RevertTarget) -> Result<()> {
        let curr_height = self.get_curr_height().await;

        let target_state_hash = match target {
            RevertTarget::LastFinalizedState => {
                let vm = self.vm.read().await;
                let state_hash = vm.revert_to_finalized()?;

                info!(
                    event = "vm reverted",
                    state_root = hex::encode(state_hash),
                    is_final = "true",
                );

                anyhow::Ok(state_hash)
            }
            RevertTarget::Commit(state_hash) => {
                let vm = self.vm.read().await;
                let state_hash = vm.revert(state_hash)?;
                let is_final = vm.get_finalized_state_root()? == state_hash;

                info!(
                    event = "vm reverted",
                    state_root = hex::encode(state_hash),
                    is_final,
                );

                anyhow::Ok(state_hash)
            }
            RevertTarget::LastEpoch => unimplemented!(),
        }?;

        // Delete any block until we reach the target_state_hash, the
        // VM was reverted to.

        // The blockchain tip (most recent block) after reverting
        let mut blk = Block::default();
        let mut label: Label = Label::Attested;

        self.db.read().await.update(|t| {
            let mut height = curr_height;
            while height != 0 {
                let b = Ledger::fetch_block_by_height(t, height)?
                    .ok_or_else(|| anyhow::anyhow!("could not fetch block"))?;
                let h = b.header();

                if h.state_hash == target_state_hash {
                    label =
                        t.fetch_block_label_by_height(h.height)?.ok_or_else(
                            || anyhow::anyhow!("could not fetch block label"),
                        )?;

                    blk = b;
                    break;
                }

                info!(
                    event = "block deleted",
                    height = h.height,
                    iter = h.iteration,
                    label = format!("{:?}", label),
                    hash = hex::encode(h.hash)
                );

                // Delete any rocksdb record related to this block
                t.delete_block(&blk)?;

                // Attempt to resubmit transactions back to mempool.
                // An error here is not considered critical.
                for tx in blk.txs().iter() {
                    if let Err(e) = Mempool::add_tx(t, tx) {
                        warn!("failed to resubmit transactions: {e}")
                    };
                }

                height -= 1;
            }

            Ok(())
        })?;

        if blk.header().state_hash != target_state_hash {
            return Err(anyhow!("Failed to revert to proper state"));
        }

        // Update blockchain tip to be the one we reverted to.
        info!(
            event = "updating blockchain tip",
            height = blk.header().height,
            iter = blk.header().iteration,
            state_root = hex::encode(blk.header().state_hash)
        );

        self.update_most_recent_block(&blk, label).await
    }

    /// Spawns consensus algorithm after aborting currently running one
    pub(crate) async fn restart_consensus(&mut self) {
        let mut task = self.task.write().await;
        let mrb = self.mrb.read().await;
        let provisioners_list = self.provisioners_list.read().await.clone();

        task.abort_with_wait().await;
        info!(
            event = "restart consensus",
            height = mrb.inner().header().height,
            iter = mrb.inner().header().iteration,
            hash = to_str(&mrb.inner().header().hash),
        );

        let base_timeouts = self.adjust_round_base_timeouts().await;
        task.spawn(
            mrb.inner(),
            provisioners_list,
            &self.db,
            &self.vm,
            &self.network,
            base_timeouts,
        );
    }

    pub(crate) async fn get_curr_height(&self) -> u64 {
        self.mrb.read().await.inner().header().height
    }

    /// Returns chain tip header
    pub(crate) async fn tip_header(&self) -> ledger::Header {
        self.mrb.read().await.inner().header().clone()
    }

    pub(crate) async fn get_curr_hash(&self) -> [u8; 32] {
        self.mrb.read().await.inner().header().hash
    }

    pub(crate) async fn get_latest_final_block(&self) -> Result<Block> {
        let mrb = self.mrb.read().await;
        if mrb.is_final() {
            return Ok(mrb.inner().clone());
        }

        // Retrieve the latest final block from the database
        let final_block = self.db.read().await.view(|v| {
            let prev_height = mrb.inner().header().height - 1;

            for height in (0..prev_height).rev() {
                if let Ok(Some(Label::Final)) =
                    v.fetch_block_label_by_height(height)
                {
                    if let Some(blk) = v.fetch_block_by_height(height)? {
                        return Ok(blk);
                    } else {
                        return Err(anyhow::anyhow!(
                            "could not fetch the latest final block by height"
                        ));
                    }
                }
            }

            Err(anyhow::anyhow!("could not find the latest final block"))
        })?;

        Ok(final_block)
    }

    pub(crate) async fn get_curr_iteration(&self) -> u8 {
        self.mrb.read().await.inner().header().iteration
    }

    pub(crate) async fn get_result_chan(
        &self,
    ) -> AsyncQueue<Result<Block, ConsensusError>> {
        self.task.read().await.result.clone()
    }

    pub(crate) async fn get_outbound_chan(&self) -> AsyncQueue<Message> {
        self.task.read().await.outbound.clone()
    }

    async fn adjust_round_base_timeouts(&self) -> TimeoutSet {
        let mut base_timeout_set = TimeoutSet::new();

        base_timeout_set.insert(
            StepName::Proposal,
            self.read_avg_timeout(MD_AVG_PROPOSAL).await,
        );

        base_timeout_set.insert(
            StepName::Validation,
            self.read_avg_timeout(MD_AVG_VALIDATION).await,
        );

        base_timeout_set.insert(
            StepName::Ratification,
            self.read_avg_timeout(MD_AVG_RATIFICATION).await,
        );

        base_timeout_set
    }

    async fn read_avg_timeout(&self, key: &[u8]) -> Duration {
        let metric = self.db.read().await.view(|t| {
            let bytes = &t.op_read(key)?.unwrap_or_default();
            let metric =
                AverageElapsedTime::read(&mut &bytes[..]).unwrap_or_default();
            Ok::<AverageElapsedTime, anyhow::Error>(metric)
        });

        metric
            .unwrap_or_default()
            .average()
            .unwrap_or(MIN_STEP_TIMEOUT)
    }
}

/// Performs full verification of block header against prev_block header where
/// prev_block is usually the blockchain tip
pub(crate) async fn verify_block_header<DB: database::DB>(
    db: Arc<RwLock<DB>>,
    prev_header: &ledger::Header,
    provisioners: &ContextProvisioners,
    header: &ledger::Header,
) -> anyhow::Result<bool> {
    let validator = Validator::new(db, prev_header, provisioners);
    validator.execute_checks(header, false).await
}
