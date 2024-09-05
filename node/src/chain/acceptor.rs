// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::database::{self, Candidate, Ledger, Mempool, Metadata};
use crate::{vm, Message, Network};
use anyhow::{anyhow, Result};
use dusk_consensus::commons::{ConsensusError, TimeoutSet};
use dusk_consensus::config::{MAX_STEP_TIMEOUT, MIN_STEP_TIMEOUT};
use dusk_consensus::user::provisioners::{ContextProvisioners, Provisioners};
use node_data::bls::PublicKey;
use node_data::events::{
    BlockEvent, Event, TransactionEvent, BLOCK_CONFIRMED, BLOCK_FINALIZED,
};
use node_data::ledger::{
    self, to_str, Block, BlockWithLabel, Label, Seed, Slash, SpentTransaction,
};
use node_data::message::AsyncQueue;
use node_data::message::Payload;

use dusk_consensus::operations::Voter;
use execution_core::stake::{Withdraw, STAKE_CONTRACT};
use metrics::{counter, gauge, histogram};
use node_data::message::payload::Vote;
use node_data::{get_current_timestamp, Serializable, StepName};
use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::consensus::Task;
use crate::chain::header_validation::{verify_faults, Validator};
use crate::chain::metrics::AverageElapsedTime;
use crate::database::rocksdb::{
    MD_AVG_PROPOSAL, MD_AVG_RATIFICATION, MD_AVG_VALIDATION, MD_HASH_KEY,
    MD_STATE_ROOT_KEY,
};

const CANDIDATES_DELETION_OFFSET: u64 = 10;

/// The offset to the current blockchain tip to consider a message as valid
/// future message.
const OFFSET_FUTURE_MSGS: u64 = 5;

pub type RollingFinalityResult = ([u8; 32], BTreeMap<u64, [u8; 32]>);

#[allow(dead_code)]
pub(crate) enum RevertTarget {
    Commit([u8; 32]),
    LastFinalizedState,
    LastEpoch,
}

/// Implements block acceptance procedure. This includes block header,
/// attestation and transactions full verifications.
/// Acceptor also manages the initialization and lifespan of Consensus task.
pub(crate) struct Acceptor<N: Network, DB: database::DB, VM: vm::VMExecution> {
    /// The tip
    tip: RwLock<BlockWithLabel>,

    /// Provisioners needed to verify next block
    pub(crate) provisioners_list: RwLock<ContextProvisioners>,

    /// Upper layer consensus task
    task: RwLock<super::consensus::Task>,

    pub(crate) db: Arc<RwLock<DB>>,
    pub(crate) vm: Arc<RwLock<VM>>,
    network: Arc<RwLock<N>>,

    event_sender: Sender<Event>,
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

#[derive(Debug)]
enum ProvisionerChange {
    Stake(PublicKey),
    Unstake(PublicKey),
    Slash(PublicKey),
    Reward(PublicKey),
}

impl ProvisionerChange {
    fn into_public_key(self) -> PublicKey {
        match self {
            ProvisionerChange::Slash(pk) => pk,
            ProvisionerChange::Unstake(pk) => pk,
            ProvisionerChange::Stake(pk) => pk,
            ProvisionerChange::Reward(pk) => pk,
        }
    }

