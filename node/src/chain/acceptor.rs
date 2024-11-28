// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::database::{self, ConsensusStorage, Ledger, Mempool, Metadata};
use crate::{vm, Message, Network};
use anyhow::{anyhow, Result};
use dusk_consensus::commons::TimeoutSet;
use dusk_consensus::config::{
    MAX_ROUND_DISTANCE, MAX_STEP_TIMEOUT, MIN_STEP_TIMEOUT,
};
use dusk_consensus::errors::{ConsensusError, HeaderError};
use dusk_consensus::user::provisioners::{ContextProvisioners, Provisioners};
use dusk_consensus::user::stake::Stake;
use execution_core::signatures::bls;
use node_data::bls::PublicKey;
use node_data::events::contract::ContractEvent;
use node_data::events::{
    BlockEvent, Event, TransactionEvent, BLOCK_CONFIRMED, BLOCK_FINALIZED,
};
use node_data::ledger::{
    self, to_str, Block, BlockWithLabel, Label, Seed, Slash,
};
use node_data::message::{AsyncQueue, Payload, Status};
use rkyv::{check_archived_root, Deserialize, Infallible};

use core::panic;
use dusk_consensus::operations::Voter;
use execution_core::stake::{SlashEvent, StakeAmount, StakeEvent};
use metrics::{counter, gauge, histogram};
use node_data::message::payload::{GetBlocks, Vote};
use node_data::{get_current_timestamp, Serializable, StepName};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{cmp, env};
use tokio::sync::mpsc::Sender;
use tokio::sync::{RwLock, RwLockReadGuard};
use tracing::{debug, error, info, trace, warn};

use super::consensus::Task;
use crate::chain::header_validation::{verify_att, verify_faults, Validator};
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
    pub(crate) tip: RwLock<BlockWithLabel>,

    /// Provisioners needed to verify next block
    pub(crate) provisioners_list: RwLock<ContextProvisioners>,

    /// Upper layer consensus task
    task: RwLock<super::consensus::Task>,

    pub(crate) db: Arc<RwLock<DB>>,
    pub(crate) vm: Arc<RwLock<VM>>,
    pub(crate) network: Arc<RwLock<N>>,

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

#[derive(Debug)]
enum ProvisionerChange {
    Stake(StakeEvent),
    Unstake(StakeEvent),
    Slash(SlashEvent),
    HardSlash(SlashEvent),
}

fn stake_event(data: &[u8]) -> StakeEvent {
    let staking_event_data = check_archived_root::<StakeEvent>(data)
        .expect("Stake event data should deserialize correctly");
    let staking_event_data: StakeEvent = staking_event_data
        .deserialize(&mut Infallible)
        .expect("Infallible");
    staking_event_data
}

fn slash_event(data: &[u8]) -> SlashEvent {
    let staking_event_data = check_archived_root::<SlashEvent>(data)
        .expect("Stake event data should deserialize correctly");
    let staking_event_data: SlashEvent = staking_event_data
        .deserialize(&mut Infallible)
        .expect("Infallible");
    staking_event_data
}

impl ProvisionerChange {
    pub fn from_event(event: &ContractEvent) -> Option<ProvisionerChange> {
        let event = match event.topic.as_str() {
            "stake" => ProvisionerChange::Stake(stake_event(&event.data)),
            "unstake" => ProvisionerChange::Unstake(stake_event(&event.data)),
            "slash" => ProvisionerChange::Slash(slash_event(&event.data)),
            "hard_slash" => {
                ProvisionerChange::HardSlash(slash_event(&event.data))
            }
            _ => return None,
        };
        Some(event)
    }
    fn key(&self) -> &bls::PublicKey {
        match &self {
            ProvisionerChange::Stake(e) => &e.keys.account,
            ProvisionerChange::Unstake(e) => &e.keys.account,
            ProvisionerChange::Slash(e) => &e.account,
            ProvisionerChange::HardSlash(e) => &e.account,
        }
    }

    fn to_public_key(&self) -> PublicKey {
        PublicKey::new(*self.key())
    }

