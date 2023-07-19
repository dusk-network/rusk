// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::database::{Candidate, Ledger, Mempool};
use crate::{database, vm, Network};
use crate::{LongLivedService, Message};
use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use dusk_bls12_381_sign::PublicKey;
use dusk_consensus::commons::{ConsensusError, Database, RoundUpdate};
use dusk_consensus::consensus::{self, Consensus};
use dusk_consensus::contract_state::{
    CallParams, Error, Operations, Output, StateRoot,
};
use dusk_consensus::user::committee::CommitteeSet;
use dusk_consensus::user::provisioners::Provisioners;
use hex::ToHex;
use node_data::ledger::{
    self, Block, Hash, Header, Signature, SpentTransaction,
};
use node_data::message::AsyncQueue;
use node_data::message::{Payload, Topics};
use node_data::Serializable;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{info, warn};

use std::any;

use super::consensus::Task;
use super::genesis;

pub(crate) enum RevertTarget {
    LastFinalizedState = 0,
    LastEpoch = 1,
}

/// Implements block acceptance procedure. This includes block header,
/// certificate and transactions full verifications.
/// Acceptor also manages the initialization and lifespan of Consensus task.
pub(crate) struct Acceptor<N: Network, DB: database::DB, VM: vm::VMExecution> {
    /// Most recently accepted block a.k.a blockchain tip
    mrb: RwLock<Block>,

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
        mrb: &Block,
        provisioners_list: &Provisioners,
        db: Arc<RwLock<DB>>,
        network: Arc<RwLock<N>>,
        vm: Arc<RwLock<VM>>,
    ) -> Self {
        let mut acc = Self {
            mrb: RwLock::new(mrb.clone()),
            provisioners_list: RwLock::new(provisioners_list.clone()),
            db: db.clone(),
            vm: vm.clone(),
            network: network.clone(),
            task: RwLock::new(Task::new_with_keys(keys_path.to_owned())),
        };

        acc.task.write().await.spawn(
            &mrb.header,
            &provisioners_list.clone(),
            &db,
            &vm,
            &network,
        );

        acc
    }
    // Re-route message to consensus task
    pub(crate) async fn reroute_msg(&self, msg: Message) {
        match &msg.payload {
            Payload::NewBlock(_) | Payload::Reduction(_) => {
                self.task.read().await.main_inbound.send(msg).await;
            }
            Payload::Agreement(_) | Payload::AggrAgreement(_) => {
                self.task.read().await.agreement_inbound.send(msg).await;
            }
            _ => warn!("invalid inbound message"),
        }
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
    ) -> anyhow::Result<()> {
        let mut task = self.task.write().await;
        let (_, public_key) = task.keys.clone();

        let mut mrb = self.mrb.write().await;
        let mut provisioners_list = self.provisioners_list.write().await;

        // Ensure block that will be marked as blockchain tip does exist
        let exists = self
            .db
            .read()
            .await
            .update(|t| t.get_block_exists(&blk.header.hash))?;

        if !exists {
            return Err(anyhow::anyhow!("could not find block"));
        }

        // Reset Consensus
        task.abort_with_wait().await;

        //  Update register.
        self.db
            .read()
            .await
            .update(|t| t.set_register(&blk.header))?;

        *provisioners_list = self.vm.read().await.get_provisioners()?;
        *mrb = blk.clone();

        Ok(())
    }

    pub(crate) async fn try_accept_block(
        &self,
        blk: &Block,
        enable_consensus: bool,
    ) -> anyhow::Result<()> {
        let mut task = self.task.write().await;
        let (_, public_key) = task.keys.clone();

        let mut mrb = self.mrb.write().await;
        let mut provisioners_list = self.provisioners_list.write().await;

        // Verify Block Header
        verify_block_header(
            self.db.clone(),
            &mrb.header.clone(),
            provisioners_list.clone(),
            &public_key,
            &blk.header,
        )
        .await?;

        // Reset Consensus
        task.abort_with_wait().await;

        // Persist block in consistency with the VM state update
        {
            let vm = self.vm.write().await;
            let txs = self.db.read().await.update(|t| {
                let (txs, state_hash) = match blk.header.iteration {
                    1 => vm.finalize(blk)?,
                    _ => vm.accept(blk)?,
                };

                assert_eq!(blk.header.state_hash, state_hash);

                // Store block with updated transactions with Error and GasSpent
                t.store_block(&blk.header, &txs)?;

                Ok(txs)
            })?;

            // Update provisioners list
            let updated_provisioners = {
                Self::needs_update(blk, &txs).then(|| vm.get_provisioners())
            };

            if let Some(updated_prov) = updated_provisioners {
                *provisioners_list = updated_prov?;
            };

            *mrb = blk.clone();

            anyhow::Ok(())
        }?;

        // Delete from mempool any transaction already included in the block
        self.db.read().await.update(|update| {
            for tx in blk.txs.iter() {
                database::Mempool::delete_tx(update, tx.hash());
            }
            Ok(())
        })?;

        tracing::info!(
            "block accepted height/iter:{}/{} hash:{} txs_count: {} state_hash:{}",
            blk.header.height,
            blk.header.iteration,
            hex::encode(blk.header.hash),
            blk.txs.len(),
            hex::encode(blk.header.state_hash)
        );

        // Restart Consensus.
        if enable_consensus {
            task.spawn(
                &mrb.header,
                &provisioners_list,
                &self.db,
                &self.vm,
                &self.network,
            );
        }

        Ok(())
    }

    /// Implements the algorithm of full revert to any of supported targets.
    ///
    /// This incorporates both VM state revert and Ledger state revert.
    pub async fn try_revert(&self, target: RevertTarget) -> Result<()> {
        let curr_height = self.get_curr_height().await;
        let curr_iteration = self.get_curr_iteration().await;

        let target_state_hash = match target {
            RevertTarget::LastFinalizedState => {
                info!("Revert VM to last finalized state");
                let state_hash = self.vm.read().await.revert()?;

                info!(
                    "VM revert completed finalized_state_hash:{}",
                    hex::encode(state_hash)
                );

                anyhow::Ok(state_hash)
            }
            RevertTarget::LastEpoch => panic!("not implemented"),
        }?;

        // Delete any block until we reach the target_state_hash, the
        // VM was reverted to.

        // The blockchain tip (most recent block) after reverting
        let mut most_recent_block = Block::default();

        self.db.read().await.update(|t| {
            let mut height = curr_height;
            while height != 0 {
                let blk = Ledger::fetch_block_by_height(t, height)?
                    .ok_or_else(|| anyhow::anyhow!("could not fetch block"))?;

                if blk.header.state_hash == target_state_hash {
                    most_recent_block = blk;
                    break;
                }

                info!(
                    "Delete block height: {} iter: {} hash: {}",
                    blk.header.height,
                    blk.header.iteration,
                    hex::encode(blk.header.hash)
                );

                // Delete any rocksdb record related to this block
                Ledger::delete_block(t, &blk)?;

                // Attempt to resubmit transactions back to mempool.
                // An error here is not considered critical.
                for tx in blk.txs().iter() {
                    Mempool::add_tx(t, tx).map_err(|err| {
                        tracing::error!("failed to resubmit transactions")
                    });
                }

                height -= 1;
            }

            Ok(())
        })?;

        if most_recent_block.header.state_hash != target_state_hash {
            return Err(anyhow!("Failed to revert to proper state"));
        }

        // Update blockchain tip to be the one we reverted to.
        info!(
            "Blockchain tip height: {} iter: {} state_hash: {}",
            most_recent_block.header.height,
            most_recent_block.header.iteration,
            hex::encode(most_recent_block.header.state_hash)
        );

        self.update_most_recent_block(&most_recent_block).await
    }

    pub(crate) async fn get_curr_height(&self) -> u64 {
        self.mrb.read().await.header.height
    }

    pub(crate) async fn get_curr_hash(&self) -> [u8; 32] {
        self.mrb.read().await.header.hash
    }

    pub(crate) async fn get_curr_timestamp(&self) -> i64 {
        self.mrb.read().await.header.timestamp
    }

    pub(crate) async fn get_curr_iteration(&self) -> u8 {
        self.mrb.read().await.header.iteration
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
    curr_header: &ledger::Header,
    curr_eligible_provisioners: Provisioners,
    curr_public_key: &node_data::bls::PublicKey,
    blk_header: &ledger::Header,
) -> anyhow::Result<()> {
    if blk_header.version > 0 {
        return Err(anyhow!("unsupported block version"));
    }

    if blk_header.height != curr_header.height + 1 {
        return Err(anyhow!(
            "invalid block height block_height: {:?}, curr_height: {:?}",
            blk_header.height,
            curr_header.height,
        ));
    }

    if blk_header.prev_block_hash != curr_header.hash {
        return Err(anyhow!("invalid previous block hash"));
    }

    if blk_header.timestamp < curr_header.timestamp {
        //TODO:
        return Err(anyhow!("invalid block timestamp"));
    }

    // Ensure block is not already in the ledger
    db.read().await.view(|v| {
        if Ledger::get_block_exists(&v, &blk_header.hash)? {
            return Err(anyhow!("block already exists"));
        }

        Ok(())
    })?;

    // Verify Certificate
    verify_block_cert(
        curr_header.seed,
        curr_eligible_provisioners,
        curr_public_key,
        blk_header.hash,
        blk_header.height,
        &blk_header.cert,
        blk_header.iteration,
    )
    .await
}

async fn verify_block_cert(
    curr_seed: Signature,
    curr_eligible_provisioners: Provisioners,
    curr_public_key: &node_data::bls::PublicKey,
    block_hash: [u8; 32],
    height: u64,
    cert: &ledger::Certificate,
    iteration: u8,
) -> anyhow::Result<()> {
    let committee = Arc::new(Mutex::new(CommitteeSet::new(
        curr_public_key.clone(),
        curr_eligible_provisioners.clone(),
    )));

    let hdr = node_data::message::Header {
        topic: 0,
        pubkey_bls: curr_public_key.clone(),
        round: height,
        step: iteration * 3,
        block_hash,
    };

    // Verify first reduction
    if let Err(e) = dusk_consensus::agreement::verifiers::verify_step_votes(
        &cert.first_reduction,
        &committee,
        curr_seed,
        &hdr,
        0,
    )
    .await
    {
        return Err(anyhow!("invalid first reduction votes"));
    }

    // Verify second reduction
    if let Err(e) = dusk_consensus::agreement::verifiers::verify_step_votes(
        &cert.second_reduction,
        &committee,
        curr_seed,
        &hdr,
        1,
    )
    .await
    {
        return Err(anyhow!("invalid second reduction votes"));
    }

    Ok(())
}
