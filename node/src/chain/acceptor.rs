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
use node_data::ledger::{self, Block, Hash, Header};
use node_data::message::AsyncQueue;
use node_data::message::{Payload, Topics};
use node_data::Serializable;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::warn;

use std::any;

use super::consensus::Task;
use super::genesis;

/// Implements block acceptance procedure. This includes block header,
/// certificate and transactions full verifications.
/// Acceptor also manages the initialization and lifespan of Consensus task.
pub(crate) struct Acceptor<N: Network, DB: database::DB, VM: vm::VMExecution> {
    /// Most recently accepted block a.k.a blockchain tip
    mrb: RwLock<Block>,

    /// List of provisioners of actual round
    provisioners_list: RwLock<Provisioners>,

    /// Upper layer consensus task
    task: RwLock<super::consensus::Task>,

    db: Arc<RwLock<DB>>,
    vm: Arc<RwLock<VM>>,
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

    pub fn needs_update(blk: &Block) -> bool {
        //TODO Remove hardcoded epoch
        if blk.header().height % 2160 == 0 {
            return true;
        }
        blk.txs.iter().filter(|t| t.err.is_none()).any(|t| {
            match &t.inner.call {
                //TODO Check for contractId too
                Some((_, method, _)) if method == "stake" => true,
                Some((_, method, _)) if method == "unstake" => true,
                _ => false,
            }
        })
    }

    pub(crate) async fn try_accept_block(
        &self,
        blk: &Block,
        enable_consensus: bool,
    ) -> anyhow::Result<()> {
        let mut task = self.task.write().await;
        let (_, public_key) = task.keys.clone();

        // Verify Block Header
        self.verify_block_header(&public_key, &blk.header).await?;

        let mut mrb = self.mrb.write().await;
        let mut provisioners_list = self.provisioners_list.write().await;

        // Reset Consensus
        task.abort_with_wait().await;

        // Persist block in consistency with the VM state update
        let updated_provisioners = {
            let vm = self.vm.write().await;
            self.db.read().await.update(|t| {
                t.store_block(blk, true)?;

                // Accept block transactions into the VM
                if blk.header.iteration == 1 {
                    vm.finalize(blk)?;
                    return Ok(());
                }

                //TODO: Retrieve eligible_provisioners
                vm.accept(blk)?;
                Ok(())
            });
            Self::needs_update(blk).then(|| vm.get_provisioners())
        };

        if let Some(updated_prov) = updated_provisioners {
            *provisioners_list = updated_prov?;
        }

        *mrb = blk.clone();

        // Delete from mempool any transaction already included in the block
        self.db.read().await.update(|update| {
            for tx in blk.txs.iter() {
                database::Mempool::delete_tx(update, tx.hash());
            }
            Ok(())
        })?;

        tracing::info!(
            "block accepted height:{} hash:{} txs_count: {}",
            blk.header.height,
            hex::encode(blk.header.hash),
            blk.txs.len(),
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

    pub(crate) async fn verify_block_header(
        &self,
        public_key: &node_data::bls::PublicKey,
        blk_header: &ledger::Header,
    ) -> anyhow::Result<()> {
        if blk_header.version > 0 {
            return Err(anyhow!("unsupported block version"));
        }

        let curr_header = self.mrb.read().await.header.clone();

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

        if blk_header.timestamp <= curr_header.timestamp {
            return Err(anyhow!("invalid block timestamp"));
        }

        // Ensure block is not already in the ledger
        self.db.read().await.view(|view| {
            if Ledger::get_block_exists(&view, &blk_header.hash)? {
                return Err(anyhow!("block already exists"));
            }

            Ok(())
        })?;

        // Verify Certificate
        self.verify_block_cert(
            public_key,
            blk_header.hash,
            blk_header.height,
            &blk_header.cert,
            blk_header.iteration,
        )
        .await
    }

    async fn verify_block_cert(
        &self,
        public_key: &node_data::bls::PublicKey,
        block_hash: [u8; 32],
        height: u64,
        cert: &ledger::Certificate,
        iteration: u8,
    ) -> anyhow::Result<()> {
        let mut mrb = self.mrb.write().await;
        let mut eligible_provisioners = self.provisioners_list.write().await;

        let committee = Arc::new(Mutex::new(CommitteeSet::new(
            public_key.clone(),
            eligible_provisioners.clone(),
        )));

        let hdr = node_data::message::Header {
            topic: 0,
            pubkey_bls: public_key.clone(),
            round: height,
            step: iteration * 3,
            block_hash,
        };

        let seed = mrb.header.seed;

        // Verify first reduction
        if let Err(e) = dusk_consensus::agreement::verifiers::verify_step_votes(
            &cert.first_reduction,
            &committee,
            seed,
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
            seed,
            &hdr,
            1,
        )
        .await
        {
            return Err(anyhow!("invalid second reduction votes"));
        }

        Ok(())
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

    pub(crate) async fn get_result_chan(
        &self,
    ) -> AsyncQueue<Result<Block, ConsensusError>> {
        self.task.read().await.result.clone()
    }

    pub(crate) async fn get_outbound_chan(&self) -> AsyncQueue<Message> {
        self.task.read().await.outbound.clone()
    }
}