    fn value(&self) -> u64 {
        match &self {
            ProvisionerChange::Stake(e) => e.value,
            ProvisionerChange::Unstake(e) => e.value,
            ProvisionerChange::Slash(e) => e.value,
            ProvisionerChange::HardSlash(e) => e.value,
        }
    }
}

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

        let tip_ts = acc.tip.read().await.inner().header().timestamp;
        Self::init_delay(tip_ts).await;

        Ok(acc)
    }

    pub async fn init_delay(tip_ts: u64) {
        let spin_time: u64 = env::var("RUSK_CONSENSUS_SPIN_TIME")
            .unwrap_or_default()
            .parse()
            .unwrap_or_default();

        let spin_time = cmp::max(spin_time, tip_ts);

        if spin_time == 0 || get_current_timestamp() > tip_ts {
            return;
        }

        info!("RUSK_CONSENSUS_SPIN_TIME is {spin_time}");

        let spin_time = UNIX_EPOCH + Duration::from_secs(spin_time);
        let mut now = SystemTime::now();
        while spin_time > now {
            let to_wait =
                spin_time.duration_since(now).expect("When the hell am I?");

            info!(
                "Waiting {to_wait:?} for consensus to be triggered at {}",
                time_util::print_system_time_to_rfc3339(&spin_time)
            );

            let chunk = match to_wait {
                // More than 1h print every 15min
                secs if secs > Duration::from_secs(60 * 60) => {
                    Duration::from_secs(15 * 60)
                }
                // More than 30min print every 10min
                secs if secs > Duration::from_secs(30 * 60) => {
                    Duration::from_secs(10 * 60)
                }
                // More than 5min print every 5min
                secs if secs > Duration::from_secs(5 * 60) => {
                    Duration::from_secs(5 * 60)
                }
                // More than 1min print every 30secs
                secs if secs > Duration::from_secs(60) => {
                    Duration::from_secs(30)
                }
                // Countdown last minute
                _ => Duration::from_secs(1),
            };

            tokio::time::sleep(chunk).await;
            now = SystemTime::now();
        }
        env::remove_var("RUSK_CONSENSUS_SPIN_TIME");
    }

    pub async fn spawn_task(&self) {
        const REDUNDANCY: usize = 16;
        const WAIT_TIMEOUT: Duration = Duration::from_secs(5);
        let provisioners_list = self.provisioners_list.read().await.clone();
        let base_timeouts = self.adjust_round_base_timeouts().await;
        let tip = self.tip.read().await.inner().clone();

        let locator = tip.header().hash;
        let msg = GetBlocks::new(locator).into();
        {
            let net = self.network.read().await;
            net.wait_for_alive_nodes(REDUNDANCY, WAIT_TIMEOUT).await;
            if let Err(e) = net.send_to_alive_peers(msg, REDUNDANCY).await {
                warn!("Unable to send GetBlocks message {e}");
            }
        }

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

    // Re-route a message to Consensus and Network
    pub(crate) async fn reroute_msg(
        &self,
        msg: Message,
    ) -> Result<(), async_channel::TrySendError<Message>> {
        // Filter out non-consensus messages
        if !msg.topic().is_consensus_msg() {
            warn!("invalid inbound message");
            return Ok(());
        }

        let consensus_task = self.task.read().await;
        // If we are syncing our chain, we blindly repropagate everything
        // beacuse we cannot verify any future message but do not want to affect
        // propagation
        if !consensus_task.is_running() {
            broadcast(&self.network, &msg).await;
            // We return here because if Consensus is not running we can't
            // process any Consensus message
            return Ok(());
        }

        let tip_header = self.tip.read().await.inner().header().clone();
        let tip_height = tip_header.height;

        match &msg.payload {
            Payload::Candidate(_)
            | Payload::Validation(_)
            | Payload::Ratification(_)
            | Payload::ValidationQuorum(_) => {
                // Process consensus msg only if they are for the current round
                // or at most 10 rounds in the future
                let msg_round = msg.header.round;
                if msg_round > tip_height
                    && msg_round <= (tip_height + MAX_ROUND_DISTANCE)
                {
                    consensus_task.main_inbound.try_send(msg);
                } else {
                    warn!(
                      event = "msg discarded",
                      topic = ?msg.topic(),
                      info = ?msg.header,
                      ray_id = msg.ray_id()
                    );
                }
            }

            Payload::Quorum(qmsg) => {
                match msg.header.compare_round(tip_height + 1) {
                    // If Quorum is for the current round, we verify it,
                    // rebroadcast it, and reroute it to Consensus
                    Status::Present => {
                        // Verify Attestation
                        let cur_seed = tip_header.seed;
                        let cur_provisioners = self
                            .provisioners_list
                            .read()
                            .await
                            .current()
                            .clone();

                        let res = verify_att(
                            &qmsg.att,
                            qmsg.header,
                            cur_seed,
                            &cur_provisioners,
                            None,
                        )
                        .await;

                        match res {
                            Ok(_) => {
                                // Reroute to Consensus
                                //
                                // INFO: rebroadcast of current-round Quorums is
                                // delegated to Consensus. We do this to allow
                                // iteration-based logic
                                trace!(
                                    event = "Rerouting Quorum to Consensus",
                                    round = qmsg.header.round,
                                    iter = qmsg.header.iteration,
                                );
                                consensus_task.main_inbound.try_send(msg);
                            }
                            Err(err) => {
                                error!("Attestation verification failed: {err}")
                            }
                        }
                    }

                    // If Quorum is for a past round, we only rebroadcast it if
                    // it's Valid, since Fail Quorums have no influence on past
                    // rounds
                    Status::Past => {
                        match qmsg.vote() {
                            Vote::Valid(_) => {
                                if let Ok(local_blk) =
                                    self.db.read().await.view(|db| {
                                        db.block_by_height(qmsg.header.round)
                                    })
                                {
                                    if let Some(l_blk) = local_blk {
                                        let l_prev =
                                            l_blk.header().prev_block_hash;
                                        let l_iter = l_blk.header().iteration;
                                        let q_prev =
                                            qmsg.header.prev_block_hash;
                                        let q_iter = qmsg.header.iteration;

                                        // Rebroadcast past Quorums if they are
                                        // from a fork or they are for a
                                        // higher-priority candidate
                                        if l_prev != q_prev || l_iter > q_iter {
                                            debug!(
                                                "Rebroadcast past-round Quorum"
                                            );
                                            broadcast(&self.network, &msg)
                                                .await;
                                        } else {
                                            debug!(
                                              event = "Quorum discarded",
                                              reason = "past round, lower priority",
                                              round = qmsg.header.round,
                                              iter = qmsg.header.iteration,
                                              vote = ?qmsg.vote(),
                                            );
                                        }
                                    }
                                } else {
                                    warn!("Could not check candidate in DB. Skipping Quorum rebroadcast");
                                };
                            }

                            _ => {
                                debug!(
                                  event = "Quorum discarded",
                                  reason = "past round, not Valid",
                                  round = qmsg.header.round,
                                  iter = qmsg.header.iteration,
                                  vote = ?qmsg.vote(),
                                );
                            }
                        }
                    }

                    Status::Future => {
                        // We do not rebroadcast future Quorum messages because
                        // we cannot pre-verify them and we want to avoid
                        // potential DoS attacks
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn selective_update(
        block_height: u64,
        stake_events: &[ContractEvent],
        provisioners_list: &mut tokio::sync::RwLockWriteGuard<
            '_,
            ContextProvisioners,
        >,
    ) -> Result<()> {
        let src = "selective";
        let changed_prov: Vec<_> = stake_events
            .iter()
            .filter_map(ProvisionerChange::from_event)
            .collect();
        if changed_prov.is_empty() {
            provisioners_list.remove_previous();
        } else {
            let mut new_prov = provisioners_list.current().clone();
            for change in changed_prov {
                let account = change.to_public_key();
                let value = change.value();
                info!(
                    event = "provisioner_update",
                    src,
                    account = account.to_bs58(),
                    value
                );
                match &change {
                    ProvisionerChange::Stake(stake_event) => {
                        match new_prov.get_member_mut(&account) {
                            Some(stake) if stake.value() == 0 => anyhow::bail!(
                                "Found an active stake with 0 amount"
                            ),
                            Some(stake) => stake.add(stake_event.value),
                            None => {
                                let amount = StakeAmount::new(
                                    stake_event.value,
                                    block_height,
                                );
                                let stake = Stake::new(
                                    amount.value,
                                    amount.eligibility,
                                );
                                new_prov.add_member_with_stake(account, stake);
                            }
                        }
                    }
                    ProvisionerChange::Unstake(unstake_event) => {
                        let unstaked = unstake_event.value;
                        new_prov.sub_stake(&account, unstaked).ok_or(
                            anyhow::anyhow!("Unstake a not existing stake"),
                        )?;
                    }
                    ProvisionerChange::Slash(slash_event)
                    | ProvisionerChange::HardSlash(slash_event) => {
                        let to_slash = new_prov
                            .get_member_mut(&account)
                            .ok_or(anyhow::anyhow!(
                                "Slashing a not existing stake"
                            ))?;
                        to_slash.subtract(slash_event.value);
                        to_slash
                            .change_eligibility(slash_event.next_eligibility);
                    }
                }
            }
            // Update new prov
            provisioners_list.update_and_swap(new_prov);
        }
        Ok(())
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
            .view(|t| t.block_exists(&blk.header().hash))?;

        if !exists {
            return Err(anyhow::anyhow!("could not find block"));
        }

        // Reset Consensus
        task.abort_with_wait().await;

        //  Update register.
        self.db.read().await.update(|db| {
            db.op_write(MD_HASH_KEY, blk.header().hash)?;
            db.op_write(MD_STATE_ROOT_KEY, blk.header().state_hash)
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

            let (stakes, finality) = self.db.read().await.update(|db| {
                let (txs, verification_output, stake_events) =
                    vm.accept(blk, &prev_block_voters[..])?;
                for spent_tx in txs.iter() {
                    events.push(TransactionEvent::Executed(spent_tx).into());
                }
                est_elapsed_time = start.elapsed();

                assert_eq!(header.state_hash, verification_output.state_root);
                assert_eq!(header.event_bloom, verification_output.event_bloom);

                let finality =
                    self.rolling_finality::<DB>(pni, blk, db, &mut events)?;

                let label = finality.0;
                // Store block with updated transactions with Error and GasSpent
                block_size_on_disk =
                    db.store_block(header, &txs, blk.faults(), label)?;

                Ok((stake_events, finality))
            })?;

            self.log_missing_iterations(
                provisioners_list.current(),
                header.iteration,
                tip.inner().header().seed,
                header.height,
            );

            for slashed in Slash::from_block(blk)? {
                info!(
                    "Slashed {} at block {} (type: {:?})",
                    slashed.provisioner.to_base58(),
                    blk.header().height,
                    slashed.r#type
                );
                slashed_count += 1;
            }

            let selective_update = Self::selective_update(
                header.height,
                &stakes,
                &mut provisioners_list,
            );

            if let Err(e) = selective_update {
                warn!("Resync provisioners due to {e:?}");
                let state_hash = blk.header().state_hash;
                let new_prov = vm.get_provisioners(state_hash)?;
                provisioners_list.update_and_swap(new_prov)
            }

            let (label, final_results) = finality;
            // Update tip
            *tip = BlockWithLabel::new_with_label(blk.clone(), label);

            let finalized = final_results.is_some();

            if let Some((prev_final_state, mut new_finals)) = final_results {
                let (_, new_final_state) =
                    new_finals.pop_last().expect("new_finals to be not empty");
                let old_finals_to_merge = new_finals
                    .into_values()
                    .chain([prev_final_state])
                    .collect::<Vec<_>>();
                vm.finalize_state(new_final_state, old_finals_to_merge)?;
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
            .update(|db| {
                // Delete any candidate block older than TIP - OFFSET
                let threshold = tip
                    .inner()
                    .header()
                    .height
                    .saturating_sub(CANDIDATES_DELETION_OFFSET);

                db.delete_candidate(|height| height <= threshold)?;

                // Delete from mempool any transaction already included in the
                // block
                for tx in tip.inner().txs().iter() {
                    let tx_id = tx.id();
                    for deleted in db
                        .delete_mempool_tx(tx_id, false)
                        .map_err(|e| warn!("Error while deleting tx: {e}"))
                        .unwrap_or_default()
                    {
                        events.push(TransactionEvent::Removed(deleted).into());
                    }

                    let spend_ids = tx.to_spend_ids();
                    for orphan_tx in db.mempool_txs_by_spendable_ids(&spend_ids)
                    {
                        for deleted_tx in db
                            .delete_mempool_tx(orphan_tx, false)
                            .map_err(|e| {
                                warn!("Error while deleting orphan_tx: {e}")
                            })
                            .unwrap_or_default()
                        {
                            events.push(
                                TransactionEvent::Removed(deleted_tx).into(),
                            );
                        }
                    }
                }
                Ok(db.count_candidates())
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
        pni: u8, // Previous Non-Attested Iterations
        blk: &Block,
        db: &mut D::P<'_>,
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
            let (hash, label) = db.block_label_by_height(height)?.ok_or(
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
            .block_header(&lfb_hash)?
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
                        .block_header(&hash)?
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
        let (blk, label) = self.db.read().await.update(|db| {
            let mut height = curr_height;
            loop {
                let b = db
                    .block_by_height(height)?
                    .ok_or_else(|| anyhow::anyhow!("could not fetch block"))?;
                let h = b.header();
                let (_, label) =
                    db.block_label_by_height(h.height)?.ok_or_else(|| {
                        anyhow::anyhow!("could not fetch block label")
                    })?;

                if h.state_hash == target_state_hash {
                    return Ok((b, label));
                }

                // the target_state_hash could not be found
                if height == 0 {
                    panic!("revert to genesis block failed");
                }

                if let Err(e) = self.event_sender.try_send(
                    BlockEvent::Deleted {
                        hash: h.hash,
                        height: h.height,
                    }
                    .into(),
                ) {
                    warn!("cannot notify event {e}")
                };

                info!(
                    event = "block deleted",
                    height = h.height,
                    iter = h.iteration,
                    label = ?label,
                    hash = hex::encode(h.hash)
                );

                // Delete any rocksdb record related to this block
                db.delete_block(&b)?;

                let now = get_current_timestamp();

                // Attempt to resubmit transactions back to mempool.
                // An error here is not considered critical.
                // Txs timestamp is reset here
                for tx in b.txs().iter() {
                    if let Err(e) = db.store_mempool_tx(tx, now) {
                        warn!("failed to resubmit transactions: {e}")
                    };
                }

                height -= 1;
            }
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

    pub(crate) async fn get_last_final_block(&self) -> Result<Block> {
        let tip: RwLockReadGuard<'_, BlockWithLabel> = self.tip.read().await;
        if tip.is_final() {
            return Ok(tip.inner().clone());
        }

        // Retrieve the last final block from the database
        let final_block = self.db.read().await.view(|v| {
            let prev_height = tip.inner().header().height - 1;

            for height in (0..prev_height).rev() {
                if let Ok(Some((hash, Label::Final(_)))) =
                    v.block_label_by_height(height)
                {
                    if let Some(blk) = v.block(&hash)? {
                        return Ok(blk);
                    } else {
                        return Err(anyhow::anyhow!(
                            "could not fetch the last final block by height"
                        ));
                    }
                }
            }

            warn!("No final block found, using genesis block");
            v.block_by_height(0)?
                .ok_or(anyhow::anyhow!("could not find the genesis block"))
        })?;

        Ok(final_block)
    }

    pub(crate) async fn get_curr_tip(&self) -> BlockWithLabel {
        self.tip.read().await.clone()
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
        let metric = self.db.read().await.view(|db| {
            let bytes = &db.op_read(key)?;
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
                    .block_header(&header.prev_block_hash)?
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

    /// Verifies if a block with header `local` can be replaced with a block
    /// with header `new`
    pub(crate) async fn verify_header_against_local(
        &self,
        local: &ledger::Header,
        new: &ledger::Header,
    ) -> Result<()> {
        let prev_header = self.db.read().await.view(|db| {
            let prev_hash = &local.prev_block_hash;
            db.block_header(prev_hash)?.ok_or(anyhow::anyhow!(
                "Unable to find block with hash {}",
                to_str(prev_hash)
            ))
        })?;

        let provisioners_list = self
            .vm
            .read()
            .await
            .get_provisioners(prev_header.state_hash)?;

        let mut provisioners_list = ContextProvisioners::new(provisioners_list);

        let changed_provisioners = self
            .vm
            .read()
            .await
            .get_changed_provisioners(prev_header.state_hash)?;
        provisioners_list.apply_changes(changed_provisioners);

        // Ensure header of the new block is valid according to prev_block
        // header
        let _ = verify_block_header(
            self.db.clone(),
            &prev_header,
            &provisioners_list,
            new,
        )
        .await?;

        Ok(())
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
) -> Result<(u8, Vec<Voter>, Vec<Voter>), HeaderError> {
    let validator = Validator::new(db, prev_header, provisioners);
    let expected_generator = provisioners.current().get_generator(
        header.iteration,
        prev_header.seed,
        header.height,
    );
    validator
        .execute_checks(header, &expected_generator, false)
        .await
}
