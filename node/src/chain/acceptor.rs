// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::database::{self, Ledger, Mempool};
use crate::{vm, Message, Network};
use anyhow::{anyhow, Result};
use dusk_consensus::commons::{ConsensusError, IterCounter, StepName};
use dusk_consensus::user::committee::{Committee, CommitteeSet};
use dusk_consensus::user::provisioners::Provisioners;
use dusk_consensus::user::sortition;
use node_data::ledger::{
    self, to_str, Block, BlockWithLabel, Label, Seed, Signature,
    SpentTransaction,
};
use node_data::message::AsyncQueue;
use node_data::message::Payload;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, warn};

use dusk_consensus::quorum::verifiers;

use dusk_consensus::config::{self, PROPOSAL_COMMITTEE_SIZE};
use dusk_consensus::quorum::verifiers::QuorumResult;

use super::consensus::Task;

#[allow(dead_code)]
pub(crate) enum RevertTarget {
    LastFinalizedState = 0,
    LastEpoch = 1,
}

/// Implements block acceptance procedure. This includes block header,
/// certificate and transactions full verifications.
/// Acceptor also manages the initialization and lifespan of Consensus task.
pub(crate) struct Acceptor<N: Network, DB: database::DB, VM: vm::VMExecution> {
    /// Most recently accepted block a.k.a blockchain tip
    mrb: RwLock<BlockWithLabel>,

    /// List of provisioners of actual round
    pub(crate) provisioners_list: RwLock<Provisioners>,

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

impl<DB: database::DB, VM: vm::VMExecution, N: Network> Acceptor<N, DB, VM> {
    pub async fn new_with_run(
        keys_path: &str,
        mrb: &BlockWithLabel,
        provisioners_list: &Provisioners,
        db: Arc<RwLock<DB>>,
        network: Arc<RwLock<N>>,
        vm: Arc<RwLock<VM>>,
    ) -> Self {
        let acc = Self {
            mrb: RwLock::new(mrb.clone()),
            provisioners_list: RwLock::new(provisioners_list.clone()),
            db: db.clone(),
            vm: vm.clone(),
            network: network.clone(),
            task: RwLock::new(Task::new_with_keys(keys_path.to_owned())),
        };

        acc.task.write().await.spawn(
            mrb.inner(),
            provisioners_list,
            &db,
            &vm,
            &network,
        );

        acc
    }
    // Re-route message to consensus task
    pub(crate) async fn reroute_msg(
        &self,
        msg: Message,
    ) -> Result<(), async_channel::SendError<Message>> {
        match &msg.payload {
            Payload::Candidate(_) | Payload::Validation(_) => {
                self.task.read().await.main_inbound.send(msg).await?;
            }
            Payload::Quorum(_) => {
                self.task.read().await.quorum_inbound.send(msg).await?;
            }
            _ => warn!("invalid inbound message"),
        }
        Ok(())
    }