    fn is_stake(&self) -> bool {
        matches!(self, ProvisionerChange::Stake(_))
    }
}

pub static DUSK_KEY: LazyLock<PublicKey> = LazyLock::new(|| {
    let dusk_cpk_bytes = include_bytes!("../../../rusk/src/assets/dusk.cpk");
    PublicKey::try_from(*dusk_cpk_bytes)
        .expect("Dusk consensus public key to be valid")
});

impl<DB: database::DB, VM: vm::VMExecution, N: Network> Acceptor<N, DB, VM> {
    /// Initializes a new `Acceptor` struct,
    ///
    /// The method loads the VM state and verifies consistency between the VM
    /// and Ledger states. If any inconsistencies are found, it reverts to the
    /// last known finalized state. Finally, it initiates a new consensus
    /// [Task].
    #[allow(clippy::too_many_arguments)]
    pub async fn init_consensus(
        keys_path: &str,
        tip: BlockWithLabel,
        provisioners_list: Provisioners,
        db: Arc<RwLock<DB>>,
        network: Arc<RwLock<N>>,
        vm: Arc<RwLock<VM>>,
        max_queue_size: usize,
        event_sender: Sender<Event>,
    ) -> anyhow::Result<Self> {
        let tip_height = tip.inner().header().height;
        let tip_state_hash = tip.inner().header().state_hash;

        let mut provisioners_list = ContextProvisioners::new(provisioners_list);

        if tip.inner().header().height > 0 {
            let changed_provisioners =
                vm.read().await.get_changed_provisioners(tip_state_hash)?;
            provisioners_list.apply_changes(changed_provisioners);
        }

        let acc = Self {
            tip: RwLock::new(tip),
            provisioners_list: RwLock::new(provisioners_list),
            db: db.clone(),
            vm: vm.clone(),
            network: network.clone(),
            task: RwLock::new(Task::new_with_keys(
                keys_path.to_string(),
                max_queue_size,
            )?),
            event_sender,
        };

        // NB. After restart, state_root returned by VM is always the last
        // finalized one.
        let state_root = vm.read().await.get_state_root()?;

        info!(
            event = "VM finalized state loaded",
            state_root = hex::encode(state_root),
        );

        if tip_height > 0 && tip_state_hash != state_root {
            if let Err(error) = vm.read().await.move_to_commit(tip_state_hash) {
                warn!(
                    event = "Cannot move to tip_state_hash",
                    ?error,
                    state_root = hex::encode(tip_state_hash)
                );

                info!("revert to last finalized state");
                // Revert to last known finalized state.
                acc.try_revert(RevertTarget::LastFinalizedState).await?;
            } else {
                info!(
                    event = "VM accepted state loaded",
                    state_root = hex::encode(tip_state_hash),
                );
            }
        }

        Ok(acc)
    }

    pub async fn spawn_task(&self) {
        let provisioners_list = self.provisioners_list.read().await.clone();
        let base_timeouts = self.adjust_round_base_timeouts().await;
        let tip = self.tip.read().await.inner().clone();

        let tip_block_voters =
            self.get_att_voters(provisioners_list.prev(), &tip).await;

        self.task.write().await.spawn(
            &tip,
            provisioners_list,
            &self.db,
            &self.vm,
            base_timeouts,
            tip_block_voters,
        );
    }

