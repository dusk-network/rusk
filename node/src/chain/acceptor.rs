// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::database::{self, Candidate, Ledger, Mempool, Metadata};
use crate::{vm, Message, Network};
use anyhow::{anyhow, Result};
use dusk_consensus::commons::{ConsensusError, TimeoutSet};
use dusk_consensus::config::{
    CONSENSUS_ROLLING_FINALITY_THRESHOLD, MAX_STEP_TIMEOUT, MIN_STEP_TIMEOUT,
};
use dusk_consensus::user::provisioners::{ContextProvisioners, Provisioners};
use node_data::bls::PublicKey;
use node_data::ledger::{
    self, to_str, Block, BlockWithLabel, Label, Seed, SpentTransaction,
};
use node_data::message::AsyncQueue;
use node_data::message::Payload;

use node_data::{Serializable, StepName};
use stake_contract_types::Unstake;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use super::consensus::Task;
use crate::chain::header_validation::Validator;
use crate::chain::metrics::AverageElapsedTime;
use crate::database::rocksdb::{
    MD_AVG_PROPOSAL, MD_AVG_RATIFICATION, MD_AVG_VALIDATION, MD_HASH_KEY,
    MD_STATE_ROOT_KEY,
};

const DUSK: u64 = 1_000_000_000;
const MINIMUM_STAKE: u64 = 1_000 * DUSK;
const CANDIDATES_DELETION_OFFSET: u64 = 10;

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

const STAKE: &str = "stake";
const UNSTAKE: &str = "unstake";
const STAKE_CONTRACT: [u8; 32] = stake_contract_id();
const fn stake_contract_id() -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[0] = 2;
    bytes
}

#[derive(Debug)]
enum ProvisionerChange {
    Stake(PublicKey),
    Unstake(PublicKey),
    Slash(PublicKey),
}

impl ProvisionerChange {
    fn into_public_key(self) -> PublicKey {
        match self {
            ProvisionerChange::Slash(pk) => pk,
            ProvisionerChange::Unstake(pk) => pk,
            ProvisionerChange::Stake(pk) => pk,
        }
    }

    fn is_stake(&self) -> bool {
        matches!(self, ProvisionerChange::Stake(_))
    }
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
            task: RwLock::new(Task::new_with_keys(keys_path.to_string())?),
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