    pub fn needs_update(blk: &Block, txs: &[SpentTransaction]) -> bool {
        //TODO Remove hardcoded epoch
        if blk.header().height % 2160 == 0 {
            return true;
        }
        txs.iter().filter(|t| t.err.is_none()).any(|t| {
            match &t.inner.inner.call {
                //TODO Check for contractId too
                Some((_, method, _)) if method == "stake" => true,
                Some((_, method, _)) if method == "unstake" => true,
                _ => false,
            }
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
        self.db
            .read()
            .await
            .update(|t| t.set_register(blk.header()))?;

        *provisioners_list = self.vm.read().await.get_provisioners()?;
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
            let committee_keys = Committee::new(
                node_data::bls::PublicKey::default(),
                provisioners_list,
                &sortition::Config {
                    committee_size: PROPOSAL_COMMITTEE_SIZE,
                    round,
                    seed,
                    step: iter * 3,
                },
            );

            if committee_keys.size() != 1 {
                let len = committee_keys.size();
                error!(
                    "Unable to generate voting committee for missed block: {len}",
                )
            } else {
                let generator = committee_keys
                    .iter()
                    .next()
                    .expect("committee to have 1 entry")
                    .to_bs58();
                warn!(
                    event = "missed iteration",
                    height = round,
                    iter,
                    generator,
                );
            }
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
        let nil_quorum = verify_block_header(
            self.db.clone(),
            &mrb.inner().header().clone(),
            &provisioners_list,
            blk.header(),
        )
        .await?;

        // Define new block label
        let label = {
            let mut label = Label::Accepted;
            if nil_quorum == Some(true) || blk.header().iteration == 0 {
                label = Label::Attested;
                if mrb.is_final() {
                    label = Label::Final;
                }
            }

            label
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
                &provisioners_list,
                header.iteration,
                mrb.inner().header().seed,
                header.height,
            );

            if Self::needs_update(blk.inner(), &txs) {
                // Update provisioners list
                *provisioners_list = vm.get_provisioners()?;
            }

            // Update most_recent_block
            *mrb = blk;

            anyhow::Ok(())
        }?;

        // Delete from mempool any transaction already included in the block
        self.db.read().await.update(|update| {
            for tx in mrb.inner().txs().iter() {
                database::Mempool::delete_tx(update, tx.hash())?;
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
        );

        // Restart Consensus.
        if enable_consensus {
            task.spawn(
                mrb.inner(),
                &provisioners_list,
                &self.db,
                &self.vm,
                &self.network,
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
                info!(event = "vm_revert to last finalized state");
                let state_hash = self.vm.read().await.revert()?;

                info!(
                    event = "vm reverted",
                    state_root = hex::encode(state_hash)
                );

                anyhow::Ok(state_hash)
            }
            _ => unimplemented!(),
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

    pub(crate) async fn get_curr_height(&self) -> u64 {
        self.mrb.read().await.inner().header().height
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
}

/// Performs full verification of block header (blk_header) against
/// local/current state.
pub(crate) async fn verify_block_header<DB: database::DB>(
    db: Arc<RwLock<DB>>,
    mrb: &ledger::Header,
    mrb_eligible_provisioners: &Provisioners,
    new_blk: &ledger::Header,
) -> anyhow::Result<Option<bool>> {
    if new_blk.version > 0 {
        return Err(anyhow!("unsupported block version"));
    }

    if new_blk.hash == [0u8; 32] {
        return Err(anyhow!("empty block hash"));
    }

    if new_blk.height != mrb.height + 1 {
        return Err(anyhow!(
            "invalid block height block_height: {:?}, curr_height: {:?}",
            new_blk.height,
            mrb.height,
        ));
    }

    if new_blk.prev_block_hash != mrb.hash {
        return Err(anyhow!("invalid previous block hash"));
    }

    if new_blk.timestamp < mrb.timestamp {
        //TODO:
        return Err(anyhow!("invalid block timestamp"));
    }

    // Ensure block is not already in the ledger
    db.read().await.view(|v| {
        if Ledger::get_block_exists(&v, &new_blk.hash)? {
            return Err(anyhow!("block already exists"));
        }

        Ok(())
    })?;

    // Verify prev_block_cert field
    if mrb.height >= 1 {
        let prev_block_seed = db.read().await.view(|v| {
            let prev_block = Ledger::fetch_block_by_height(&v, mrb.height - 1)?
                .ok_or_else(|| anyhow::anyhow!("could not fetch block"))?;

            Ok::<_, anyhow::Error>(prev_block.header().seed)
        })?;

        // Terms in use
        // genesis_blk -> ... -> prev_block -> most_recent_block(mrb) -> new_blk
        // (pending to be accepted)
        let prev_eligible_provisioners = &mrb_eligible_provisioners; // TODO: This should be the set of  actual eligible provisioners of
                                                                     // previous block. See also #1124
        verify_block_cert(
            prev_block_seed,
            prev_eligible_provisioners,
            mrb.hash,
            mrb.height,
            &new_blk.prev_block_cert,
            mrb.iteration,
            true,
        )
        .await?;
    }

    // Verify Failed iterations
    let mut failed_iterations_have_nil_quorum: Option<bool> = None;
    for iteration in 0..new_blk.failed_iterations.cert_list.len() {
        if let Some(cert) = &new_blk.failed_iterations.cert_list[iteration] {
            info!(
                event = "verify_cert",
                cert_type = "failed_cert",
                iter = iteration
            );

            let quorums = verify_block_cert(
                mrb.seed,
                mrb_eligible_provisioners,
                [0u8; 32],
                new_blk.height,
                cert,
                iteration as u8,
                false,
            )
            .await?;

            match failed_iterations_have_nil_quorum {
                None => {
                    failed_iterations_have_nil_quorum = Some(
                        quorums.0.quorum_reached()
                            && quorums.1.quorum_reached(),
                    )
                }
                Some(true) => {
                    failed_iterations_have_nil_quorum = Some(
                        quorums.0.quorum_reached()
                            && quorums.1.quorum_reached(),
                    )
                }
                Some(false) => {}
            }
        }
    }

    // Verify Certificate
    verify_block_cert(
        mrb.seed,
        mrb_eligible_provisioners,
        new_blk.hash,
        new_blk.height,
        &new_blk.cert,
        new_blk.iteration,
        true,
    )
    .await?;

    Ok(failed_iterations_have_nil_quorum)
}

pub async fn verify_block_cert(
    curr_seed: Signature,
    curr_eligible_provisioners: &Provisioners,
    block_hash: [u8; 32],
    height: u64,
    cert: &ledger::Certificate,
    iteration: u8,
    enable_quorum_check: bool,
) -> anyhow::Result<(QuorumResult, QuorumResult)> {
    let committee = Arc::new(Mutex::new(CommitteeSet::new(
        node_data::bls::PublicKey::default(),
        curr_eligible_provisioners.clone(),
    )));

    let hdr = node_data::message::Header {
        topic: 0,
        pubkey_bls: node_data::bls::PublicKey::default(),
        round: height,
        step: iteration.step_from_name(StepName::Ratification),
        block_hash,
    };

    let mut result = (QuorumResult::default(), QuorumResult::default());

    // Verify validation
    match verifiers::verify_step_votes(
        &cert.validation,
        &committee,
        curr_seed,
        &hdr,
        0,
        config::VALIDATION_COMMITTEE_SIZE,
        enable_quorum_check,
    )
    .await
    {
        Ok(validation_quorum_result) => {
            result.0 = validation_quorum_result;
        }
        Err(e) => {
            return Err(anyhow!(
            "invalid validation, hash = {}, round = {}, iter = {}, seed = {},  sv = {:?}, err = {}",
            to_str(&hdr.block_hash),
            hdr.round,
            iteration,
            to_str(&curr_seed.inner()),
            cert.validation,
            e
        ));
        }
    };

    // Verify ratification
    match verifiers::verify_step_votes(
        &cert.ratification,
        &committee,
        curr_seed,
        &hdr,
        1,
        config::RATIFICATION_COMMITTEE_SIZE,
        enable_quorum_check,
    )
    .await
    {
        Ok(ratification_quorum_result) => {
            result.1 = ratification_quorum_result;
        }
        Err(e) => {
            return Err(anyhow!(
            "invalid ratification, hash = {}, round = {}, iter = {}, seed = {},  sv = {:?}, err = {}",
            to_str(&hdr.block_hash),
            hdr.round,
            iteration,
            to_str(&curr_seed.inner()),
            cert.ratification,
            e,
        ));
        }
    }

    Ok(result)
}