    async fn get_att_voters(
        &self,
        provisioners_list: &Provisioners,
        tip: &Block,
    ) -> Vec<Voter> {
        if tip.header().height == 0 {
            return vec![];
        };

        let prev_seed = self.get_prev_block_seed().await.expect("valid seed");
        Validator::<DB>::get_voters(tip.header(), provisioners_list, prev_seed)
            .await
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
                    task.main_inbound.try_send(msg);
                }
            }
            Payload::Quorum(payload) => {
                // Prevent the rebroadcast of any quorum messages if the
                // blockchain tip has already been updated for the same round.
                if let Vote::Valid(hash) = payload.vote() {
                    if *hash != self.get_curr_hash().await {
                        broadcast(&self.network, &msg).await;
                    }
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
                    Some(stake) => {
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
        let generator = blk.header().generator_bls_pubkey.0;
        let generator = generator
            .try_into()
            .map_err(|e| anyhow::anyhow!("Cannot deserialize bytes {e:?}"))?;
        let reward = ProvisionerChange::Reward(generator);
        let dusk_reward = ProvisionerChange::Reward(DUSK_KEY.clone());
        let mut changed_provisioners = vec![reward, dusk_reward];

        // Update provisioners if a slash has been applied
        let slashed = Slash::from_block(blk)?
            .into_iter()
            .map(|f| ProvisionerChange::Slash(f.provisioner));
        changed_provisioners.extend(slashed);

        // FIX_ME: This relies on the stake contract being called only by the
        // transfer contract. We should change this once third-party contracts
        // hit the chain.
        let stake_calls =
            txs.iter().filter(|t| t.err.is_none()).filter_map(|t| {
                match &t.inner.inner.call() {
                    Some(call)
                        if (call.contract == STAKE_CONTRACT
                            && (call.fn_name == STAKE
                                || call.fn_name == UNSTAKE)) =>
                    {
                        Some((&call.fn_name, &call.fn_args))
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
                let unstake: Withdraw =
                    rkyv::from_bytes(calldata).map_err(|e| {
                        anyhow::anyhow!("Cannot deserialize unstake rkyv {e:?}")
                    })?;
                ProvisionerChange::Unstake(PublicKey::new(*unstake.account()))
            }
            STAKE => {
                let stake: execution_core::stake::Stake =
                    rkyv::from_bytes(calldata).map_err(|e| {
                        anyhow::anyhow!("Cannot deserialize stake rkyv {e:?}")
                    })?;
                ProvisionerChange::Stake(PublicKey::new(*stake.account()))
            }
            e => unreachable!("Parsing unexpected method: {e}"),
        };
        Ok(change)
    }

    /// Updates tip together with provisioners list.
    ///
    /// # Arguments
    ///
    /// * `blk` - Block that already exists in ledger
    pub(crate) async fn update_tip(
        &self,
        blk: &Block,
        label: Label,
    ) -> anyhow::Result<()> {
        let mut task = self.task.write().await;

        let mut tip = self.tip.write().await;
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

        let vm = self.vm.read().await;
        let current_prov = vm.get_provisioners(blk.header().state_hash)?;
        provisioners_list.update(current_prov);

        let changed_provisioners =
            vm.get_changed_provisioners(blk.header().state_hash)?;
        provisioners_list.apply_changes(changed_provisioners);

        *tip = BlockWithLabel::new_with_label(blk.clone(), label);

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

    /// Return true if the accepted blocks triggered a rolling finality
    pub(crate) async fn try_accept_block(
        &mut self,
        blk: &Block,
        enable_consensus: bool,
    ) -> anyhow::Result<bool> {
        let mut events = vec![];
        let mut task = self.task.write().await;

        let mut tip = self.tip.write().await;
        let mut provisioners_list = self.provisioners_list.write().await;
        let block_time =
            blk.header().timestamp - tip.inner().header().timestamp;

        let header_verification_start = std::time::Instant::now();
        // Verify Block Header
        let (pni, prev_block_voters, tip_block_voters) = verify_block_header(
            self.db.clone(),
            &tip.inner().header().clone(),
            &provisioners_list,
            blk.header(),
        )
        .await?;

        // Elapsed time header verification
        histogram!("dusk_block_header_elapsed")
            .record(header_verification_start.elapsed());

        let start = std::time::Instant::now();
        let mut est_elapsed_time = Duration::default();
        let mut block_size_on_disk = 0;
        let mut slashed_count: usize = 0;
        // Persist block in consistency with the VM state update
        let (label, finalized) = {
            let header = blk.header();
            verify_faults(self.db.clone(), header.height, blk.faults()).await?;

            let vm = self.vm.write().await;

            let (txs, rolling_result) = self.db.read().await.update(|db| {
                let (txs, verification_output) =
                    vm.accept(blk, Some(&prev_block_voters[..]))?;
                for spent_tx in txs.iter() {
                    events.push(TransactionEvent::Executed(spent_tx).into());
                }
                est_elapsed_time = start.elapsed();

                assert_eq!(header.state_hash, verification_output.state_root);
                assert_eq!(header.event_hash, verification_output.event_hash);

                let rolling_results =
                    self.rolling_finality::<DB>(pni, blk, db, &mut events)?;

                let label = rolling_results.0;
                // Store block with updated transactions with Error and GasSpent
                block_size_on_disk =
                    db.store_block(header, &txs, blk.faults(), label)?;

                Ok((txs, rolling_results))
            })?;

            self.log_missing_iterations(
                provisioners_list.current(),
                header.iteration,
                tip.inner().header().seed,
                header.height,
            );

            for slashed in Slash::from_block(blk)? {
                info!("Slashed {}", slashed.provisioner.to_base58());
                slashed_count += 1;
            }

            let selective_update =
                Self::selective_update(blk, &txs, &vm, &mut provisioners_list);

            if let Err(e) = selective_update {
                warn!("Resync provisioners due to {e:?}");
                let state_hash = blk.header().state_hash;
                let new_prov = vm.get_provisioners(state_hash)?;
                provisioners_list.update_and_swap(new_prov)
            }

            let (label, final_results) = rolling_result;
            // Update tip
            *tip = BlockWithLabel::new_with_label(blk.clone(), label);

            let finalized = final_results.is_some();

            if let Some((prev_final_state, mut new_finals)) = final_results {
                let (_, new_final_state) =
                    new_finals.pop_last().expect("new_finals to be not empty");
                let states_to_forget = new_finals
                    .into_values()
                    .chain([prev_final_state])
                    .collect::<Vec<_>>();
                vm.finalize_state(new_final_state, states_to_forget)?;
            }

            anyhow::Ok((label, finalized))
        }?;

        // Abort consensus.
        // A fully valid block is accepted, consensus task must be aborted.
        task.abort_with_wait().await;

        Self::emit_metrics(
            tip.inner(),
            &label,
            est_elapsed_time,
            block_time,
            block_size_on_disk,
            slashed_count,
        );

        // Clean up the database
        let count = self
            .db
            .read()
            .await
            .update(|t| {
                // Delete any candidate block older than TIP - OFFSET
                let threshold = tip
                    .inner()
                    .header()
                    .height
                    .saturating_sub(CANDIDATES_DELETION_OFFSET);

                Candidate::delete(t, |height| height <= threshold)?;

                // Delete from mempool any transaction already included in the
                // block
                for tx in tip.inner().txs().iter() {
                    let tx_id = tx.id();
                    let deleted = Mempool::delete_tx(t, tx_id)
                        .map_err(|e| warn!("Error while deleting tx: {e}"))
                        .unwrap_or_default();
                    if deleted {
                        events.push(TransactionEvent::Removed(tx_id).into());
                    }

                    let spend_ids = tx.to_spend_ids();
                    for orphan_tx in t.get_txs_by_spendable_ids(&spend_ids) {
                        let deleted = Mempool::delete_tx(t, orphan_tx)
                            .map_err(|e| {
                                warn!("Error while deleting orphan_tx: {e}")
                            })
                            .unwrap_or_default();
                        if deleted {
                            events.push(
                                TransactionEvent::Removed(orphan_tx).into(),
                            );
                        }
                    }
                }
                Ok(Candidate::count(t))
            })
            .map_err(|e| warn!("Error while cleaning up the database: {e}"));

        gauge!("dusk_stored_candidates_count")
            .set(count.unwrap_or_default() as f64);

        {
            // Avoid accumulation of future msgs while the node is syncing up
            let round = tip.inner().header().height;
            let mut f = task.future_msg.lock().await;
            f.remove_msgs_out_of_range(round + 1, OFFSET_FUTURE_MSGS);
            histogram!("dusk_future_msg_count").record(f.msg_count() as f64);
        }

        let fsv_bitset = tip.inner().header().att.validation.bitset;
        let ssv_bitset = tip.inner().header().att.ratification.bitset;

        let duration = start.elapsed();
        info!(
            event = "block accepted",
            height = tip.inner().header().height,
            iter = tip.inner().header().iteration,
            hash = to_str(&tip.inner().header().hash),
            txs = tip.inner().txs().len(),
            state_hash = to_str(&tip.inner().header().state_hash),
            fsv_bitset,
            ssv_bitset,
            block_time,
            generator = tip.inner().header().generator_bls_pubkey.to_bs58(),
            dur_ms = duration.as_millis(),
            ?label
        );

        events.push(BlockEvent::Accepted(tip.inner()).into());

        for node_event in events {
            if let Err(e) = self.event_sender.try_send(node_event) {
                warn!("cannot notify event {e}")
            };
        }

        // Restart Consensus.
        if enable_consensus {
            let base_timeouts = self.adjust_round_base_timeouts().await;
            task.spawn(
                tip.inner(),
                provisioners_list.clone(),
                &self.db,
                &self.vm,
                base_timeouts,
                tip_block_voters,
            );
        }

        Ok(finalized)
    }

    /// Perform the rolling finality checks, updating the database with new
    /// labels if required
    ///
    /// Returns
    /// - Current accepted block label
    /// - Previous last finalized state root
    /// - List of the new finalized state root
    fn rolling_finality<D: database::DB>(
        &self,
        pni: u8,
        blk: &Block,
        db: &D::P<'_>,
        events: &mut Vec<Event>,
    ) -> Result<(Label, Option<RollingFinalityResult>)> {
        let confirmed_after = match pni {
            0 => 1u64,
            n => 2 * n as u64,
        };
        let block_label = if pni == 0 {
            Label::Attested(confirmed_after)
        } else {
            return Ok((Label::Accepted(confirmed_after), None));
        };
        let mut finalized_blocks = BTreeMap::new();

        let current_height = blk.header().height;
        let mut labels = BTreeMap::new();

        // Retrieve latest blocks up to the Last Finalized Block
        let mut lfb_hash = None;
        for height in (0..current_height).rev() {
            let (hash, label) = db.fetch_block_label_by_height(height)?.ok_or(
                anyhow!("Cannot find block label for height {height}"),
            )?;
            if let Label::Final(_) = label {
                lfb_hash = Some(hash);
                break;
            }
            labels.insert(height, (hash, label));
        }
        let lfb_hash =
            lfb_hash.expect("Unable to find last finalized block hash");
        let lfb_state_root = db
            .fetch_block_header(&lfb_hash)?
            .ok_or(anyhow!(
                "Cannot get header for last finalized block hash {}",
                to_str(&lfb_hash)
            ))?
            .state_hash;

        // A block is considered stable when is either Confirmed or Attested
        // We start with `stable_count=1` because we are sure to be processing
        // an Attested block
        let mut stable_count = 1;

        // Iterate from TIP to LFB to set Label::Confirmed
        for (&height, (hash, label)) in labels.iter_mut().rev() {
            match label {
                Label::Accepted(ref confirmed_after)
                | Label::Attested(ref confirmed_after) => {
                    if &stable_count >= confirmed_after {
                        info!(
                            event = "block confirmed",
                            src = "rolling_finality",
                            current_height,
                            height,
                            confirmed_after,
                            hash = to_str(hash),
                            ?label,
                        );
                        *label = Label::Confirmed(current_height - height);

                        let event = BlockEvent::StateChange {
                            hash: *hash,
                            state: BLOCK_CONFIRMED,
                            height: current_height,
                        };
                        events.push(event.into());

                        db.store_block_label(height, hash, *label)?;
                        stable_count += 1;
                    } else {
                        break;
                    }
                }
                Label::Confirmed(_) => {
                    stable_count += 1;
                    continue;
                }
                Label::Final(_) => {
                    warn!("Found a final block during rolling finality scan. This should be a bug");
                    break;
                }
            }
        }

        // Iterate from LFB to tip to set Label::Final
        for (height, (hash, mut label)) in labels.into_iter() {
            match label {
                Label::Final(_) => {
                    warn!("Found a final block during rolling finality. This should be a bug")
                }
                Label::Accepted(_) | Label::Attested(_) => break,
                Label::Confirmed(_) => {
                    let finalized_after = current_height - height;
                    label = Label::Final(finalized_after);
                    let event = BlockEvent::StateChange {
                        hash,
                        state: BLOCK_FINALIZED,
                        height: current_height,
                    };
                    events.push(event.into());
                    db.store_block_label(height, &hash, label)?;

                    let state_hash = db
                        .fetch_block_header(&hash)?
                        .map(|h| h.state_hash)
                        .ok_or(anyhow!(
                            "Cannot get header for hash {}",
                            to_str(&hash)
                        ))?;
                    info!(
                        event = "block finalized",
                        src = "rolling_finality",
                        current_height,
                        height,
                        finalized_after,
                        hash = to_str(&hash),
                        state_root = to_str(&state_hash),
                    );
                    finalized_blocks.insert(height, state_hash);
                }
            }
        }

        let finalized_result = if finalized_blocks.is_empty() {
            None
        } else {
            Some((lfb_state_root, finalized_blocks))
        };

        Ok((block_label, finalized_result))
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

        // The blockchain tip after reverting
        let (blk, (_, label)) = self.db.read().await.update(|t| {
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

                let now = get_current_timestamp();

                // Attempt to resubmit transactions back to mempool.
                // An error here is not considered critical.
                // Txs timestamp is reset here
                for tx in b.txs().iter() {
                    if let Err(e) = Mempool::add_tx(t, tx, now) {
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

        self.update_tip(&blk, label).await
    }

    /// Spawns consensus algorithm after aborting currently running one
    pub(crate) async fn restart_consensus(&mut self) {
        let mut task = self.task.write().await;
        let tip = self.tip.read().await.inner().clone();
        let provisioners_list = self.provisioners_list.read().await.clone();

        task.abort_with_wait().await;
        info!(
            event = "restart consensus",
            height = tip.header().height,
            iter = tip.header().iteration,
            hash = to_str(&tip.header().hash),
        );

        let tip_block_voters =
            self.get_att_voters(provisioners_list.prev(), &tip).await;

        let base_timeouts = self.adjust_round_base_timeouts().await;
        task.spawn(
            &tip,
            provisioners_list,
            &self.db,
            &self.vm,
            base_timeouts,
            tip_block_voters,
        );
    }

    pub(crate) async fn get_curr_height(&self) -> u64 {
        self.tip.read().await.inner().header().height
    }

    /// Returns chain tip header
    pub(crate) async fn tip_header(&self) -> ledger::Header {
        self.tip.read().await.inner().header().clone()
    }

    pub(crate) async fn get_curr_hash(&self) -> [u8; 32] {
        self.tip.read().await.inner().header().hash
    }

    pub(crate) async fn get_latest_final_block(&self) -> Result<Block> {
        let tip = self.tip.read().await;
        if tip.is_final() {
            return Ok(tip.inner().clone());
        }

        // Retrieve the latest final block from the database
        let final_block = self.db.read().await.view(|v| {
            let prev_height = tip.inner().header().height - 1;

            for height in (0..prev_height).rev() {
                if let Ok(Some((hash, Label::Final(_)))) =
                    v.fetch_block_label_by_height(height)
                {
                    if let Some(blk) = v.fetch_block(&hash)? {
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
        self.tip.read().await.inner().header().iteration
    }

    pub(crate) async fn get_result_chan(
        &self,
    ) -> AsyncQueue<Result<(), ConsensusError>> {
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

    async fn get_prev_block_seed(&self) -> Result<Seed> {
        let tip = self.tip.read().await;
        let header = tip.inner().header();
        if header.height == 0 {
            return Ok(Seed::default());
        }

        self.db
            .read()
            .await
            .view(|t| {
                let res = t
                    .fetch_block_header(&header.prev_block_hash)?
                    .map(|prev| prev.seed);

                anyhow::Ok::<Option<Seed>>(res)
            })?
            .ok_or_else(|| anyhow::anyhow!("could not retrieve seed"))
    }

    fn emit_metrics(
        blk: &Block,
        block_label: &Label,
        est_elapsed_time: Duration,
        block_time: u64,
        block_size_on_disk: usize,
        slashed_count: usize,
    ) {
        // The Cumulative number of all executed transactions
        counter!("dusk_txn_count").increment(blk.txs().len() as u64);

        // The Cumulative number of all blocks by label
        counter!(format!("dusk_block_{:?}", *block_label)).increment(1);

        // A histogram of block time
        if blk.header().height > 1 {
            histogram!("dusk_block_time").record(block_time as f64);
        }

        histogram!("dusk_block_iter").record(blk.header().iteration as f64);

        // Elapsed time of Accept/Finalize call
        histogram!("dusk_block_est_elapsed").record(est_elapsed_time);

        // A histogram of slashed count
        histogram!("dusk_slashed_count").record(slashed_count as f64);

        histogram!("dusk_block_disk_size").record(block_size_on_disk as f64);
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
/// Returns the number of Previous Non-Attested Iterations (PNI).
pub(crate) async fn verify_block_header<DB: database::DB>(
    db: Arc<RwLock<DB>>,
    prev_header: &ledger::Header,
    provisioners: &ContextProvisioners,
    header: &ledger::Header,
) -> anyhow::Result<(u8, Vec<Voter>, Vec<Voter>)> {
    let validator = Validator::new(db, prev_header, provisioners);
    validator.execute_checks(header, false).await
}