        Ok(acc)
    }

    pub async fn spawn_task(&self) {
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
    ) -> Result<(), async_channel::TrySendError<Message>> {
        let curr_tip = self.get_curr_height().await;

        // Enqueue consensus msg only if local tip is close enough to the
        // network tip.
        let enable_enqueue =
            msg.header.round >= curr_tip && msg.header.round < (curr_tip + 10);

        match &msg.payload {
            Payload::Candidate(_)
            | Payload::Validation(_)
            | Payload::Ratification(_) => {
                let task = self.task.read().await;
                if !task.is_running() {
                    broadcast(&self.network, &msg).await;
                }

                if enable_enqueue {
                    task.main_inbound.try_send(msg)?;
                }
            }
            Payload::Quorum(_) => {
                let task = self.task.read().await;
                if !task.is_running() {
                    broadcast(&self.network, &msg).await;
                }

                if enable_enqueue {
                    task.quorum_inbound.try_send(msg)?;
                }
            }
            _ => warn!("invalid inbound message"),
        }
        Ok(())
    }

    fn selective_update(
        blk: &Block,
        txs: &[SpentTransaction],
        vm: &tokio::sync::RwLockWriteGuard<'_, VM>,
        provisioners_list: &mut tokio::sync::RwLockWriteGuard<
            '_,
            ContextProvisioners,
        >,
    ) -> Result<()> {
        let src = "selective";
        let changed_prov = Self::changed_provisioners(blk, txs)?;
        if changed_prov.is_empty() {
            provisioners_list.remove_previous();
        } else {
            let mut new_prov = provisioners_list.current().clone();
            for change in changed_prov {
                let is_stake = change.is_stake();
                info!(event = "provisioner_update", src, ?change);
                let pk = change.into_public_key();
                let prov = pk.to_bs58();
                match vm.get_provisioner(pk.inner())? {
                    Some(stake) if stake.value() >= MINIMUM_STAKE => {
                        debug!(event = "new_stake", src, prov, ?stake);
                        let replaced = new_prov.replace_stake(pk, stake);
                        if replaced.is_none() && !is_stake {
                            anyhow::bail!("Replaced a not existing stake")
                        };
                        debug!(event = "old_stake", src, prov, ?replaced);
                    }
                    _ => {
                        let removed = new_prov.remove_stake(&pk).ok_or(
                            anyhow::anyhow!("Removed a not existing stake"),
                        )?;
                        debug!(event = "removed_stake", src, prov, ?removed);
                    }
                }
            }
            // Update new prov
            provisioners_list.update_and_swap(new_prov);
        }
        Ok(())
    }

    fn changed_provisioners(
        blk: &Block,
        txs: &[SpentTransaction],
    ) -> Result<Vec<ProvisionerChange>> {
        let mut changed_provisioners = vec![];

        // Update provisioners if a slash has been applied
        for bytes in blk.header().failed_iterations.to_missed_generators_bytes()
        {
            let slashed = bytes.0.try_into().map_err(|e| {
                anyhow::anyhow!("Cannot deserialize bytes {e:?}")
            })?;
            changed_provisioners.push(ProvisionerChange::Slash(slashed));
        }

        // FIX_ME: This relies on the stake contract being called only by the
        // transfer contract. We should change this once third-party contracts
        // hit the chain.
        let stake_calls =
            txs.iter().filter(|t| t.err.is_none()).filter_map(|t| {
                match &t.inner.inner.call {
                    Some((STAKE_CONTRACT, fn_name, data))
                        if (fn_name == STAKE || fn_name == UNSTAKE) =>
                    {
                        Some((fn_name, data))
                    }
                    _ => None,
                }
            });

        for (f, data) in stake_calls {
            changed_provisioners.push(Self::parse_stake_call(f, data)?);
        }

        Ok(changed_provisioners)
    }

    fn parse_stake_call(
        fn_name: &str,
        calldata: &[u8],
    ) -> Result<ProvisionerChange> {
        let change = match fn_name {
            UNSTAKE => {
                let unstake: Unstake =
                    rkyv::from_bytes(calldata).map_err(|e| {
                        anyhow::anyhow!("Cannot deserialize unstake rkyv {e:?}")
                    })?;
                ProvisionerChange::Unstake(PublicKey::new(unstake.public_key))
            }
            STAKE => {
                let stake: stake_contract_types::Stake =
                    rkyv::from_bytes(calldata).map_err(|e| {
                        anyhow::anyhow!("Cannot deserialize stake rkyv {e:?}")
                    })?;
                ProvisionerChange::Stake(PublicKey::new(stake.public_key))
            }
            e => unreachable!("Parsing unexpected method: {e}"),
        };
        Ok(change)
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

        // Final from rolling
        let mut ffr = false;

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
                    ffr = true;
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

            for slashed in header.failed_iterations.to_missed_generators_bytes()
            {
                info!("Slashed {}", slashed.to_base58())
            }

            let selective_update = Self::selective_update(
                blk.inner(),
                &txs,
                &vm,
                &mut provisioners_list,
            );

            if let Err(e) = selective_update {
                warn!("Resync provisioners due to {e:?}");
                let state_hash = blk.inner().header().state_hash;
                let new_prov = vm.get_provisioners(state_hash)?;
                provisioners_list.update_and_swap(new_prov)
            }

            // Update most_recent_block
            *mrb = blk;

            anyhow::Ok(())
        }?;

        // Clean up the database
        let count = self
            .db
            .read()
            .await
            .update(|t| {
                // Delete any candidate block older than TIP - OFFSET
                let threshold = mrb
                    .inner()
                    .header()
                    .height
                    .checked_sub(CANDIDATES_DELETION_OFFSET)
                    .unwrap_or(0);
                Candidate::delete(t, |height| height <= threshold)?;

                // Delete from mempool any transaction already included in the
                // block
                for tx in mrb.inner().txs().iter() {
                    let _ = Mempool::delete_tx(t, tx.hash())
                        .map_err(|e| warn!("Error while deleting tx: {e}"));

                    let nullifiers = tx.to_nullifiers();
                    for orphan_tx in t.get_txs_by_nullifiers(&nullifiers) {
                        let _ = Mempool::delete_tx(t, orphan_tx).map_err(|e| {
                            warn!("Error while deleting orphan_tx: {e}")
                        });
                    }
                }
                Ok(Candidate::count(t))
            })
            .map_err(|e| warn!("Error while cleaning up the database: {e}"));

        debug!(
            event = "stats",
            height = mrb.inner().header().height,
            candidates_count = count.unwrap_or_default(),
        );

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
            ffr
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
        let (blk, label) = self.db.read().await.update(|t| {
            let mut height = curr_height;
            while height != 0 {
                let b = Ledger::fetch_block_by_height(t, height)?
                    .ok_or_else(|| anyhow::anyhow!("could not fetch block"))?;
                let h = b.header();
                let label =
                    t.fetch_block_label_by_height(h.height)?.ok_or_else(
                        || anyhow::anyhow!("could not fetch block label"),
                    )?;

                if h.state_hash == target_state_hash {
                    return Ok((b, label));
                }

                info!(
                    event = "block deleted",
                    height = h.height,
                    iter = h.iteration,
                    label = ?label,
                    hash = hex::encode(h.hash)
                );

                // Delete any rocksdb record related to this block
                t.delete_block(&b)?;

                // Attempt to resubmit transactions back to mempool.
                // An error here is not considered critical.
                for tx in b.txs().iter() {
                    if let Err(e) = Mempool::add_tx(t, tx) {
                        warn!("failed to resubmit transactions: {e}")
                    };
                }

                height -= 1;
            }

            Err(anyhow!("not found"))
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
            let bytes = &t.op_read(key)?;
            let metric = match bytes {
                Some(bytes) => AverageElapsedTime::read(&mut &bytes[..])
                    .unwrap_or_default(),
                None => {
                    let mut metric = AverageElapsedTime::default();
                    metric.push_back(MAX_STEP_TIMEOUT);
                    metric
                }
            };

            Ok::<AverageElapsedTime, anyhow::Error>(metric)
        });

        metric
            .unwrap_or_default()
            .average()
            .unwrap_or(MIN_STEP_TIMEOUT)
            .max(MIN_STEP_TIMEOUT)
            .min(MAX_STEP_TIMEOUT)
    }
}

async fn broadcast<N: Network>(network: &Arc<RwLock<N>>, msg: &Message) {
    let _ = network.read().await.broadcast(msg).await.map_err(|err| {
        warn!("Unable to broadcast msg: {:?} {err} ", msg.topic())
    });
}

/// Performs full verification of block header against prev_block header where
/// prev_block is usually the blockchain tip
///
/// Returns true if there is a cerificate for each failed iteration, and if
/// that certificate has a quorum in the ratification phase.
///
/// If there are no failed iterations, it returns true
pub(crate) async fn verify_block_header<DB: database::DB>(
    db: Arc<RwLock<DB>>,
    prev_header: &ledger::Header,
    provisioners: &ContextProvisioners,
    header: &ledger::Header,
) -> anyhow::Result<bool> {
    let validator = Validator::new(db, prev_header, provisioners);
    validator.execute_checks(header, false).await
}
